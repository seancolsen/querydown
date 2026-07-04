pub const CTE_ALIAS_PREFIX: &str = "cte";
pub const CTE_PK_COLUMN_ALIAS: &str = "pk";
pub const CTE_VALUE_COLUMN_PREFIX: &str = "v";

/// The prefix used to name the CTEs that carry the result of each non-final stage of a query
/// pipeline (stages separated by `~~~`). Each such CTE becomes the base "table" of the next stage.
pub const PIPELINE_CTE_ALIAS_PREFIX: &str = "pipe";

/// We may eventually make this configurable
pub const INDENT_SPACER: &str = "  ";

/// The name of the custom comparison that, when defined on a table, configures that table's default
/// text search (see [`super::expr::convert_condition_set`]). When a table has no such custom
/// comparison, the default text search falls back to searching all of the table's text-like columns.
pub const DEFAULT_TEXT_SEARCH_COMPARISON_NAME: &str = "__querydown_default_text_search";

/// The name of the custom comparison that, when defined on a table, supplies the meaning of comparing
/// directly against a linked record of that table (see [`super::comparisons::convert_comparison`]).
/// For example, defining `#users.__querydown_linked_record_comparison:@x = username:@x` lets
/// `#issues author:alice` mean `#issues author.username:alice`.
pub const LINKED_RECORD_COMPARISON_NAME: &str = "__querydown_linked_record_comparison";

pub const VAR_INFINITY: &str = "infinity";
pub const VAR_NOW: &str = "now";
pub const VAR_TRUE: &str = "true";
pub const VAR_FALSE: &str = "false";
pub const VAR_NULL: &str = "null";
