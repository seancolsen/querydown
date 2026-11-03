pub fn no_current_table() -> String {
    "Non-FK columns can only appear at the end of a path.".to_string()
}

pub fn col_not_in_table(column_name: &str, table_name: &str) -> String {
    format!("Column `{column_name}` not found within table `{table_name}`.")
}

pub fn table_not_found(table_name: &str) -> String {
    format!("Table `{table_name}` not found.")
}

pub fn no_path_to_table(from_table: &str, to_table: &str) -> String {
    format!(
        "The table `{to_table}` is not related to the table `{from_table}`, so `#{to_table}` can't \
         be used here."
    )
}

pub fn no_path_from_table_to_itself(table: &str) -> String {
    format!(
        "The table `{table}` has no foreign key that refers to `{table}` itself, so `#{table}` \
         can't be used here. To reach other `{table}` records through a related table, name that \
         table first, as in `#other_table.#{table}`."
    )
}

pub fn no_path_to_table_via_column(from_table: &str, to_table: &str, column: &str) -> String {
    format!(
        "The table `{to_table}` is not related to the table `{from_table}` via a foreign key \
         column named `{column}`, so `#{to_table}({column})` can't be used here."
    )
}

/// `linking_columns` lists the foreign key columns that would each pick one of the tied routes,
/// when naming a column is enough to resolve the ambiguity. Otherwise it's empty.
pub fn ambiguous_path_to_table(
    from_table: &str,
    to_table: &str,
    linking_columns: &[String],
) -> String {
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
    message
}

pub fn table_with_one_path_unsupported(table_name: &str) -> String {
    format!(
        "Paths to a single related record via `>>` (as in `>>{table_name}`) are not yet supported."
    )
}

pub fn no_path_parts() -> String {
    "Cannot build a ClarifiedPath without any path parts".to_string()
}

pub fn unknown_scalar_function(function_name: &str) -> String {
    format!("Scalar function `{}` does not exist.", function_name)
}

pub fn unknown_aggregate_function(function_name: &str) -> String {
    format!("Aggregate function `{}` does not exist.", function_name)
}

pub fn unknown_window_function(function_name: &str) -> String {
    format!("Window function `{}` does not exist.", function_name)
}

pub fn window_fn_wrong_arg_count(
    function_name: &str,
    min_args: usize,
    max_args: usize,
    actual: usize,
) -> String {
    if min_args == max_args {
        format!(
            "The window function `{function_name}` expects {min_args} argument(s) but received \
             {actual}."
        )
    } else {
        format!(
            "The window function `{function_name}` expects between {min_args} and {max_args} \
             arguments but received {actual}."
        )
    }
}

pub fn unknown_variable(variable_name: &str) -> String {
    format!("Unknown variable `@{}`.", variable_name)
}

pub fn constant_refers_to_itself(name: &str) -> String {
    format!("The constant `@{name}` is defined in terms of itself.")
}

pub fn aggregate_fn_applied_to_a_path_without_a_column() -> String {
    "A column must be specified when using an aggregate function.".to_string()
}

pub fn path_to_many_with_column_name_and_no_agg_fn(column_name: &str) -> String {
    format!(
        "The column `{}` requires an aggregate function.",
        column_name
    )
}

pub fn aggregate_fn_without_argument(function_name: &str) -> String {
    format!(
        "The aggregate function `{}` requires an argument. Only `count` may be used on its own.",
        function_name
    )
}

pub fn list_sorted_by_row_expr() -> String {
    "The `list` function removes duplicate values, so it can only be sorted by the listed value \
     itself (e.g. `%list(\\s)`) or by an aggregated sorting expression that summarizes the records \
     sharing each value (e.g. `%list(\\\\created_at%min)` or `%list(\\\\%count \\d)`). To sort \
     by each record without removing duplicates, use `list_all` instead."
        .to_string()
}

pub fn aggregated_sort_outside_list() -> String {
    "Aggregated sorting expressions (e.g. `\\\\created_at%min` or `\\\\%count`) are only \
     supported within the `list` function, where they order its distinct values."
        .to_string()
}

pub fn aggregated_sort_in_grouped_list() -> String {
    "Aggregated sorting expressions (e.g. `\\\\created_at%min`) within `list` are only supported \
     when listing values from a related table (e.g. \
     `#comments.user.username%list(\\\\created_at%min)`), not when listing a column of grouped \
     records."
        .to_string()
}

pub fn value_sort_without_value(function_name: &str) -> String {
    format!(
        "`\\s` sorts by the value being aggregated, but `%{}` is used on its own, without a value.",
        function_name
    )
}

pub fn aggregate_fn_applied_to_a_non_path() -> String {
    "Aggregate functions must be applied directly to a column, without any intermediate computations. This restriction may be relaxed in future versions".to_string()
}

pub fn expected_one_arg() -> String {
    "Expected exactly one argument.".to_string()
}

pub fn expected_two_args() -> String {
    "Expected exactly two arguments.".to_string()
}

pub fn multiple_fk_from_col() -> String {
    "The schema has multiple links from the same column.".to_string()
}

pub fn column_glob_on_path_to_many() -> String {
    "Column globs (`*`) can not be used on paths that reference many records.".to_string()
}

pub fn column_glob_after_non_fk_column(column_name: &str) -> String {
    format!("Column globs (`*`) can only be placed after foreign key columns. The column `{}` is not a foreign key column.", column_name)
}

/// If this happens, it's a bug somewhere up the stack.
///
/// TODO: we should improve the ClarifiedPath data structure to make this impossible
pub fn empty_path() -> String {
    "Bug: Empty path.".to_string()
}

pub fn ungrouped_unaggregated_column() -> String {
    "In a grouped query, every result column must either be a grouping column (marked with `\\g`) or contain an aggregate function.".to_string()
}

pub fn compare_two_ranges() -> String {
    "Cannot compare two ranges.".to_string()
}

pub fn compare_range_without_eq() -> String {
    "Range comparison must use the `:` operator.".to_string()
}

pub fn computed_column_unknown_table(table_name: &str) -> String {
    format!("Computed column refers to unknown table `{table_name}`.")
}

pub fn function_wrong_arg_count(function_name: &str, expected: usize, actual: usize) -> String {
    format!("The function `{function_name}` expects {expected} argument(s) but received {actual}.")
}

pub fn custom_comparison_unknown_table(table_name: &str) -> String {
    format!("Custom comparison refers to unknown table `{table_name}`.")
}

pub fn custom_comparison_operator_mismatch(name: &str) -> String {
    format!(
        "The custom comparison `{name}` cannot be called with this operator. The operator may only \
         be switched when the comparison is defined with `:` and every comparison in its body also \
         uses `:`."
    )
}

pub fn custom_comparison_with_range(name: &str) -> String {
    format!("The custom comparison `{name}` cannot be used with a range.")
}

pub fn no_default_text_search_columns(table_name: &str) -> String {
    format!(
        "The table `{table_name}` has no text-like columns, so it cannot be used with the default \
         text search. Define a `__querydown_default_text_search` custom comparison on the table to \
         configure its default text search."
    )
}

pub fn scope_is_not_a_single_related_record() -> String {
    "The path before a scoped condition set (e.g. the `issue` in `issue{ ... }`) must lead to a \
     single related record."
        .to_string()
}
