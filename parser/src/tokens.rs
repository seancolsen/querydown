pub(crate) const CASE_BEGIN: char = '?';
/// Separates a case variant's condition (first) from its value (second), e.g. the `~` in `a:1 ~ "x"`.
pub(crate) const CASE_VARIANT_SEPARATOR: char = '~';
/// Prefixes the fallback value of a case expression and marks its end, e.g. the `~~` in `~~ "other"`.
pub(crate) const CASE_FALLBACK: &str = "~~";
pub(crate) const COLUMN_ALIAS_PREFIX: &str = "->";
pub(crate) const COLUMN_CONTROL_FLAG_DESC: char = 'd';
pub(crate) const COLUMN_CONTROL_FLAG_GROUP: char = 'g';
pub(crate) const COLUMN_CONTROL_FLAG_HIDE: char = 'h';
pub(crate) const COLUMN_CONTROL_FLAG_NULLS_FIRST: char = 'n';
pub(crate) const COLUMN_CONTROL_FLAG_PARTITION: char = 'p';
pub(crate) const COLUMN_CONTROL_FLAG_SORT: char = 's';
pub(crate) const COLUMN_CONTROL_FLAGS_PREFIX: char = '\\';
pub(crate) const COLUMN_GLOB_ADJUSTMENT_BRACE_L: char = '(';
pub(crate) const COLUMN_GLOB_ADJUSTMENT_BRACE_R: char = ')';
pub(crate) const COLUMN_GLOB: char = '*';
pub(crate) const COLUMN_SPEC_PREFIX: char = '$';
/// Introduces a single-line comment, running to the end of the line.
pub(crate) const COMMENT_LINE: &str = "//";
/// Opens a block comment. Block comments may be nested.
pub(crate) const COMMENT_BLOCK_L: &str = "/*";
/// Closes a block comment.
pub(crate) const COMMENT_BLOCK_R: &str = "*/";
pub(crate) const COMPARE_MATCH: &str = ":";
pub(crate) const COMPARE_EQ: &str = ":=";
pub(crate) const COMPARE_GT: &str = ":>";
pub(crate) const COMPARE_GTE: &str = ":>=";
pub(crate) const COMPARE_LIKE: &str = ":~~";
pub(crate) const COMPARE_LT: &str = ":<";
pub(crate) const COMPARE_LTE: &str = ":<=";
pub(crate) const COMPARE_REGEX: &str = ":~";
pub(crate) const COMPARISON_RANGE_BOUND_SEPARATOR: &str = "..";
pub(crate) const COMPARISON_RANGE_BOUND_EXCLUSIVE: &str = "<";
pub(crate) const COMPOSITION_ARGUMENT_BRACE_L: char = '(';
pub(crate) const COMPOSITION_ARGUMENT_BRACE_R: char = ')';
pub(crate) const COMPOSITION_PIPE_AGGREGATE: char = '%';
pub(crate) const COMPOSITION_PIPE_SCALAR: char = '|';
pub(crate) const CONDITION_SET_AND_BRACE_L: char = '{';
pub(crate) const CONDITION_SET_AND_BRACE_R: char = '}';
pub(crate) const CONDITION_SET_OR_BRACE_L: char = '[';
pub(crate) const CONDITION_SET_OR_BRACE_R: char = ']';
/// Shorthand operator for joining expressions into an "OR" condition set without brackets, e.g.
/// `foo:1,bar:2`. This has the lowest precedence of any operator.
pub(crate) const CONDITION_SET_OR_SHORTHAND: char = ',';
pub(crate) const CONST_SIGIL: char = '@';
/// Prefix introducing a user-defined function definition, e.g. `@@fiscal_year = @date => ...`.
pub(crate) const FUNCTION_SIGIL: &str = "@@";
/// Separates a user-defined function's parameter list from its body, e.g. the `=>` in
/// `@date => (@date - 1m)|year`.
pub(crate) const FUNCTION_ARROW: &str = "=>";
/// Separates the left-hand side of a definition from its value, e.g. the `=` in a computed column
/// definition `#users.age = birth_date|age|years|floor`.
pub(crate) const DEFINITION_ASSIGN: char = '=';
pub(crate) const DB_IDENTIFIER_QUOTE: char = '`';
pub(crate) const EXPR_PAREN_L: char = '(';
pub(crate) const EXPR_PAREN_R: char = ')';
pub(crate) const EXPR_DIVIDE: char = '/';
pub(crate) const EXPR_TIMES: char = '*';
pub(crate) const EXPR_PLUS: char = '+';
pub(crate) const EXPR_MINUS: char = '-';
pub(crate) const HAS_QUANTITY_AT_LEAST_ONE: &str = "++";
pub(crate) const HAS_QUANTITY_ZERO: &str = "--";
pub(crate) const LITERAL_NULL: &str = "null";
pub(crate) const LITERAL_TRUE: &str = "true";
pub(crate) const LITERAL_FALSE: &str = "false";
/// Separates a key from its value within an annotation object entry, e.g. the `:` in `@{width:100}`.
pub(crate) const ANNOTATION_KEY_VALUE_SEPARATOR: char = ':';
/// Prefix for boolean negation of an expression, e.g. `!foo:2` or `!is_deleted`.
pub(crate) const NEGATE: char = '!';
pub(crate) const PATH_SEPARATOR: char = '.';
pub(crate) const PATH_TO_TABLE_WITH_ONE_PREFIX: &str = ">>";
/// Open a group of nested result column specs, written after a path head as `$issue.( ... )`.
pub(crate) const RESULT_COLUMNS_NESTING_BRACE_L: char = '(';
/// Closes a group of nested result column specs.
pub(crate) const RESULT_COLUMNS_NESTING_BRACE_R: char = ')';
/// Prefix denoting a standalone sorting expression, written as the literal `\\`.
pub(crate) const SORT_EXPR_PREFIX: &str = "\\\\";
/// Open a group of nested sorting expressions, written after a path head as `\\issue.( ... )`.
pub(crate) const SORTING_NESTING_BRACE_L: char = '(';
/// Closes a group of nested sorting expressions.
pub(crate) const SORTING_NESTING_BRACE_R: char = ')';
pub(crate) const STRING_ESCAPE_PREFIX: char = '\\';
pub(crate) const STRING_QUOTE_DOUBLE: char = '"';
pub(crate) const STRING_QUOTE_SINGLE: char = '\'';
pub(crate) const TABLE_SIGIL: char = '#';
pub(crate) const TABLE_WITH_MANY_COLUMN_BRACE_L: char = '(';
pub(crate) const TABLE_WITH_MANY_COLUMN_BRACE_R: char = ')';
pub(crate) const TRANSFORMATION_DELIMITER: &str = "~~~";
pub(crate) const WINDOW_DEFINITION_BRACE_L: char = '(';
pub(crate) const WINDOW_DEFINITION_BRACE_R: char = ')';
pub(crate) const WINDOW_DEFINITION_PREFIX: &str = "%%";
