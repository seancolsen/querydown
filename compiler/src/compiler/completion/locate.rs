//! Finding out what the word being typed is, by parsing the source with a sentinel in its place.
//!
//! The sentinel is an identifier that the user won't have written. Wherever the parser accepts a
//! name, it accepts the sentinel, so the syntax tree records what role the word plays: a column in
//! a path, a table, a function's name, a variable, a search term, and so on. Walking the tree to
//! the sentinel then reveals its context, such as which table a column belongs to, following the
//! same rules as the compiler.

use std::fmt::Debug;

use querydown_parser::ast::*;
use querydown_parser::highlight::{Grammar, Token};
use querydown_parser::{parse, parse_conditions, parse_definitions, parse_display, parse_sorting};

use super::site::{closing, open_brackets, Sigil, Site};
use super::{section_definitions, CompletionRequest, Located};
use crate::compiler::Compiler;
use crate::schema::links::Link;
use crate::schema::TableId;

/// The identifier that stands in for the word being typed. It's alphanumeric so that it parses
/// exactly as a typical word would (a bare search term may not contain underscores).
pub(super) const SENTINEL: &str = "QdCompletionCursor";

/// A name that stands in for a base table when the source before the cursor doesn't yet have one.
const PLACEHOLDER_BASE_TABLE: &str = "QdCompletionBase";

fn is_sentinel(name: &str) -> bool {
    name.starts_with(SENTINEL)
}

/// Whether the sentinel is somewhere within `node`. (The trees involved are small, so their debug
/// representation is a cheap way to find out.)
fn contains_sentinel<T: Debug>(node: &T) -> bool {
    format!("{node:?}").contains(SENTINEL)
}

/// The table whose columns are in scope.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TableCtx {
    /// A table in the schema.
    Schema(TableId),
    /// The output of a query: a pipeline stage or a user-defined table. Only the columns' names are
    /// known, and they have no links.
    Virtual(Vec<String>),
    /// Unknown, e.g. within a function definition (whose body could be used with any table), or
    /// after a path that doesn't resolve.
    Unknown,
}

/// What the word being typed is.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Target {
    /// The base table at the start of a query.
    BaseTable,
    /// A table related to `from`, as in `#comments` or `>>#projects`.
    Table {
        from: TableCtx,
    },
    /// The table that hosts a computed column or custom comparison definition.
    DefinitionTable,
    /// A column of `table`. `condition` means it stands as a condition (or the left of a
    /// comparison), where custom comparisons may also be used.
    Column {
        table: TableCtx,
        condition: bool,
    },
    /// The linking column in `#table(column)`.
    LinkingColumn {
        table: TableCtx,
        from: TableCtx,
    },
    ScalarFunction,
    AggregateFunction,
    WindowFunction,
    /// A constant or variable, after `@`.
    Variable,
    /// Something that can't be completed, such as a new name being defined, an alias, or a string
    /// on the right of a comparison.
    Nothing,
}

/// The target, plus the variables (function parameters and local assignments) in scope there.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Found {
    pub target: Target,
    pub locals: Vec<String>,
    /// A computed column not to offer, because it's the one being defined.
    pub excluded: Option<String>,
}

/// Parses the source with the sentinel in place of the word being typed, and finds what it is.
/// `None` if the source won't parse even then.
pub(super) fn locate(
    compiler: &Compiler,
    request: &CompletionRequest,
    site: &Site,
    sigil: Sigil,
    tokens: &[Token],
) -> Option<Located> {
    let source = request.source;
    let before = &source[..site.word_start];
    // A direct call's parentheses are required, so supply them if they're missing. (Unless a
    // function is being defined, so try without them too.)
    let mut probes = vec![format!("{before}{SENTINEL}")];
    if sigil == Sigil::Function && !source[site.word_end..].starts_with('(') {
        probes.insert(0, format!("{before}{SENTINEL}()"));
    }
    // First with the source as it is, then without what follows the cursor, closing any brackets
    // left open. A whole query also needs a base table, which the source before the cursor may not
    // have reached yet.
    let closers: String = open_brackets(source, site.word_start, tokens)
        .iter()
        .rev()
        .map(|(_, open)| closing(*open))
        .collect();
    let mut endings = vec![source[site.word_end..].to_string(), closers.clone()];
    if request.grammar == Grammar::Query {
        endings.push(format!("{closers} #{PLACEHOLDER_BASE_TABLE}"));
    }
    let attempts = endings
        .iter()
        .flat_map(|ending| probes.iter().map(move |probe| format!("{probe}{ending}")));
    let walker = Walker { compiler };
    attempts
        .into_iter()
        .find_map(|attempt| walker.attempt(request, &attempt))
}

struct Walker<'c> {
    compiler: &'c Compiler,
}

/// The scope of an expression being walked.
#[derive(Debug, Clone)]
struct Env {
    table: TableCtx,
    locals: Vec<String>,
    /// Whether the expression is on the right of a comparison, where bare words are strings.
    rhs: bool,
}

impl Env {
    fn new(table: TableCtx) -> Self {
        Env {
            table,
            locals: vec![],
            rhs: false,
        }
    }

    fn with_table(&self, table: TableCtx) -> Self {
        Env {
            table,
            rhs: false,
            ..self.clone()
        }
    }

    fn found(&self, target: Target) -> Option<Found> {
        Some(Found {
            target,
            locals: self.locals.clone(),
            excluded: None,
        })
    }
}

fn found(target: Target) -> Option<Found> {
    Some(Found {
        target,
        locals: vec![],
        excluded: None,
    })
}

impl Walker<'_> {
    /// Parses `text` with the request's grammar and walks the tree to the sentinel.
    fn attempt(&self, request: &CompletionRequest, text: &str) -> Option<Located> {
        let (found, definitions) = match request.grammar {
            Grammar::Query => {
                let query = parse(text).ok()?;
                (self.query(&query, &[]), query.definitions)
            }
            Grammar::Definitions => {
                let definitions = parse_definitions(text).ok()?;
                (self.definitions(&definitions, &[]), definitions)
            }
            grammar => {
                let definitions = section_definitions(request);
                let table = match request.base_table {
                    Some(name) => base_table_ctx(self.compiler, name, &definitions),
                    None => TableCtx::Unknown,
                };
                let env = Env::new(table);
                let found = match grammar {
                    Grammar::Conditions => {
                        let conditions = parse_conditions(text).ok()?;
                        self.entries(&conditions.entries, &env)
                    }
                    Grammar::Sorting => parse_sorting(text)
                        .ok()?
                        .iter()
                        .find_map(|sort| self.expr(&sort.expr, &env, false)),
                    _ => parse_display(text)
                        .ok()?
                        .iter()
                        .find_map(|statement| self.result_column(statement, &env)),
                };
                (found, definitions)
            }
        };
        // A tree without the sentinel means the parser dropped it somewhere that can't be
        // completed; let the fallback guess instead.
        Some(Located {
            found: found?,
            definitions,
        })
    }

    fn query(&self, query: &Query, outer_tables: &[TableDef]) -> Option<Found> {
        if let Some(found) = self.definitions(&query.definitions, outer_tables) {
            return Some(found);
        }
        if is_sentinel(&query.base_table) {
            return found(Target::BaseTable);
        }
        let (index, transformation) = query
            .transformations
            .iter()
            .enumerate()
            .find(|(_, transformation)| contains_sentinel(transformation))?;
        let table = self.stage_table(query, index, outer_tables);
        self.transformation(transformation, &Env::new(table))
    }

    /// The table that a query's stage `index` reads from: the base table for the first stage, or
    /// the previous stage's output.
    fn stage_table(&self, query: &Query, index: usize, outer_tables: &[TableDef]) -> TableCtx {
        let mut definitions = query.definitions.clone();
        definitions
            .tables
            .splice(0..0, outer_tables.iter().cloned());
        if index == 0 {
            return base_table_ctx(self.compiler, &query.base_table, &definitions);
        }
        let earlier_stages = Query {
            definitions,
            base_table: query.base_table.clone(),
            transformations: query.transformations[..index].to_vec(),
        };
        virtual_table(self.compiler, earlier_stages)
    }

    fn definitions(&self, definitions: &Definitions, outer_tables: &[TableDef]) -> Option<Found> {
        let Definitions {
            constants,
            functions,
            custom_comparisons,
            computed_columns,
            tables,
        } = definitions;
        for constant in constants {
            if is_sentinel(&constant.name) {
                return found(Target::Nothing);
            }
            // A constant is inlined wherever it's used, so no table is in scope.
            let env = Env::new(TableCtx::Unknown);
            if let Some(found) = self.expr(&constant.expr, &env, false) {
                return Some(found);
            }
        }
        for function in functions {
            if is_sentinel(&function.name) || function.params.iter().any(|p| is_sentinel(p)) {
                return found(Target::Nothing);
            }
            // Likewise a function's body.
            let mut env = Env::new(TableCtx::Unknown);
            env.locals.extend(function.params.iter().cloned());
            if let Some(found) = self.function_body(&function.body, env) {
                return Some(found);
            }
        }
        for comparison in custom_comparisons {
            if is_sentinel(&comparison.table) {
                return found(Target::DefinitionTable);
            }
            if is_sentinel(&comparison.name) || is_sentinel(&comparison.param) {
                return found(Target::Nothing);
            }
            let mut env = Env::new(self.schema_table(&comparison.table));
            env.locals.push(comparison.param.clone());
            if let Some(found) = self.condition_entry(&comparison.body, &env) {
                return Some(found);
            }
        }
        for computed_column in computed_columns {
            if is_sentinel(&computed_column.table) {
                return found(Target::DefinitionTable);
            }
            if is_sentinel(&computed_column.name) {
                return found(Target::Nothing);
            }
            let env = Env::new(self.schema_table(&computed_column.table));
            if let Some(mut found) = self.condition_entry(&computed_column.expr, &env) {
                // A computed column can't refer to itself.
                found.excluded = Some(computed_column.name.clone());
                return Some(found);
            }
        }
        let mut available_tables = outer_tables.to_vec();
        for table in tables {
            if is_sentinel(&table.name) {
                return found(Target::Nothing);
            }
            if let Some(found) = self.query(&table.query, &available_tables) {
                return Some(found);
            }
            available_tables.push(table.clone());
        }
        None
    }

    fn function_body(&self, body: &FunctionBody, mut env: Env) -> Option<Found> {
        for assignment in &body.assignments {
            if is_sentinel(&assignment.name) {
                return found(Target::Nothing);
            }
            if let Some(found) = self.expr(&assignment.expr, &env, false) {
                return Some(found);
            }
            env.locals.push(assignment.name.clone());
        }
        self.expr(&body.expr, &env, false)
    }

    fn transformation(&self, transformation: &Transformation, env: &Env) -> Option<Found> {
        self.entries(&transformation.conditions.entries, env)
            .or_else(|| {
                transformation
                    .sorting
                    .iter()
                    .find_map(|sort| self.expr(&sort.expr, env, false))
            })
            .or_else(|| {
                transformation
                    .result_columns
                    .iter()
                    .find_map(|statement| self.result_column(statement, env))
            })
    }

    fn result_column(&self, statement: &ResultColumnStatement, env: &Env) -> Option<Found> {
        match statement {
            ResultColumnStatement::Spec(spec) => self.column_spec(spec, env),
            ResultColumnStatement::Glob(glob) => {
                if let Some(found) = self.path(&glob.head, env, false) {
                    return Some(found);
                }
                let env = env.with_table(self.follow(&env.table, &glob.head));
                glob.specs
                    .iter()
                    .find_map(|spec| self.column_spec(spec, &env))
            }
        }
    }

    fn column_spec(&self, spec: &ColumnSpec, env: &Env) -> Option<Found> {
        if spec.alias.as_deref().is_some_and(is_sentinel) {
            return found(Target::Nothing);
        }
        self.expr(&spec.expr, env, false)
    }

    /// The entries of a boolean condition set, in which a bare word is a search term.
    fn entries(&self, entries: &[Expr], env: &Env) -> Option<Found> {
        entries
            .iter()
            .find_map(|entry| self.condition_entry(entry, env))
    }

    /// One condition. A bare word here is a search term, but the user may well be typing a column
    /// (to compare it), so columns are offered.
    fn condition_entry(&self, expr: &Expr, env: &Env) -> Option<Found> {
        self.expr(expr, env, true)
    }

    /// `condition` means that `expr` stands as a condition (or the left side of a comparison).
    fn expr(&self, expr: &Expr, env: &Env, condition: bool) -> Option<Found> {
        match expr {
            Expr::Number(_) | Expr::Date(_) | Expr::Duration(_) => None,
            Expr::String(s) if is_sentinel(s) => match env.rhs {
                true => env.found(Target::Nothing),
                // A search term.
                false => env.found(Target::Column {
                    table: env.table.clone(),
                    condition: true,
                }),
            },
            Expr::String(_) => None,
            Expr::Variable(name) if is_sentinel(name) => env.found(Target::Variable),
            Expr::Variable(_) => None,
            Expr::Path(parts) => self.path(parts, env, condition),
            Expr::ConditionSet(set) => match env.rhs {
                true => set.entries.iter().find_map(|e| self.expr(e, env, false)),
                false => self.entries(&set.entries, env),
            },
            Expr::ScopedConditionSet(scoped) => {
                if let Some(found) = self.path(&scoped.path, env, false) {
                    return Some(found);
                }
                let env = env.with_table(self.follow(&env.table, &scoped.path));
                self.entries(&scoped.condition_set.entries, &env)
            }
            Expr::HasQuantity(has) => self.path(&has.path_parts, env, false),
            Expr::Case(case) => case
                .variants
                .iter()
                .find_map(|variant| {
                    self.expr(&variant.condition, env, true)
                        .or_else(|| self.expr(&variant.value, env, false))
                })
                .or_else(|| self.expr(&case.fallback, env, false)),
            Expr::Call(call) => self.call(call, env),
            Expr::Product(a, b)
            | Expr::Quotient(a, b)
            | Expr::Sum(a, b)
            | Expr::Difference(a, b) => self
                .expr(a, env, false)
                .or_else(|| self.expr(b, env, false)),
            Expr::Comparison(comparison) => {
                let left = Env {
                    rhs: false,
                    ..env.clone()
                };
                let right = Env {
                    rhs: true,
                    ..env.clone()
                };
                self.comparison_side(&comparison.left, &left, true)
                    .or_else(|| self.comparison_side(&comparison.right, &right, false))
            }
            Expr::Not(inner) => self.expr(inner, env, condition),
            Expr::Window(window) => {
                if is_sentinel(&window.function) {
                    return env.found(Target::WindowFunction);
                }
                window
                    .args
                    .iter()
                    .chain(&window.partition_by)
                    .chain(window.order_by.iter().map(|sort| &sort.expr))
                    .find_map(|e| self.expr(e, env, false))
            }
            Expr::AnonymousFunctionCall(call) => {
                if let Some(found) = call.args.iter().find_map(|e| self.expr(e, env, false)) {
                    return Some(found);
                }
                if call.params.iter().any(|p| is_sentinel(p)) {
                    return env.found(Target::Nothing);
                }
                // Applied right where it's written, so the surrounding table remains in scope.
                let mut env = env.clone();
                env.locals.extend(call.params.iter().cloned());
                self.function_body(&call.body, env)
            }
            Expr::Subquery(query) => self.query(query, &[]),
        }
    }

    fn comparison_side(&self, side: &ComparisonSide, env: &Env, condition: bool) -> Option<Found> {
        match side {
            ComparisonSide::Expr(e) => self.expr(e, env, condition),
            ComparisonSide::Expansion(set) => set
                .entries
                .iter()
                .find_map(|e| self.expr(e, env, condition)),
            ComparisonSide::Range(range) => self
                .expr(&range.lower.expr, env, false)
                .or_else(|| self.expr(&range.upper.expr, env, false)),
        }
    }

    fn call(&self, call: &Call, env: &Env) -> Option<Found> {
        if is_sentinel(&call.name) {
            return env.found(match call.dimension {
                FunctionDimension::Scalar => Target::ScalarFunction,
                FunctionDimension::Aggregate => Target::AggregateFunction,
            });
        }
        if let Some(found) = call.args.iter().find_map(|e| self.expr(e, env, false)) {
            return Some(found);
        }
        // An aggregate's sorting entries are evaluated from the first `#` table of the aggregated
        // path, or from the current table when there is none.
        let sort_table = match call.args.first() {
            Some(Expr::Path(parts)) => {
                match parts
                    .iter()
                    .position(|part| matches!(part, PathPart::TableWithMany(_)))
                {
                    Some(index) => self.follow(&env.table, &parts[..=index]),
                    None => env.table.clone(),
                }
            }
            _ => env.table.clone(),
        };
        let sort_env = env.with_table(sort_table);
        call.order_by.iter().find_map(|sort| match &sort.key {
            AggregateSortKey::Expr(e) => self.expr(e, &sort_env, false),
            AggregateSortKey::Value => None,
        })
    }

    /// Walks a path from `env`'s table, one part at a time.
    fn path(&self, parts: &[PathPart], env: &Env, condition: bool) -> Option<Found> {
        let mut table = env.table.clone();
        for part in parts {
            match part {
                PathPart::Column(name) if is_sentinel(name) => {
                    return env.found(Target::Column { table, condition });
                }
                PathPart::TableWithOne(name) if is_sentinel(name) => {
                    return env.found(Target::Table { from: table });
                }
                PathPart::TableWithMany(many) if is_sentinel(&many.table) => {
                    return env.found(Target::Table { from: table });
                }
                PathPart::TableWithMany(many) => {
                    let target = self.schema_table(&many.table);
                    if many.linking_column.as_deref().is_some_and(is_sentinel) {
                        return env.found(Target::LinkingColumn {
                            table: target,
                            from: table,
                        });
                    }
                    let filter_env = env.with_table(target.clone());
                    if let Some(found) = self.entries(&many.condition_set.entries, &filter_env) {
                        return Some(found);
                    }
                    table = target;
                }
                _ => table = self.follow(&table, std::slice::from_ref(part)),
            }
        }
        None
    }

    fn follow(&self, table: &TableCtx, parts: &[PathPart]) -> TableCtx {
        follow_path(self.compiler, table.clone(), parts)
    }

    fn schema_table(&self, name: &str) -> TableCtx {
        self.compiler
            .resolve_table(name)
            .map_or(TableCtx::Unknown, TableCtx::Schema)
    }
}

/// The table reached by following `parts` from `table`. Columns lead along their foreign keys, and
/// `#table` parts to the named table.
pub(super) fn follow_path(compiler: &Compiler, table: TableCtx, parts: &[PathPart]) -> TableCtx {
    let schema_table = |name: &str| {
        compiler
            .resolve_table(name)
            .map_or(TableCtx::Unknown, TableCtx::Schema)
    };
    parts.iter().fold(table, |table, part| match part {
        PathPart::Column(name) => {
            let TableCtx::Schema(table_id) = table else {
                return TableCtx::Unknown;
            };
            let table = &compiler.schema.tables[&table_id];
            compiler
                .options
                .resolve_identifier(&table.column_lookup, name)
                .and_then(|column_id| table.forward_links_to_one.get(column_id))
                .map_or(TableCtx::Unknown, |link| {
                    TableCtx::Schema(link.get_end().table_id)
                })
        }
        PathPart::TableWithOne(name) => schema_table(name),
        PathPart::TableWithMany(many) => schema_table(&many.table),
    })
}

/// The table a query is based on: a user-defined table from `definitions`, or one in the schema.
pub(super) fn base_table_ctx(
    compiler: &Compiler,
    name: &str,
    definitions: &Definitions,
) -> TableCtx {
    let is_user_table = definitions.tables.iter().any(|table| {
        let lookup = std::collections::HashMap::from([(table.name.clone(), ())]);
        compiler.options.resolve_identifier(&lookup, name).is_some()
    });
    if is_user_table {
        // The user-defined table's columns are the output of a query that reads all of them.
        return virtual_table(
            compiler,
            Query {
                definitions: definitions.clone(),
                base_table: name.to_string(),
                transformations: vec![],
            },
        );
    }
    compiler
        .resolve_table(name)
        .map_or(TableCtx::Unknown, TableCtx::Schema)
}

/// The output columns of `query`, or unknown if it doesn't compile.
fn virtual_table(compiler: &Compiler, query: Query) -> TableCtx {
    compiler
        .output_column_names(query)
        .map_or(TableCtx::Unknown, TableCtx::Virtual)
}
