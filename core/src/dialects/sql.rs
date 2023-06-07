pub const AND: &str = "AND";
pub const COUNT_STAR: &str = "COUNT(*)";
pub const DIVIDE: &str = "/";
pub const EQ: &str = "=";
pub const FALSE: &str = "FALSE";
pub const GT: &str = ">";
pub const GTE: &str = ">=";
pub const INFINITY: &str = "INFINITY";
pub const LIKE: &str = "LIKE";
pub const LT: &str = "<";
pub const LTE: &str = "<=";
pub const MAX: &str = "MAX";
pub const MIN: &str = "MIN";
pub const MINUS: &str = "-";
pub const NEQ: &str = "<>";
pub const NLIKE: &str = "NOT LIKE";
pub const NOW: &str = "NOW()";
pub const NULL: &str = "NULL";
pub const OR: &str = "OR";
pub const PLUS: &str = "+";
pub const TIMES: &str = "*";
pub const TRUE: &str = "TRUE";
pub const ASC: &str = "ASC";
pub const DESC: &str = "DESC";
pub const NULLS_FIRST: &str = "NULLS FIRST";
pub const NULLS_LAST: &str = "NULLS LAST";

pub fn value_is_null(expr: String) -> String {
    format!("{expr} IS NULL")
}

pub fn value_is_not_null(expr: String) -> String {
    format!("{expr} IS NOT NULL")
}
