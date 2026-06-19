mod comparisons;
mod constants;
mod expr;
mod functions;
mod join_tree;
mod paths;
mod rendering;
mod result_columns;
mod scope;

use std::collections::HashMap;

use querydown_parser::ast::{
    AnnotationValue, ComputedColumn, ConstantDef, CustomComparisonDef, FunctionDef, Query,
    Transformation,
};
use querydown_parser::parse;
use serde::Serialize;

use crate::{
    schema::{primitive_schema::PrimitiveSchema, Column, Schema, Table, ValueType},
    sql::tree::{Cte, Select},
    Options,
};

use self::{
    constants::PIPELINE_CTE_ALIAS_PREFIX,
    expr::convert_condition_set,
    rendering::Render,
    result_columns::{convert_result_columns, convert_sort_exprs, ConvertedResultColumns},
    scope::Scope,
};

/// The output of compiling Querydown code: the generated SQL plus column-level annotation.
///
/// `column_annotations` is positionally aligned with the columns of the result set — one entry per
/// output column, in order, with `null` for any column that has no annotation.
#[derive(Debug, Serialize)]
pub struct CompileResult {
    pub sql: String,
    #[serde(rename = "columnAnnotations")]
    pub column_annotations: Vec<Option<AnnotationValue>>,
}

pub struct Compiler {
    options: Options,
    schema: Schema,
}

impl Compiler {
    pub fn new(schema_json: &str, options: Options) -> Result<Self, String> {
        let primitive_schema = serde_json::from_str::<PrimitiveSchema>(schema_json)
            .map_err(|_| "Schema input is not valid JSON.")?;
        let schema = Schema::try_from(primitive_schema)?;
        Ok(Self { options, schema })
    }

    /// Compiles Querydown source code into SQL plus column annotations.
    pub fn compile(&self, input: String) -> Result<CompileResult, String> {
        self.compile_query(parse(&input)?)
    }

    /// Compiles an already-parsed [`Query`] into SQL plus column annotations.
    ///
    /// This is the lower-level counterpart to [`Compiler::compile`], which parses source code
    /// before compiling it. It exists so that a consumer can parse the sections of a query
    /// independently — via the parser crate's section parsers — reassemble them with
    /// [`Query::from_parts`], and then compile the result, rather than handing over a single string.
    pub fn compile_query(&self, query: Query) -> Result<CompileResult, String> {

        let transformations = if query.transformations.is_empty() {
            vec![Transformation::default()]
        } else {
            query.transformations
        };
        let n_stages = transformations.len();

        // Each non-final stage of the pipeline is compiled into a CTE. The CTE is also represented
        // as a synthetic `Table` (with the same name as the CTE) whose columns are the stage's
        // output columns, so that the next stage can reference them as if they were real columns.
        let mut synthetic_tables: Vec<Table> = Vec::new();
        let mut pipeline_ctes: Vec<Cte> = Vec::new();
        let mut final_stage: Option<StageOutput> = None;

        for (i, transformation) in transformations.into_iter().enumerate() {
            let is_last = i == n_stages - 1;
            // Confine the borrow of the stage's base table (which, for later stages, points into
            // `synthetic_tables`) to this block so that we can push to `synthetic_tables` after.
            let stage = {
                let base_table: &Table = if i == 0 {
                    resolve_base_table(&self.options, &self.schema, &query.base_table)?
                } else {
                    &synthetic_tables[i - 1]
                };
                self.compile_stage(
                    base_table,
                    transformation,
                    &query.definitions.constants,
                    &query.definitions.functions,
                    &query.definitions.custom_comparisons,
                    &query.definitions.computed_columns,
                    is_last,
                )?
            };

            if is_last {
                final_stage = Some(stage);
            } else {
                let cte_alias = format!("{PIPELINE_CTE_ALIAS_PREFIX}{i}");
                synthetic_tables.push(build_pipeline_table(cte_alias.clone(), &stage.output_names));
                pipeline_ctes.push(Cte {
                    alias: cte_alias,
                    select: stage.select,
                    // Pipeline CTEs are used as a stage's base table (via `FROM`), not joined, so
                    // this field is never consulted.
                    join_column_name: String::new(),
                });
            }
        }

        // The unwrap is safe because `n_stages >= 1`, so the final stage is always produced.
        let mut final_stage = final_stage.unwrap();
        // The pipeline CTEs must come before any CTEs the final stage produces for its own joins,
        // since later pipeline stages reference earlier ones.
        pipeline_ctes.append(&mut final_stage.select.ctes);
        final_stage.select.ctes = pipeline_ctes;

        // Rendering only needs the dialect (for quoting); the base table chosen here is irrelevant.
        let mut render_scope = Scope::build(&self.options, &self.schema, &query.base_table)?;

        Ok(CompileResult {
            sql: format!("{};", final_stage.select.render(&mut render_scope)),
            column_annotations: final_stage.column_annotations,
        })
    }

    /// Compiles a single stage of a query pipeline into a [`Select`], using `base_table` as the
    /// stage's base table. For a non-final stage, every output column is given an explicit alias so
    /// that the resulting CTE has well-defined column names for the next stage to reference.
    #[allow(clippy::too_many_arguments)]
    fn compile_stage(
        &self,
        base_table: &Table,
        transformation: Transformation,
        constants: &[ConstantDef],
        functions: &[FunctionDef],
        custom_comparisons: &[CustomComparisonDef],
        computed_columns: &[ComputedColumn],
        is_last: bool,
    ) -> Result<StageOutput, String> {
        let mut scope = Scope::build_with_base_table(&self.options, &self.schema, base_table);
        // The query's definitions are in scope for every stage of the pipeline.
        scope.register_constants(constants.to_vec());
        scope.register_functions(functions.to_vec());
        scope.register_custom_comparisons(custom_comparisons.to_vec())?;
        scope.register_computed_columns(computed_columns.to_vec())?;

        let mut select = Select::from(base_table.name.clone());
        select.conditions = convert_condition_set(transformation.conditions, &mut scope)?;
        let standalone_sorting = convert_sort_exprs(transformation.sorting, &mut scope)?;
        let ConvertedResultColumns {
            mut columns,
            sorting: column_sorting,
            grouping,
            column_annotations,
            output_names,
        } = convert_result_columns(transformation.result_columns, &mut scope)?;

        let output_names = if columns.is_empty() {
            // With no explicit result columns, the stage emits every column of its base table.
            let mut names: Vec<(usize, String)> = base_table
                .columns
                .values()
                .map(|c| (c.id, c.name.clone()))
                .collect();
            names.sort_by_key(|(id, _)| *id);
            names.into_iter().map(|(_, name)| name).collect()
        } else if is_last {
            output_names
        } else {
            // Force an explicit, unique alias on every column of a non-final stage so that the CTE
            // has deterministic column names matching its synthetic table.
            let names = dedupe_names(output_names);
            for (column, name) in columns.iter_mut().zip(names.iter()) {
                column.alias = Some(name.clone());
            }
            names
        };

        select.columns = columns;
        select.grouping = grouping;
        // Standalone `\\` sorts take precedence over column `\s` sorts, hence they come first.
        select.sorting = standalone_sorting
            .into_iter()
            .chain(column_sorting)
            .collect();
        (select.joins, select.ctes) = scope.decompose_join_tree();

        Ok(StageOutput {
            select,
            output_names,
            column_annotations,
        })
    }
}

/// The compiled output of a single pipeline stage.
struct StageOutput {
    select: Select,
    /// The names of the stage's output columns, used to build the synthetic table for the next
    /// stage. Only meaningful for non-final stages.
    output_names: Vec<String>,
    column_annotations: Vec<Option<AnnotationValue>>,
}

/// Resolves the query's base table by name within the schema.
fn resolve_base_table<'a>(
    options: &Options,
    schema: &'a Schema,
    name: &str,
) -> Result<&'a Table, String> {
    options
        .resolve_identifier(&schema.table_lookup, name)
        .map(|id| schema.tables.get(id).unwrap())
        .ok_or_else(|| format!("Base table `{}` does not exist.", name))
}

/// Builds a synthetic [`Table`] representing the output of a pipeline stage. The table's name
/// matches its CTE alias, and its columns are the stage's output columns (with unknown types, since
/// types are not tracked through a pipeline). It has no links, so the next stage can reference its
/// columns directly but cannot traverse relationships from it.
fn build_pipeline_table(name: String, column_names: &[String]) -> Table {
    let mut columns = HashMap::new();
    let mut column_lookup = HashMap::new();
    for (index, column_name) in column_names.iter().enumerate() {
        let id = index + 1;
        columns.insert(
            id,
            Column {
                id,
                name: column_name.clone(),
                r#type: ValueType::Unknown,
            },
        );
        column_lookup.insert(column_name.clone(), id);
    }
    Table {
        // This id is never used for schema lookups (the table is not in the schema), but we keep it
        // distinct from real table ids for clarity.
        id: usize::MAX,
        name,
        columns,
        column_lookup,
        forward_links_to_one: HashMap::new(),
        reverse_links_to_one: HashMap::new(),
        reverse_links_to_many: HashMap::new(),
    }
}

/// Makes a list of column names unique by appending `_2`, `_3`, … to later duplicates. This keeps a
/// pipeline stage's synthetic-table column lookup unambiguous when two output columns would
/// otherwise share a name.
fn dedupe_names(names: Vec<String>) -> Vec<String> {
    let mut counts = HashMap::<String, usize>::new();
    names
        .into_iter()
        .map(|name| {
            let count = counts.entry(name.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                name
            } else {
                format!("{name}_{count}")
            }
        })
        .collect()
}
