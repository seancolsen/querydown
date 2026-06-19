pub const CTE_ALIAS_PREFIX: &str = "cte";
pub const CTE_PK_COLUMN_ALIAS: &str = "pk";
pub const CTE_VALUE_COLUMN_PREFIX: &str = "v";

/// The prefix used to name the CTEs that carry the result of each non-final stage of a query
/// pipeline (stages separated by `~~~`). Each such CTE becomes the base "table" of the next stage.
pub const PIPELINE_CTE_ALIAS_PREFIX: &str = "pipe";

/// We may eventually make this configurable
pub const INDENT_SPACER: &str = "  ";

pub const VAR_INFINITY: &str = "infinity";
pub const VAR_NOW: &str = "now";
pub const VAR_TRUE: &str = "true";
pub const VAR_FALSE: &str = "false";
pub const VAR_NULL: &str = "null";
