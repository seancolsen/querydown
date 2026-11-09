//! The messages of the errors the compiler reports, each as an [`Error`] without a location. The
//! caller locates it (via [`Error::at`]) at the syntax tree node at fault.

use crate::Error;

pub fn no_current_table() -> Error {
    Error::compile("Non-FK columns can only appear at the end of a path.")
}

pub fn col_not_in_table(column_name: &str, table_name: &str) -> Error {
    Error::compile(format!(
        "Column `{column_name}` not found within table `{table_name}`."
    ))
}

pub fn base_table_not_found(table_name: &str) -> Error {
    Error::compile(format!("Base table `{table_name}` does not exist."))
}

pub fn schema_has_no_tables() -> Error {
    Error::schema("The schema contains no tables.")
}

pub fn table_not_found(table_name: &str) -> Error {
    Error::compile(format!("Table `{table_name}` not found."))
}

pub fn no_path_to_table(from_table: &str, to_table: &str) -> Error {
    Error::compile(format!(
        "The table `{to_table}` is not related to the table `{from_table}`, so `#{to_table}` can't \
         be used here."
    ))
}

pub fn no_path_from_table_to_itself(table: &str) -> Error {
    Error::compile(format!(
        "The table `{table}` has no foreign key that refers to `{table}` itself, so `#{table}` \
         can't be used here. To reach other `{table}` records through a related table, name that \
         table first, as in `#other_table.#{table}`."
    ))
}

pub fn no_path_to_table_via_column(from_table: &str, to_table: &str, column: &str) -> Error {
    Error::compile(format!(
        "The table `{to_table}` is not related to the table `{from_table}` via a foreign key \
         column named `{column}`, so `#{to_table}({column})` can't be used here."
    ))
}

/// `linking_columns` lists the foreign key columns that would each pick one of the tied routes,
/// when naming a column is enough to resolve the ambiguity. Otherwise it's empty.
pub fn ambiguous_path_to_table(
    from_table: &str,
    to_table: &str,
    linking_columns: &[String],
) -> Error {
    let mut message = format!(
        "The table `{to_table}` is related to the table `{from_table}` in more than one equally \
         direct way, so `#{to_table}` is ambiguous here."
    );
    if !linking_columns.is_empty() {
        let options = linking_columns
            .iter()
            .map(|column| format!("`#{to_table}({column})`"))
            .collect::<Vec<_>>()
            .join(" or ");
        message.push_str(&format!(
            " Name the linking column to use, as in {options}."
        ));
    }
    Error::compile(message)
}

pub fn table_with_one_path_unsupported(table_name: &str) -> Error {
    Error::compile(format!(
        "Paths to a single related record via `>>` (as in `>>{table_name}`) are not yet supported."
    ))
}

pub fn no_path_parts() -> Error {
    Error::compile("Cannot build a ClarifiedPath without any path parts")
}

pub fn unknown_scalar_function(function_name: &str) -> Error {
    Error::compile(format!(
        "Scalar function `{}` does not exist.",
        function_name
    ))
}

pub fn unknown_aggregate_function(function_name: &str) -> Error {
    Error::compile(format!(
        "Aggregate function `{}` does not exist.",
        function_name
    ))
}

pub fn unknown_window_function(function_name: &str) -> Error {
    Error::compile(format!(
        "Window function `{}` does not exist.",
        function_name
    ))
}

pub fn window_in_indirect_condition() -> Error {
    Error::compile(
        "A window function can only be used in a condition when it's written directly among the \
     query's conditions (e.g. `#issues %%(project\\p created_at\\sd)%row_number:1`), not when \
     it's reached through a computed column, a custom comparison, a function's body, or a scoped \
     condition. To filter on it, compute it as a result column, and then filter on that column in \
     the next stage of a pipeline (`~~~`).",
    )
}

pub fn window_in_related_records() -> Error {
    Error::compile("Window functions can't be used with related records, e.g. in a filter like `#comments{…}` or \
     in a computed column that's aggregated across a to-many relationship. Instead, compute the \
     value as a result column of one stage of a pipeline (`~~~`), and use it in the next stage."
        )
}

pub fn window_in_aggregate() -> Error {
    Error::compile("Window functions can't be used within an aggregate function. Instead, compute the value as a \
     result column of one stage of a pipeline (`~~~`), and aggregate it in the next stage."
        )
}

pub fn window_in_window() -> Error {
    Error::compile("Window functions can't be nested within another window function. Instead, compute the inner \
     value as a result column of one stage of a pipeline (`~~~`), and use it in the next stage."
        )
}

pub fn window_in_grouped_stage() -> Error {
    Error::compile("A query can't combine `\\g` grouping with a window function in its result columns or sorting. \
     Use a pipeline (`~~~`) to group the output of a window function in a later stage, or to apply \
     a window function to the groups."
        )
}

pub fn window_fn_wrong_arg_count(
    function_name: &str,
    min_args: usize,
    max_args: usize,
    actual: usize,
) -> Error {
    Error::compile(if min_args == max_args {
        format!(
            "The window function `{function_name}` expects {min_args} argument(s) but received \
             {actual}."
        )
    } else {
        format!(
            "The window function `{function_name}` expects between {min_args} and {max_args} \
             arguments but received {actual}."
        )
    })
}

pub fn unknown_variable(variable_name: &str) -> Error {
    Error::compile(format!("Unknown variable `@{}`.", variable_name))
}

pub fn constant_refers_to_itself(name: &str) -> Error {
    Error::compile(format!(
        "The constant `@{name}` is defined in terms of itself."
    ))
}

pub fn aggregate_fn_applied_to_a_path_without_a_column() -> Error {
    Error::compile("A column must be specified when using an aggregate function.")
}

pub fn path_to_many_with_column_name_and_no_agg_fn(column_name: &str) -> Error {
    Error::compile(format!(
        "The column `{}` requires an aggregate function.",
        column_name
    ))
}

pub fn aggregate_fn_without_argument(function_name: &str) -> Error {
    Error::compile(format!(
        "The aggregate function `{}` requires an argument. Only `count` may be used on its own.",
        function_name
    ))
}

pub fn list_sorted_by_row_expr() -> Error {
    Error::compile("The `list` function removes duplicate values, so it can only be sorted by the listed value \
     itself (e.g. `%list(\\s)`) or by an aggregated sorting expression that summarizes the records \
     sharing each value (e.g. `%list(\\\\created_at%min)` or `%list(\\\\%count \\d)`). To sort \
     by each record without removing duplicates, use `list_all` instead."
        )
}

pub fn aggregated_sort_outside_list() -> Error {
    Error::compile(
        "Aggregated sorting expressions (e.g. `\\\\created_at%min` or `\\\\%count`) are only \
     supported within the `list` and `top` functions, where they order their distinct values.",
    )
}

pub fn top_without_sorting() -> Error {
    Error::compile("The `top` function collects the values that tie for first place, so it needs at least one \
     sorting entry to decide what comes first (e.g. `%top(\\\\%count \\d)` for the most frequent \
     values, or `%top(\\sd)` for the greatest value)."
        )
}

pub fn top_sorted_by_row_expr() -> Error {
    Error::compile("The `top` function removes duplicate values, so it can only be sorted by the value itself \
     (e.g. `%top(\\sd)`) or by an aggregated sorting expression that summarizes the records \
     sharing each value (e.g. `%top(\\\\%count \\d)` or `%top(\\\\created_at%max \\d)`)."
        )
}

pub fn top_in_grouped_query() -> Error {
    Error::compile(
        "The `top` function is only supported when collecting values from a related table (e.g. \
     `#comments.user.username%top(\\\\%count \\d)`), not when collecting a column of grouped \
     records.",
    )
}

pub fn aggregated_sort_in_grouped_list() -> Error {
    Error::compile("Aggregated sorting expressions (e.g. `\\\\created_at%min`) within `list` are only supported \
     when listing values from a related table (e.g. \
     `#comments.user.username%list(\\\\created_at%min)`), not when listing a column of grouped \
     records."
        )
}

pub fn value_sort_without_value(function_name: &str) -> Error {
    Error::compile(format!(
        "`\\s` sorts by the value being aggregated, but `%{}` is used on its own, without a value.",
        function_name
    ))
}

pub fn aggregate_fn_applied_to_a_non_path() -> Error {
    Error::compile("Aggregate functions must be applied directly to a column, without any intermediate computations. This restriction may be relaxed in future versions")
}

pub fn expected_one_arg() -> Error {
    Error::compile("Expected exactly one argument.")
}

pub fn expected_two_args() -> Error {
    Error::compile("Expected exactly two arguments.")
}

pub fn multiple_fk_from_col() -> Error {
    Error::schema("The schema has multiple links from the same column.")
}

pub fn column_glob_on_path_to_many() -> Error {
    Error::compile("Column globs (`*`) can not be used on paths that reference many records.")
}

pub fn column_glob_after_non_fk_column(column_name: &str) -> Error {
    Error::compile(format!("Column globs (`*`) can only be placed after foreign key columns. The column `{}` is not a foreign key column.", column_name))
}

/// If this happens, it's a bug somewhere up the stack.
///
/// TODO: we should improve the ClarifiedPath data structure to make this impossible
pub fn empty_path() -> Error {
    Error::compile("Bug: Empty path.")
}

pub fn ungrouped_unaggregated_column() -> Error {
    Error::compile("In a grouped query, every result column must either be a grouping column (marked with `\\g`) or contain an aggregate function.")
}

pub fn compare_two_ranges() -> Error {
    Error::compile("Cannot compare two ranges.")
}

pub fn compare_range_without_eq() -> Error {
    Error::compile("Range comparison must use the `:` operator.")
}

pub fn computed_column_unknown_table(table_name: &str) -> Error {
    Error::compile(format!(
        "Computed column refers to unknown table `{table_name}`."
    ))
}

pub fn function_wrong_arg_count(function_name: &str, expected: usize, actual: usize) -> Error {
    Error::compile(format!(
        "The function `{function_name}` expects {expected} argument(s) but received {actual}."
    ))
}

pub fn custom_comparison_unknown_table(table_name: &str) -> Error {
    Error::compile(format!(
        "Custom comparison refers to unknown table `{table_name}`."
    ))
}

pub fn custom_comparison_operator_mismatch(name: &str) -> Error {
    Error::compile(format!(
        "The custom comparison `{name}` cannot be called with this operator. The operator may only \
         be switched when the comparison is defined with `:` and every comparison in its body also \
         uses `:`."
    ))
}

pub fn custom_comparison_with_range(name: &str) -> Error {
    Error::compile(format!(
        "The custom comparison `{name}` cannot be used with a range."
    ))
}

pub fn no_default_text_search_columns(table_name: &str) -> Error {
    Error::compile(format!(
        "The table `{table_name}` has no text-like columns, so it cannot be used with the default \
         text search. Define a `__querydown_default_text_search` custom comparison on the table to \
         configure its default text search."
    ))
}

pub fn scope_is_not_a_single_related_record() -> Error {
    Error::compile("The path before a scoped condition set (e.g. the `issue` in `issue{ ... }`) must lead to a \
     single related record."
        )
}
