mod comparisons;
mod constants;
mod expr;
mod functions;
mod join_tree;
mod paths;
mod rendering;
mod result_columns;
mod scope;
mod window;

use std::collections::HashMap;

use querydown_parser::ast::{
    AnnotationValue, ComputedColumn, ConstantDef, CustomComparisonDef, Expr, FunctionDef, Query,
    Transformation,
};
use querydown_parser::parse;
use serde::Serialize;

use crate::{
    schema::{primitive_schema::PrimitiveSchema, Column as SchemaColumn, Schema, Table, ValueType},
    sql::tree::{Column, Cte, Select, SqlExpr},
    Options,
};

use self::{
    constants::PIPELINE_CTE_ALIAS_PREFIX,
    expr::{convert_condition_set, convert_expr},
    rendering::Render,
    result_columns::{convert_result_columns, convert_sort_exprs, ConvertedResultColumns},
    scope::Scope,
    window::{extract_window_conditions, SplitConditions, WindowProjection},
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
        // Each user-defined table (`#name = #( ... )`) is compiled into a CTE plus a synthetic
        // `Table` (named after the definition) representing its output columns. The synthetic tables
        // accumulate so that a later user-defined table — and ultimately the main query — can use an
        // earlier one as its base table.
        let mut udt_ctes: Vec<Cte> = Vec::new();
        let mut udt_tables: Vec<Table> = Vec::new();
        for table_def in &query.definitions.tables {
            let stage = compile_pipeline(
                &self.options,
                &self.schema,
                table_def.query.clone(),
                &udt_tables,
                true,
            )?;
            udt_tables.push(build_pipeline_table(
                table_def.name.clone(),
                &stage.output_names,
            ));
            udt_ctes.push(Cte {
                alias: table_def.name.clone(),
                select: stage.select,
                // User-defined-table CTEs are used as a base table (via `FROM`), not joined, so this
                // field is never consulted.
                join_column_name: String::new(),
            });
        }

        let mut final_stage =
            compile_pipeline(&self.options, &self.schema, query, &udt_tables, false)?;

        // The user-defined-table CTEs must come before any the main query produced, since the main
        // query references them.
        let mut ctes = udt_ctes;
        ctes.append(&mut final_stage.select.ctes);
        final_stage.select.ctes = ctes;

        // Rendering only needs the dialect (for quoting), so the base table chosen here is
        // irrelevant; any real table works (and the query's own base table may be a user-defined
        // table that does not exist in the schema).
        let any_table = self
            .schema
            .tables
            .values()
            .next()
            .ok_or("The schema contains no tables.")?;
        let mut render_scope = Scope::build_with_base_table(&self.options, &self.schema, any_table);

        Ok(CompileResult {
            sql: format!("{};", final_stage.select.render(&mut render_scope)),
            column_annotations: final_stage.column_annotations,
        })
    }
}

/// Compiles a whole query (which may itself be a multi-stage `~~~` pipeline) into a single
/// [`StageOutput`]. `extra_tables` are synthetic base tables — the user-defined tables already in
/// scope — that the query's first stage may use as its base table in addition to the schema's tables.
///
/// When `final_is_cte` is true, the pipeline's final stage is given explicit column aliases (exactly
/// like a non-final stage), so that the whole result can itself be materialized as a CTE — this is
/// how a user-defined table is compiled.
fn compile_pipeline(
    options: &Options,
    schema: &Schema,
    query: Query,
    extra_tables: &[Table],
    final_is_cte: bool,
) -> Result<StageOutput, String> {
    let transformations = if query.transformations.is_empty() {
        vec![Transformation::default()]
    } else {
        query.transformations
    };
    let n_stages = transformations.len();

    // Each non-final stage of the pipeline is compiled into a CTE. The CTE is also represented as a
    // synthetic `Table` (with the same name as the CTE) whose columns are the stage's output
    // columns, so that the next stage can reference them as if they were real columns.
    let mut synthetic_tables: Vec<Table> = Vec::new();
    let mut pipeline_ctes: Vec<Cte> = Vec::new();
    let mut final_stage: Option<StageOutput> = None;

    for (i, transformation) in transformations.into_iter().enumerate() {
        let is_final = i == n_stages - 1;
        // When the whole pipeline's output will itself become a CTE, even the final stage is given
        // explicit column aliases, just like a non-final stage.
        let is_last = is_final && !final_is_cte;
        // Confine the borrow of the stage's base table (which, for later stages, points into
        // `synthetic_tables`) to this block so that we can push to `synthetic_tables` after.
        let stage = {
            let base_table: &Table = if i == 0 {
                resolve_base_table(options, schema, &query.base_table, extra_tables)?
            } else {
                &synthetic_tables[i - 1]
            };
            compile_stage(
                options,
                schema,
                base_table,
                transformation,
                &query.definitions.constants,
                &query.definitions.functions,
                &query.definitions.custom_comparisons,
                &query.definitions.computed_columns,
                is_last,
            )?
        };

        if is_final {
            final_stage = Some(stage);
        } else {
            let cte_alias = format!("{PIPELINE_CTE_ALIAS_PREFIX}{i}");
            synthetic_tables.push(build_pipeline_table(cte_alias.clone(), &stage.output_names));
            pipeline_ctes.push(Cte {
                alias: cte_alias,
                select: stage.select,
                // Pipeline CTEs are used as a stage's base table (via `FROM`), not joined, so this
                // field is never consulted.
                join_column_name: String::new(),
            });
        }
    }

    // The unwrap is safe because `n_stages >= 1`, so the final stage is always produced.
    let mut final_stage = final_stage.unwrap();
    // The pipeline CTEs must come before any CTEs the final stage produces for its own joins, since
    // later pipeline stages reference earlier ones.
    pipeline_ctes.append(&mut final_stage.select.ctes);
    final_stage.select.ctes = pipeline_ctes;
    Ok(final_stage)
}

/// Compiles a scalar subquery (`#( ... )`) into a parenthesized `(SELECT ...)` SQL expression, to be
/// inlined wherever the subquery's value is referenced. The inner query is expected to produce a
/// single value.
pub(super) fn convert_subquery(query: Query, scope: &mut Scope) -> Result<SqlExpr, String> {
    let stage = compile_pipeline(scope.options, scope.schema, query, &[], false)?;
    // Rendering only reads the dialect from the scope, so rendering the subquery with the enclosing
    // scope is safe and does not disturb the outer query's compilation state.
    let sql = stage.select.render(scope);
    Ok(SqlExpr::atom(format!("({sql})")))
}

/// Compiles a single stage of a query pipeline into a [`Select`], using `base_table` as the
/// stage's base table. For a non-final stage, every output column is given an explicit alias so
/// that the resulting CTE has well-defined column names for the next stage to reference.
#[allow(clippy::too_many_arguments)]
fn compile_stage(
    options: &Options,
    schema: &Schema,
    base_table: &Table,
    transformation: Transformation,
    constants: &[ConstantDef],
    functions: &[FunctionDef],
    custom_comparisons: &[CustomComparisonDef],
    computed_columns: &[ComputedColumn],
    is_last: bool,
) -> Result<StageOutput, String> {
    let mut scope = Scope::build_with_base_table(options, schema, base_table);
    // The query's definitions are in scope for every stage of the pipeline.
    scope.register_constants(constants.to_vec());
    scope.register_functions(functions.to_vec());
    scope.register_custom_comparisons(custom_comparisons.to_vec())?;
    scope.register_computed_columns(computed_columns.to_vec())?;

    // SQL disallows window functions in a `WHERE` clause. When a stage's conditions reference a
    // window function, the window-bearing conditions are split off to be applied in an outer
    // query that wraps this one (see `wrap_stage_for_windows`).
    let SplitConditions {
        inner: inner_conditions,
        outer: outer_conditions,
        projections,
    } = extract_window_conditions(transformation.conditions);
    let is_wrapped = !projections.is_empty();

    let mut select = Select::from(base_table.name.clone());
    select.conditions = convert_condition_set(inner_conditions, &mut scope)?;
    let standalone_sorting = convert_sort_exprs(transformation.sorting, &mut scope)?;
    let ConvertedResultColumns {
        mut columns,
        sorting: column_sorting,
        grouping,
        column_annotations,
        output_names,
    } = convert_result_columns(transformation.result_columns, &mut scope)?;

    select.grouping = grouping;
    // Standalone `\\` sorts take precedence over column `\s` sorts, hence they come first.
    select.sorting = standalone_sorting
        .into_iter()
        .chain(column_sorting)
        .collect();

    if is_wrapped {
        // Give every real result column an explicit, unique alias so the wrapping query can
        // reference it by name. With no explicit result columns, expand the base table's columns.
        let real_output_names = if columns.is_empty() {
            let (expanded, names) = expand_base_table_columns(base_table, &scope);
            columns = expanded;
            names
        } else {
            let names = dedupe_names(output_names);
            for (column, name) in columns.iter_mut().zip(names.iter()) {
                column.alias = Some(name.clone());
            }
            names
        };
        select.columns = columns;
        return wrap_stage_for_windows(
            options,
            schema,
            scope,
            select,
            real_output_names,
            column_annotations,
            projections,
            outer_conditions,
            constants,
            functions,
        );
    }

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
    (select.joins, select.ctes) = scope.decompose_join_tree();

    Ok(StageOutput {
        select,
        output_names,
        column_annotations,
    })
}

/// Wraps a stage whose conditions reference window functions. The supplied `inner` select is
/// finished (its window values are projected as hidden columns, its real columns aliased to
/// `output_names`) and becomes a CTE. An outer select reads from that CTE, selecting the real
/// columns through and applying the window predicate that SQL would not permit in a `WHERE`.
#[allow(clippy::too_many_arguments)]
fn wrap_stage_for_windows(
    options: &Options,
    schema: &Schema,
    mut scope: Scope,
    mut inner: Select,
    output_names: Vec<String>,
    column_annotations: Vec<Option<AnnotationValue>>,
    projections: Vec<WindowProjection>,
    outer_conditions: querydown_parser::ast::ConditionSet,
    constants: &[ConstantDef],
    functions: &[FunctionDef],
) -> Result<StageOutput, String> {
    // Project each extracted window function as a hidden column on the inner select, aliased so
    // the outer query can filter on it.
    for projection in &projections {
        let sql = convert_expr(Expr::Window(projection.window.clone()), &mut scope)?;
        inner
            .columns
            .push(Column::new(sql, Some(projection.alias.clone())));
    }
    (inner.joins, inner.ctes) = scope.decompose_join_tree();

    let wrap_alias = scope.get_alias("qdwin_src");

    // Compile the outer (window) conditions against a synthetic table whose columns are the
    // inner select's outputs: the real result columns plus the projected window values.
    let mut wrap_column_names = output_names.clone();
    wrap_column_names.extend(projections.iter().map(|p| p.alias.clone()));
    let synthetic = build_pipeline_table(wrap_alias.clone(), &wrap_column_names);
    let mut outer_scope = Scope::build_with_base_table(options, schema, &synthetic);
    outer_scope.register_constants(constants.to_vec());
    outer_scope.register_functions(functions.to_vec());
    let outer_conditions_sql = convert_condition_set(outer_conditions, &mut outer_scope)?;

    let inner_cte = Cte {
        alias: wrap_alias.clone(),
        select: inner,
        join_column_name: String::new(),
    };
    let mut outer = Select::from(wrap_alias.clone());
    outer.columns = output_names
        .iter()
        .map(|name| {
            Column::new(
                SqlExpr::atom(options.dialect.table_column(&wrap_alias, name)),
                Some(name.clone()),
            )
        })
        .collect();
    outer.conditions = outer_conditions_sql;
    outer.ctes = vec![inner_cte];

    Ok(StageOutput {
        select: outer,
        output_names,
        column_annotations,
    })
}

/// Builds an explicit [`Column`] for every column of `base_table` (aliased to the column name),
/// returning the columns together with their names. Used when a wrapped stage has no explicit result
/// columns and must still expose each base-table column by name to the wrapping query.
fn expand_base_table_columns(base_table: &Table, scope: &Scope) -> (Vec<Column>, Vec<String>) {
    let mut entries: Vec<(usize, String)> = base_table
        .columns
        .values()
        .map(|c| (c.id, c.name.clone()))
        .collect();
    entries.sort_by_key(|(id, _)| *id);
    let mut columns = Vec::with_capacity(entries.len());
    let mut names = Vec::with_capacity(entries.len());
    for (_, name) in entries {
        let expr = scope.table_column_expr(&base_table.name, &name);
        columns.push(Column::new(expr, Some(name.clone())));
        names.push(name);
    }
    (columns, names)
}

/// The compiled output of a single pipeline stage.
struct StageOutput {
    select: Select,
    /// The names of the stage's output columns, used to build the synthetic table for the next
    /// stage. Only meaningful for non-final stages.
    output_names: Vec<String>,
    column_annotations: Vec<Option<AnnotationValue>>,
}

/// Resolves the query's base table by name, checking the user-defined tables in `extra_tables` first
/// and then the schema's tables.
fn resolve_base_table<'a>(
    options: &Options,
    schema: &'a Schema,
    name: &str,
    extra_tables: &'a [Table],
) -> Result<&'a Table, String> {
    let extra_lookup: HashMap<String, usize> = extra_tables
        .iter()
        .enumerate()
        .map(|(i, table)| (table.name.clone(), i))
        .collect();
    if let Some(&i) = options.resolve_identifier(&extra_lookup, name) {
        return Ok(&extra_tables[i]);
    }
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
            SchemaColumn {
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
