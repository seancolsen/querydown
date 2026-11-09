use serde::Serialize;

/// What a span of Querydown source _is_, for the purpose of syntax highlighting.
///
/// Each kind is a distinct role a piece of source can play, so that a highlighter can style them
/// independently — e.g. a bare word is a [`Column`](TokenKind::Column) reference in `$title` but a
/// [`String`](TokenKind::String) in `title:performance` and a [`SearchTerm`](TokenKind::SearchTerm)
/// when it stands alone as a condition.
///
/// Serialized (e.g. for the JS bindings) as the kebab-case name returned by
/// [`as_str`](TokenKind::as_str).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenKind {
    /// A line (`// ...`) or block (`/* ... */`) comment.
    Comment,
    /// A string literal: a quoted string (`"foo"`, `'foo'`) or a bare word on the right-hand side of
    /// a comparison (the `performance` in `title:performance`). Also an unquoted annotation value.
    String,
    /// An escape sequence within a quoted string or identifier, e.g. `\n` or `é`.
    StringEscape,
    /// A bare word standing alone as a condition, which searches the table's text columns, e.g.
    /// `backend` in `#issues backend`.
    SearchTerm,
    /// A number, e.g. `42` or `-1.5`.
    Number,
    /// A duration, e.g. `3d` or `1y6m`.
    Duration,
    /// A date, e.g. `@2024-01-31`.
    Date,
    /// A built-in constant, e.g. `@true`, `@null`, or `@now`.
    Constant,
    /// A user-defined constant, function parameter, or local assignment, e.g. `@user_id`.
    Variable,
    /// The base table of a query, e.g. `#issues` at the start of `#issues status:open`.
    BaseTable,
    /// Any other table: a related table (`#comments`, `>>project`), a user-defined table, or the
    /// table of a computed column or custom comparison definition.
    Table,
    /// A column reference, e.g. `title` or `` `due date` ``.
    Column,
    /// The name of a scalar function: piped (`|upper`), called directly (`@@max`), or defined
    /// (`@@fiscal_year = ...`).
    Function,
    /// The name of an aggregate or window function, e.g. `count` in `%count`.
    AggregateFunction,
    /// A function pipe: `|` (scalar), `%` (aggregate), or `%%` (window definition).
    Pipe,
    /// A comparison operator, e.g. `:`, `:=`, `:>=`, or `:~`.
    ComparisonOperator,
    /// Any other operator: arithmetic (`+ - * /`), negation (`!`), the `,` "OR" shorthand, a range
    /// (`..`, `<`), or the alias arrow (`->`).
    Operator,
    /// A "has" quantity prefix: `++` (at least one) or `--` (none).
    HasQuantity,
    /// A column glob, the `*` in `$*` or `$author.*`.
    Glob,
    /// The parts of a case expression: `?`, `~`, and `~~`.
    Case,
    /// A path separator: `.`, or the `>>` that leads to a table with a single related record.
    PathSeparator,
    /// The `$` that begins a result column.
    ResultColumn,
    /// The `\\` that begins a sorting expression.
    Sort,
    /// Column control or sort flags, e.g. `\sd` or `\g2`.
    Flag,
    /// The alias given to a result column, e.g. `total` in `$%count -> total`.
    Alias,
    /// The `@` that introduces a column annotation, e.g. in `@{width:100}`.
    Annotation,
    /// The key of an annotation object entry, e.g. `width` in `@{width:100}`.
    AnnotationKey,
    /// A scalar value within an annotation, e.g. `100` in `@{width:100}`.
    AnnotationValue,
    /// A bracket within an annotation: the `{ }` and `[ ]` of `@{tags:[a b]}`.
    AnnotationBracket,
    /// The `~~~` that separates the stages of a query pipeline.
    Pipeline,
    /// The `=` and `=>` within definitions, e.g. `@@plus_one = @x => @x + 1`.
    Definition,
    /// A bracket: `(`, `)`, `[`, `]`, `{`, or `}`.
    Bracket,
    /// Other punctuation, e.g. the `:` between an annotation key and its value.
    Punctuation,
    /// A character that can't begin any valid Querydown syntax.
    Invalid,
}

impl TokenKind {
    /// Every kind, in declaration order.
    pub const ALL: [TokenKind; 34] = [
        TokenKind::Comment,
        TokenKind::String,
        TokenKind::StringEscape,
        TokenKind::SearchTerm,
        TokenKind::Number,
        TokenKind::Duration,
        TokenKind::Date,
        TokenKind::Constant,
        TokenKind::Variable,
        TokenKind::BaseTable,
        TokenKind::Table,
        TokenKind::Column,
        TokenKind::Function,
        TokenKind::AggregateFunction,
        TokenKind::Pipe,
        TokenKind::ComparisonOperator,
        TokenKind::Operator,
        TokenKind::HasQuantity,
        TokenKind::Glob,
        TokenKind::Case,
        TokenKind::PathSeparator,
        TokenKind::ResultColumn,
        TokenKind::Sort,
        TokenKind::Flag,
        TokenKind::Alias,
        TokenKind::Annotation,
        TokenKind::AnnotationKey,
        TokenKind::AnnotationValue,
        TokenKind::AnnotationBracket,
        TokenKind::Pipeline,
        TokenKind::Definition,
        TokenKind::Bracket,
        TokenKind::Punctuation,
        TokenKind::Invalid,
    ];

    /// The kind's kebab-case name, e.g. `"aggregate-function"`. This is also its serialized form.
    pub fn as_str(self) -> &'static str {
        match self {
            TokenKind::Comment => "comment",
            TokenKind::String => "string",
            TokenKind::StringEscape => "string-escape",
            TokenKind::SearchTerm => "search-term",
            TokenKind::Number => "number",
            TokenKind::Duration => "duration",
            TokenKind::Date => "date",
            TokenKind::Constant => "constant",
            TokenKind::Variable => "variable",
            TokenKind::BaseTable => "base-table",
            TokenKind::Table => "table",
            TokenKind::Column => "column",
            TokenKind::Function => "function",
            TokenKind::AggregateFunction => "aggregate-function",
            TokenKind::Pipe => "pipe",
            TokenKind::ComparisonOperator => "comparison-operator",
            TokenKind::Operator => "operator",
            TokenKind::HasQuantity => "has-quantity",
            TokenKind::Glob => "glob",
            TokenKind::Case => "case",
            TokenKind::PathSeparator => "path-separator",
            TokenKind::ResultColumn => "result-column",
            TokenKind::Sort => "sort",
            TokenKind::Flag => "flag",
            TokenKind::Alias => "alias",
            TokenKind::Annotation => "annotation",
            TokenKind::AnnotationKey => "annotation-key",
            TokenKind::AnnotationValue => "annotation-value",
            TokenKind::AnnotationBracket => "annotation-bracket",
            TokenKind::Pipeline => "pipeline",
            TokenKind::Definition => "definition",
            TokenKind::Bracket => "bracket",
            TokenKind::Punctuation => "punctuation",
            TokenKind::Invalid => "invalid",
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_str_matches_serialization() {
        for kind in TokenKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
        }
    }

    #[test]
    fn test_all_is_unique() {
        let unique: std::collections::HashSet<_> = TokenKind::ALL.iter().collect();
        assert_eq!(unique.len(), TokenKind::ALL.len());
    }
}
