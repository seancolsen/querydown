mod aggregate_sorting;
mod comparisons;
pub mod completion;
mod constants;
mod expr;
mod functions;
mod join_tree;
mod paths;
mod rendering;
mod result_columns;
mod scope;
mod temporal;
mod typing;
mod window;

use std::collections::HashMap;

use querydown_parser::ast::{
    AnnotationValue, ComputedColumn, ConstantDef, CustomComparisonDef, Expr, ExprKind, FunctionDef,
    Query, TableDef, Transformation,
};
use querydown_parser::parse;
use serde::Serialize;

use crate::{
    error::Error,
    errors::msg,
    schema::{primitive_schema::PrimitiveSchema, Column as SchemaColumn, Schema, Table, ValueType},
    sql::tree::{Column, Cte, Select, SqlExpr},
    Options,
};

use self::{
    constants::PIPELINE_CTE_ALIAS_PREFIX,
    expr::{convert_condition_set, convert_expr},
    rendering::Render,
    result_columns::{convert_result_columns, convert_sort_exprs, ConvertedResultColumns},
    scope::{Scope, WindowBan},
    window::{extract_window_conditions, SplitConditions, WINDOW_PROJECTION_ALIAS_PREFIX},
};

/// The alias of the CTE that computes the window functions referenced by a stage's conditions (see
/// `compile_stage`).
const WINDOW_SOURCE_CTE_ALIAS: &str = "qdwin_src";

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
    /// Creates a compiler for the database described by `schema_json` (the JSON produced by
    /// running the dialect's [introspection query](crate::Dialect::introspection_sql)).
    ///
    /// # Errors
    ///
    /// Returns an [`ErrorKind::Schema`](crate::ErrorKind::Schema) error if `schema_json` is not
    /// valid JSON, doesn't have the expected structure, or is internally inconsistent (e.g. a link
    /// refers to a table that isn't in the schema).
    pub fn new(schema_json: &str, options: Options) -> Result<Self, Error> {
        let primitive_schema = serde_json::from_str::<PrimitiveSchema>(schema_json)
            .map_err(|e| Error::schema(format!("The schema is not valid: {e}.")))?;
        let schema = Schema::try_from(primitive_schema)?;
        Ok(Self { options, schema })
    }

    /// Compiles Querydown source code into SQL plus column annotations.
    ///
    /// # Errors
    ///
    /// - An [`ErrorKind::Syntax`](crate::ErrorKind::Syntax) error if `input` is not valid
    ///   Querydown. Its [`span`](Error::span) locates the problem within `input`; use
    ///   [`Error::render`] with `input` to display it.
    /// - An [`ErrorKind::Compile`](crate::ErrorKind::Compile) error if `input` is valid Querydown
    ///   that can't be compiled against the schema (e.g. it refers to a column that doesn't exist).
    ///   Its [`span`](Error::span) locates the construct at fault within `input`.
    pub fn compile(&self, input: String) -> Result<CompileResult, Error> {
        self.compile_query(parse(&input)?)
    }

    /// Compiles an already-parsed [`Query`] into SQL plus column annotations.
    ///
    /// This is the lower-level counterpart to [`Compiler::compile`], which parses source code
    /// before compiling it. It exists so that a consumer can parse the sections of a query
    /// independently — via the parser crate's section parsers — reassemble them with
    /// [`Query::from_parts`], and then compile the result, rather than handing over a single string.
    ///
    /// # Errors
    ///
    /// Returns an [`ErrorKind::Compile`](crate::ErrorKind::Compile) error if the query can't be
    /// compiled against the schema. Its [`span`](Error::span) locates the construct at fault, as
    /// recorded in the query's syntax tree: when the query was assembled from sections, the span is
    /// relative to the text of the error's [`section`](Error::section).
    pub fn compile_query(&self, query: Query) -> Result<CompileResult, Error> {
        let (udt_ctes, udt_tables) = self.compile_user_tables(&query.definitions.tables)?;

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
            .ok_or_else(msg::schema_has_no_tables)?;
        let mut render_scope = Scope::build_with_base_table(&self.options, &self.schema, any_table);

        Ok(CompileResult {
            sql: format!("{};", final_stage.select.render(&mut render_scope)),
            column_annotations: final_stage.column_annotations,
        })
    }

    /// Compiles each user-defined table (`#name = #( ... )`) into a CTE plus a synthetic `Table`
    /// (named after the definition) representing its output columns. The synthetic tables
    /// accumulate so that a later user-defined table — and ultimately the main query — can use an
    /// earlier one as its base table.
    fn compile_user_tables(&self, defs: &[TableDef]) -> Result<(Vec<Cte>, Vec<Table>), Error> {
        let mut udt_ctes: Vec<Cte> = Vec::new();
        let mut udt_tables: Vec<Table> = Vec::new();
        for table_def in defs {
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
        Ok((udt_ctes, udt_tables))
    }

    /// The names of the columns that `query` outputs, which are the columns available to a later
    /// pipeline stage (or to a query using it as a user-defined table). Used by autocompletion.
    pub(crate) fn output_column_names(&self, query: Query) -> Result<Vec<String>, Error> {
        let (_, udt_tables) = self.compile_user_tables(&query.definitions.tables)?;
        let stage = compile_pipeline(&self.options, &self.schema, query, &udt_tables, true)?;
        Ok(stage.output_names)
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
) -> Result<StageOutput, Error> {
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
                resolve_base_table(options, schema, &query.base_table, extra_tables)
                    .map_err(|err| err.at(query.base_table_loc))?
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
pub(super) fn convert_subquery(query: Query, scope: &mut Scope) -> Result<SqlExpr, Error> {
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
) -> Result<StageOutput, Error> {
    let mut scope = Scope::build_with_base_table(options, schema, base_table);
    // The query's definitions are in scope for every stage of the pipeline.
    scope.register_constants(constants.to_vec());
    scope.register_functions(functions.to_vec());
    scope.register_custom_comparisons(custom_comparisons.to_vec())?;
    scope.register_computed_columns(computed_columns.to_vec())?;

    // SQL disallows window functions in a `WHERE` clause. When a stage's conditions reference a
    // window function, the conditions that do are split off from the rest. An inner query (a CTE)
    // selects the base table's rows that pass the other conditions, along with the value of each
    // window function, computed over those rows. The stage then reads from that CTE in place of its
    // base table, under the base table's own alias, so that everything else about the stage —
    // result columns, grouping, sorting, related records — compiles exactly as it otherwise would.
    // Its `WHERE` applies the window conditions, referencing the precomputed values.
    let SplitConditions {
        inner: inner_conditions,
        outer: outer_conditions,
        windows,
    } = extract_window_conditions(transformation.conditions);

    let mut select = Select::from(base_table.name.clone());
    let mut window_source_cte = None;
    if windows.is_empty() {
        select.conditions = scope.with_window_ban(WindowBan::Condition, |scope| {
            convert_condition_set(inner_conditions, scope)
        })?;
    } else {
        let source_alias = scope.get_alias(WINDOW_SOURCE_CTE_ALIAS);
        let mut source_scope = Scope::build_with_base_table(options, schema, base_table);
        source_scope.register_constants(constants.to_vec());
        source_scope.register_functions(functions.to_vec());
        source_scope.register_custom_comparisons(custom_comparisons.to_vec())?;
        source_scope.register_computed_columns(computed_columns.to_vec())?;
        let mut source = Select::from(base_table.name.clone());
        source.conditions = source_scope.with_window_ban(WindowBan::Condition, |scope| {
            convert_condition_set(inner_conditions, scope)
        })?;
        (source.columns, _) = expand_base_table_columns(base_table, &source_scope);
        for (index, (window, loc)) in windows.into_iter().enumerate() {
            let alias = format!("{WINDOW_PROJECTION_ALIAS_PREFIX}{index}");
            let expr = Expr::new(ExprKind::Window(window.clone()), loc);
            let sql = convert_expr(expr, &mut source_scope)?;
            source.columns.push(Column::new(sql, Some(alias.clone())));
            let reference = scope.table_column_expr(scope.get_base_alias(), &alias);
            scope.add_window_substitution(window, reference);
        }
        (source.joins, source.ctes) = source_scope.decompose_join_tree();
        select.base_table = source_alias.clone();
        select.base_table_alias = Some(base_table.name.clone());
        window_source_cte = Some(Cte {
            alias: source_alias,
            select: source,
            // This CTE is used as the stage's base table (via `FROM`), not joined, so this field
            // is never consulted.
            join_column_name: String::new(),
        });
        select.conditions = scope.with_window_ban(WindowBan::Condition, |scope| {
            convert_condition_set(outer_conditions, scope)
        })?;
    }

    let standalone_sorting = convert_sort_exprs(transformation.sorting, &mut scope)?;
    let ConvertedResultColumns {
        mut columns,
        sorting: column_sorting,
        grouping,
        mut column_annotations,
        mut output_names,
    } = convert_result_columns(transformation.result_columns, &mut scope)?;
    if let Some(window_loc) = scope.compiled_window().filter(|_| !grouping.is_empty()) {
        return Err(msg::window_in_grouped_stage().at(window_loc));
    }
    if columns.is_empty() {
        // With no explicit result columns, the stage emits every column of its base table. We list
        // them explicitly rather than emitting `SELECT *` so that the result set is exactly the
        // set of columns Querydown knows about, in a deterministic order.
        let (expanded, names) = expand_base_table_columns(base_table, &scope);
        column_annotations = vec![None; expanded.len()];
        columns = expanded;
        output_names = names;
    }

    select.grouping = grouping;
    // Standalone `\\` sorts take precedence over column `\s` sorts, hence they come first.
    select.sorting = standalone_sorting
        .into_iter()
        .chain(column_sorting)
        .collect();

    let output_names = if is_last {
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
    if let Some(cte) = window_source_cte {
        select.ctes.insert(0, cte);
    }

    Ok(StageOutput {
        select,
        output_names,
        column_annotations,
    })
}

/// Builds an explicit, unaliased [`Column`] for every column of `base_table` (in column-id order),
/// returning the columns together with their names. Used when a stage has no explicit result
/// columns, so that it lists every known column rather than selecting `*`.
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
        columns.push(Column::new(expr, None));
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
) -> Result<&'a Table, Error> {
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
        .ok_or_else(|| msg::base_table_not_found(name))
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
                type_name: None,
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
