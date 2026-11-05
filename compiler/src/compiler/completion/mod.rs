//! Autocompletion: what could be typed at a given position in Querydown source.
//!
//! [`Compiler::complete`] works in three steps:
//!
//! 1. **The site** ([`site`]). It finds the word being typed before the cursor and the sigil that
//!    introduces it (`#` for a table, `|` for a function, `@` for a variable, and so on), using the
//!    highlighting tokens. It also recognizes places where nothing should be offered, such as
//!    strings, comments, numbers and flags, and it recognizes column annotations, whose keys and
//!    values only the application knows ([`annotation`]).
//!
//! 2. **The target** ([`locate`]). It replaces the word with a _sentinel_ identifier and parses the
//!    result with the real parser. Then it finds the sentinel in the syntax tree and works out what
//!    it is (a column, a table, a function name…) and, for a column, which table it belongs to, by
//!    following the same scoping rules as the compiler: paths like `author.team.`, scoped conditions
//!    like `issue{…}`, filters like `#comments{…}`, aggregate sorting, column globs, nesting groups,
//!    definitions and pipeline stages. Source being edited is often invalid, so when the source
//!    doesn't parse even with the sentinel, it tries again with the text after the cursor replaced
//!    by whatever closing brackets are missing. When that fails too, a lexical fallback guesses the
//!    target from the sigil and the base table.
//!
//! 3. **The items** ([`catalog`] and below). It lists what fits the target: the schema's tables and
//!    columns, computed columns and custom comparisons from the definitions, built-in and
//!    user-defined functions, constants and variables in scope.
//!
//! Completion never fails: anything it can't make sense of just yields no items.

mod annotation;
mod catalog;
mod locate;
mod site;
#[cfg(test)]
mod tests;

use querydown_parser::ast::Definitions;
use querydown_parser::highlight::{self, Grammar, TokenKind};
use querydown_parser::parse_definitions;
use serde::Serialize;

use crate::schema::links::Link;
use crate::schema::{Table, TableId};

use super::Compiler;
use locate::{Found, TableCtx, Target};
use site::{Sigil, SiteKind};

pub use annotation::{AnnotationPathSegment, AnnotationPosition, AnnotationSlot};

/// What to complete: a position within some Querydown source.
#[derive(Debug, Clone, Copy)]
pub struct CompletionRequest<'a> {
    /// The source being edited.
    pub source: &'a str,
    /// The cursor's position, as a byte offset into `source`.
    pub cursor: usize,
    /// How to read `source`: a whole query, or one section of a sectioned query.
    pub grammar: Grammar,
    /// For a section other than the definitions: the query's base table, which a sectioned query
    /// supplies separately. Columns are offered from this table.
    pub base_table: Option<&'a str>,
    /// For a section other than the definitions: the text of the definitions section, so that the
    /// constants, functions, computed columns and custom comparisons it defines can be offered.
    pub definitions: Option<&'a str>,
}

impl<'a> CompletionRequest<'a> {
    /// A request to complete at `cursor` within a whole query.
    pub fn query(source: &'a str, cursor: usize) -> Self {
        Self {
            source,
            cursor,
            grammar: Grammar::Query,
            base_table: None,
            definitions: None,
        }
    }
}

/// The result of [`Compiler::complete`]: the items that could replace the text from `from` to `to`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Completions {
    /// Where the text being completed begins (a byte offset). Typically the start of the word
    /// before the cursor, so that an item replaces what has been typed of it so far.
    pub from: usize,
    /// Where the text being completed ends (a byte offset): the cursor.
    pub to: usize,
    /// What could be typed, in no particular order. Filtering by what has been typed so far is left
    /// to the editor.
    pub items: Vec<CompletionItem>,
    /// Set when the cursor is within a column annotation (`@{…}`). Querydown doesn't know what
    /// annotations an application accepts, so it offers no items there; instead it says where in
    /// the annotation the cursor is, for the application to complete from its own knowledge.
    pub annotation: Option<AnnotationPosition>,
}

/// Something that could be typed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompletionItem {
    /// The name, as shown and matched against what has been typed, e.g. `due date`.
    pub label: String,
    pub kind: CompletionKind,
    /// The text to insert, when it differs from the label: e.g. `` `due date` ``, quoted because it
    /// isn't a plain identifier.
    pub insert: Option<String>,
    /// A short note shown beside the label, e.g. a column's type or a function's signature.
    pub detail: Option<String>,
    /// A longer description, e.g. what a function does.
    pub info: Option<String>,
    /// Ranks the item above (positive) or below (negative) others that match equally well, e.g.
    /// tables related to the current one above unrelated tables.
    pub boost: i32,
}

/// What kind of thing a [`CompletionItem`] names. Serialized in kebab-case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionKind {
    /// A table from the schema, or a user-defined table.
    Table,
    /// A column of a table (including one output by an earlier pipeline stage).
    Column,
    /// A computed column defined before the base table, e.g. `#users.age = …`.
    ComputedColumn,
    /// A custom comparison defined before the base table, e.g. `#issues.comment:@x = …`.
    CustomComparison,
    /// A scalar function, built-in or user-defined.
    Function,
    /// An aggregate function, applied with `%`.
    AggregateFunction,
    /// A window function, applied after a `%%( … )` window definition.
    WindowFunction,
    /// A built-in constant, e.g. `now`.
    Constant,
    /// A user-defined constant, a function parameter or a local assignment.
    Variable,
}

impl CompletionKind {
    pub const ALL: [CompletionKind; 9] = [
        CompletionKind::Table,
        CompletionKind::Column,
        CompletionKind::ComputedColumn,
        CompletionKind::CustomComparison,
        CompletionKind::Function,
        CompletionKind::AggregateFunction,
        CompletionKind::WindowFunction,
        CompletionKind::Constant,
        CompletionKind::Variable,
    ];

    /// The kebab-case name it is serialized as, e.g. `"computed-column"`.
    pub fn as_str(self) -> &'static str {
        match self {
            CompletionKind::Table => "table",
            CompletionKind::Column => "column",
            CompletionKind::ComputedColumn => "computed-column",
            CompletionKind::CustomComparison => "custom-comparison",
            CompletionKind::Function => "function",
            CompletionKind::AggregateFunction => "aggregate-function",
            CompletionKind::WindowFunction => "window-function",
            CompletionKind::Constant => "constant",
            CompletionKind::Variable => "variable",
        }
    }
}

impl Compiler {
    /// Lists what could be typed at the cursor in `request.source`. See the
    /// [module documentation](self).
    pub fn complete(&self, request: &CompletionRequest) -> Completions {
        let source = request.source;
        let mut cursor = request.cursor.min(source.len());
        while !source.is_char_boundary(cursor) {
            cursor -= 1;
        }
        let tokens = highlight::tokenize(source, request.grammar);
        let site = site::analyze(source, cursor, &tokens);
        let mut completions = Completions {
            from: site.word_start,
            to: cursor,
            items: vec![],
            annotation: None,
        };
        match site.kind {
            SiteKind::Blocked => completions,
            SiteKind::Annotation(position, from) => {
                completions.from = from;
                completions.annotation = Some(position);
                completions
            }
            SiteKind::Code(sigil) => {
                let located = locate::locate(self, request, &site, sigil, &tokens);
                let Located { found, definitions } =
                    located.unwrap_or_else(|| self.fallback(request, &site, sigil, &tokens));
                completions.items = self.items(&found, &definitions);
                completions
            }
        }
    }

    /// Guesses the target from the sigil alone, for source that doesn't parse even with the
    /// sentinel in it. Columns are offered from the base table (following any path written just
    /// before the word, like `author.team.`).
    fn fallback(
        &self,
        request: &CompletionRequest,
        site: &site::Site,
        sigil: Sigil,
        tokens: &[highlight::Token],
    ) -> Located {
        let source = request.source;
        // In a whole query, the base table is the first token the parser marked as one (it keeps
        // the marks of what it parsed before failing), and the definitions are what precedes it.
        let base_token = tokens
            .iter()
            .take_while(|token| token.span.end <= site.word_start)
            .find(|token| token.kind == TokenKind::BaseTable);
        let (base_table, definitions) = match request.grammar {
            Grammar::Query => (
                base_token.map(|token| unquote(source[token.span.clone()].trim_start_matches('#'))),
                base_token
                    .and_then(|token| parse_definitions(&source[..token.span.start]).ok())
                    .unwrap_or_default(),
            ),
            Grammar::Definitions => (None, Definitions::default()),
            _ => (
                request.base_table.map(str::to_string),
                section_definitions(request),
            ),
        };
        // Past a `~~~`, the columns are those of the previous stage, which can't be known here.
        let in_later_stage = tokens
            .iter()
            .any(|token| token.kind == TokenKind::Pipeline && token.span.end <= site.word_start);
        let base = match (&base_table, in_later_stage) {
            (Some(name), false) => locate::base_table_ctx(self, name, &definitions),
            _ => TableCtx::Unknown,
        };
        let target = match sigil {
            Sigil::Table if request.grammar == Grammar::Query && base_token.is_none() => {
                Target::BaseTable
            }
            Sigil::Table => Target::Table { from: base },
            Sigil::Function | Sigil::Pipe => Target::ScalarFunction,
            Sigil::Aggregate => Target::AggregateFunction,
            Sigil::Window => Target::WindowFunction,
            Sigil::Variable => Target::Variable,
            Sigil::None => {
                let path = site::path_before(source, site.word_start, tokens);
                Target::Column {
                    table: locate::follow_path(self, base, &path),
                    condition: false,
                }
            }
        };
        Located {
            found: Found {
                target,
                locals: vec![],
                excluded: None,
            },
            definitions,
        }
    }

    /// The items that fit `found`.
    fn items(&self, found: &Found, definitions: &Definitions) -> Vec<CompletionItem> {
        // A definition being typed contains the sentinel; it isn't something to offer.
        let defined = |name: &str| !name.starts_with(locate::SENTINEL);
        match &found.target {
            Target::Nothing => vec![],
            Target::BaseTable => {
                let mut items = self.table_items(None);
                items.extend(
                    definitions
                        .tables
                        .iter()
                        .filter(|def| defined(&def.name))
                        .map(|def| CompletionItem {
                            detail: Some("user-defined table".to_string()),
                            boost: 1,
                            ..item(&def.name, CompletionKind::Table)
                        }),
                );
                items
            }
            Target::DefinitionTable => self.table_items(None),
            Target::Table { from } => self.table_items(Some(from)),
            Target::Column { table, condition } => {
                let mut items = self.column_items(table);
                if let TableCtx::Schema(table_id) = table {
                    let table = &self.schema.tables[table_id];
                    let hosted_by_table = |name: &str| self.resolve_table(name) == Some(table.id);
                    items.extend(
                        definitions
                            .computed_columns
                            .iter()
                            .filter(|def| defined(&def.name) && hosted_by_table(&def.table))
                            .filter(|def| found.excluded.as_ref() != Some(&def.name))
                            .map(|def| CompletionItem {
                                detail: Some("computed".to_string()),
                                ..item(&def.name, CompletionKind::ComputedColumn)
                            }),
                    );
                    if *condition {
                        items.extend(
                            definitions
                                .custom_comparisons
                                .iter()
                                .filter(|def| defined(&def.name) && hosted_by_table(&def.table))
                                .filter(|def| !def.name.starts_with("__querydown"))
                                .map(|def| CompletionItem {
                                    detail: Some("comparison".to_string()),
                                    ..item(&def.name, CompletionKind::CustomComparison)
                                }),
                        );
                    }
                }
                items
            }
            Target::LinkingColumn { table, from } => self.linking_column_items(table, from),
            Target::ScalarFunction => {
                let mut items = catalog::scalar_function_items();
                items.extend(
                    definitions
                        .functions
                        .iter()
                        .filter(|def| defined(&def.name))
                        .map(|def| CompletionItem {
                            detail: Some(format!("@{} => …", def.params.join(" @"))),
                            info: Some("User-defined function".to_string()),
                            ..item(&def.name, CompletionKind::Function)
                        }),
                );
                items
            }
            Target::AggregateFunction => catalog::aggregate_function_items(),
            Target::WindowFunction => catalog::window_function_items(),
            Target::Variable => {
                let mut items = catalog::constant_items();
                let constants = definitions.constants.iter().map(|def| &def.name);
                let mut names: Vec<&String> = constants.chain(&found.locals).collect();
                names.retain(|name| defined(name));
                // A local may shadow a constant; offer the name once.
                names.sort();
                names.dedup();
                items.extend(
                    names
                        .into_iter()
                        .map(|name| item(name, CompletionKind::Variable)),
                );
                items
            }
        }
    }

    /// The schema's tables. When completing a table related to `from`, the tables it links to
    /// directly come first.
    fn table_items(&self, from: Option<&TableCtx>) -> Vec<CompletionItem> {
        let related: Vec<TableId> = match from {
            Some(TableCtx::Schema(id)) => self.schema.tables[id]
                .get_links()
                .map(|link| link.get_end().table_id)
                .collect(),
            _ => vec![],
        };
        let mut tables: Vec<&Table> = self.schema.tables.values().collect();
        tables.sort_by(|a, b| a.name.cmp(&b.name));
        tables
            .into_iter()
            .map(|table| CompletionItem {
                boost: i32::from(related.contains(&table.id)),
                ..item(&table.name, CompletionKind::Table)
            })
            .collect()
    }

    /// The columns of `table`, in the schema's order, with their types.
    fn column_items(&self, table: &TableCtx) -> Vec<CompletionItem> {
        match table {
            TableCtx::Unknown => vec![],
            TableCtx::Virtual(names) => names
                .iter()
                .map(|name| item(name, CompletionKind::Column))
                .collect(),
            TableCtx::Schema(table_id) => {
                let table = &self.schema.tables[table_id];
                let mut columns: Vec<_> = table.columns.values().collect();
                columns.sort_by_key(|column| column.id);
                columns
                    .into_iter()
                    .map(|column| {
                        let link = table
                            .forward_links_to_one
                            .get(&column.id)
                            .map(|link| &self.schema.tables[&link.get_end().table_id].name);
                        let detail = match (&column.type_name, link) {
                            (Some(type_name), Some(target)) => format!("{type_name} → {target}"),
                            (None, Some(target)) => format!("→ {target}"),
                            (Some(type_name), None) => type_name.clone(),
                            (None, None) => String::new(),
                        };
                        CompletionItem {
                            detail: (!detail.is_empty()).then_some(detail),
                            ..item(&column.name, CompletionKind::Column)
                        }
                    })
                    .collect()
            }
        }
    }

    /// The foreign key columns of `table`, which may name the link to follow in `#table(column)`.
    /// Those linking to `from` come first.
    fn linking_column_items(&self, table: &TableCtx, from: &TableCtx) -> Vec<CompletionItem> {
        let TableCtx::Schema(table_id) = table else {
            return vec![];
        };
        let table = &self.schema.tables[table_id];
        let mut links: Vec<_> = table.forward_links_to_one.iter().collect();
        links.sort_by_key(|(column_id, _)| **column_id);
        links
            .into_iter()
            .map(|(column_id, link)| {
                let target_id = link.get_end().table_id;
                let target = &self.schema.tables[&target_id].name;
                CompletionItem {
                    detail: Some(format!("→ {target}")),
                    boost: i32::from(matches!(from, TableCtx::Schema(id) if *id == target_id)),
                    ..item(&table.columns[column_id].name, CompletionKind::Column)
                }
            })
            .collect()
    }

    /// Finds a schema table by name, with the compiler's identifier resolution.
    fn resolve_table(&self, name: &str) -> Option<TableId> {
        self.options
            .resolve_identifier(&self.schema.table_lookup, name)
            .copied()
    }
}

/// The outcome of locating the target: what it is, plus the definitions in effect.
struct Located {
    found: Found,
    definitions: Definitions,
}

/// The definitions supplied alongside a section, or none if they don't parse.
fn section_definitions(request: &CompletionRequest) -> Definitions {
    request
        .definitions
        .and_then(|text| parse_definitions(text).ok())
        .unwrap_or_default()
}

/// An item with just a name and kind. The name is quoted with backticks for insertion if it isn't
/// a plain identifier.
fn item(name: &str, kind: CompletionKind) -> CompletionItem {
    CompletionItem {
        label: name.to_string(),
        kind,
        insert: (!is_plain_identifier(name)).then(|| quote(name)),
        detail: None,
        info: None,
        boost: 0,
    }
}

/// Whether `name` can be written without backticks.
fn is_plain_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// `name` quoted with backticks, as an identifier.
fn quote(name: &str) -> String {
    format!("`{}`", name.replace('\\', "\\\\").replace('`', "\\`"))
}

/// The name within a possibly backtick-quoted identifier.
fn unquote(text: &str) -> String {
    text.strip_prefix('`')
        .and_then(|text| text.strip_suffix('`'))
        .unwrap_or(text)
        .to_string()
}
