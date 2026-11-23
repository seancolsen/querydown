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

pub fn sorting_within_union() -> Error {
    Error::compile(
        "A query within a union (`+++`) can't be sorted, because the union doesn't keep the order \
         of its queries' rows. To sort the union's result, add a pipeline stage after it, e.g. \
         `~~~ \\\\created_at`.",
    )
}

pub fn union_column_count_mismatch(expected: usize, actual: usize) -> Error {
    let columns = |n: usize| {
        if n == 1 {
            "1 column".to_string()
        } else {
            format!("{n} columns")
        }
    };
    Error::compile(format!(
        "Each query in a union (`+++`) must have the same number of result columns. This query has \
         {}, but the first has {}.",
        columns(actual),
        columns(expected),
    ))
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

pub fn no_relationships_from_derived_table(to_table: &str) -> Error {
    Error::compile(format!(
        "`#{to_table}` can't be used here, because the result of an earlier pipeline stage (or a \
         user-defined table) has no relationships to other tables. Compute what you need from \
         `#{to_table}` in the stage that reads the original table, and output it as a result column."
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

/// A table or column name as it would be written in Querydown: quoted with backticks unless it's a
/// plain identifier.
fn written_identifier(name: &str) -> String {
    let mut chars = name.chars();
    let is_plain = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if is_plain {
        name.to_owned()
    } else {
        format!("`{name}`")
    }
}

/// `columns` holds the foreign key columns that lead to the one related record, when following
/// columns is enough to reach it.
pub fn path_to_table_reaches_one_record(
    from_table: &str,
    to_table: &str,
    linking_column: Option<&str>,
    columns: Option<&[String]>,
) -> Error {
    let part = match linking_column {
        Some(column) => format!("#{to_table}({column})"),
        None => format!("#{to_table}"),
    };
    let error = Error::compile(format!(
        "Each record of the table `{from_table}` is related to only one record of the table \
         `{to_table}`, but `{part}` refers to many related records, so it can't be used here."
    ));
    match columns {
        Some(columns) => error.with_hint(format!(
            "To refer to the one related record, follow the linking column instead, as in `{}`.",
            columns
                .iter()
                .map(|column| written_identifier(column))
                .collect::<Vec<_>>()
                .join(".")
        )),
        None => error,
    }
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

pub fn aggregate_fn_not_applied_to_a_column() -> Error {
    Error::compile(
        "An aggregate function must be applied to a column, optionally followed by scalar \
         functions that compute the value to aggregate for each record (e.g. \
         `#comments.body|length%avg`). It can't be applied to another kind of expression.",
    )
}

pub fn expected_one_arg() -> Error {
    Error::compile("Expected exactly one argument.")
}

pub fn expected_two_args() -> Error {
    Error::compile("Expected exactly two arguments.")
}

/// A function with optional arguments was given too few or too many. The counts include the piped
/// value.
pub fn expected_args_between(min: usize, max: usize, actual: usize) -> Error {
    Error::compile(format!(
        "Expected between {min} and {max} arguments, but received {actual}."
    ))
}

pub fn json_path_without_steps() -> Error {
    Error::compile(
        "`json_path` needs at least one step: an object's key or an array's position, as in \
         `json_path(\"items\" 0)`.",
    )
}

pub fn json_object_key_without_value() -> Error {
    Error::compile(
        "`json_object` takes pairs of keys and values, but this key has no value after it.",
    )
}

pub fn expected_at_least_one_arg() -> Error {
    Error::compile("Expected at least one argument.")
}

/// An argument that must be written as a literal (such as the `flags` of `regex_replace`) wasn't.
/// `example` is a literal that would do.
pub fn argument_not_literal(param: &str, kind: &str, example: &str) -> Error {
    Error::compile(format!(
        "The `{param}` argument must be a literal {kind}, as in `{example}`."
    ))
}

pub fn argument_not_whole_number(param: &str) -> Error {
    Error::compile(format!(
        "The `{param}` argument must be a whole number of 0 or more."
    ))
}

pub fn unknown_regex_flag(letter: char) -> Error {
    Error::compile(if letter == 'i' {
        "Regex matching is already case-insensitive, so there is no `i` flag. Use `c` to make it \
         case-sensitive."
            .to_string()
    } else {
        format!(
            "Unknown regex flag `{letter}`. The flags are `c` (case-sensitive), `m` (`^` and `$` \
             match at line breaks), and `s` (`.` matches a newline)."
        )
    })
}

pub fn repeated_regex_flag(letter: char) -> Error {
    Error::compile(format!("Repeated regex flag `{letter}`."))
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

pub fn outer_record_unreachable() -> Error {
    Error::compile(
        "A value passed to a function or a custom comparison is being used somewhere that its \
         record can't be reached from. A value that refers to a record (to one of its columns, \
         say) can be used within a filter on that record's related records, like `#comments{…}`, \
         but no deeper: not within a filter nested inside that one, nor within a filter on the \
         related records of some other record (as in `author.#comments{…}`).",
    )
}

pub fn condition_set_as_value() -> Error {
    Error::compile(
        "A set of values can only expand a comparison when it's written directly in the \
         comparison, as in `status:[open reopened]`. A set that gets there some other way (held by \
         a constant, passed as an argument, or wrapped in parentheses) is a single condition \
         instead, which can't be compared with a value. Write the set out in the comparison.",
    )
}

pub fn negated_value() -> Error {
    Error::compile(
        "`!` negates a condition, but this is a value. To negate a comparison, put the `!` before \
         the whole comparison, e.g. `!status:closed`. Within an expansion, write each comparison \
         out instead: `[status:open !status:closed]`.",
    )
}

pub fn literal_as_condition(kind: &str) -> Error {
    Error::compile(format!(
        "A {kind} can't stand alone as a condition. If it follows a comma, note that a comma \
         separates whole conditions: to compare one column with several values, use an expansion \
         like `id:[1 2]`."
    ))
}

pub fn aggregate_in_condition() -> Error {
    Error::compile(
        "An aggregate of the records being queried (like `%count` or `created_at%max`) can't be \
         used in a condition, because conditions choose the records that get aggregated. To \
         filter on an aggregate, compute it as a result column, and then filter on that column in \
         the next stage of a pipeline, e.g. `#issues $status \\g $%count -> n ~~~ n:>5`.",
    )
}

pub fn aggregate_in_aggregated_value() -> Error {
    Error::compile(
        "An aggregate function can't be used within the value that another aggregate function \
         aggregates (such as `body%max` in `#comments.body|concat(body%max)%list`, or in the \
         definition of a computed column that's aggregated), because that value is computed for \
         each record before the records are aggregated. An aggregate of records related to each \
         one is fine, e.g. `#issues.title|concat(#comments)%list`.",
    )
}

pub fn aggregate_in_related_records_filter() -> Error {
    Error::compile(
        "An aggregate of the related records themselves (like `%count` or `created_at%max`) \
         can't be used within their own filter, because the filter chooses the records that get \
         aggregated. An aggregate of records related to *them* is fine, e.g. \
         `#comments{#reactions:>5}`.",
    )
}

pub fn related_aggregate_in_grouped_query() -> Error {
    Error::compile(
        "A count or aggregate of related records (like `#comments`) has a value for each record, \
         not for each group, so in a grouped query it must either be a grouping column (marked \
         with `\\g`) or be aggregated across the group. To aggregate it, name it with a computed \
         column first, e.g. define `#issues.comment_count = #comments` and then use \
         `$comment_count%sum`.",
    )
}

pub fn ungrouped_unaggregated_sort() -> Error {
    Error::compile(
        "In a grouped query, the result can only be sorted by a grouping column (one marked with \
         `\\g`) or by an expression that contains an aggregate function.",
    )
}

/// The functions that take a function as an argument, for messages about them.
const FUNCTIONS_TAKING_A_FUNCTION: &str = "`map`, `filter`, `some`, `every`, or `reduce`";

/// A function (`@e => …`) was given as an argument to something other than the functions that
/// apply one.
pub fn function_arg_not_accepted() -> Error {
    Error::compile(format!(
        "A function (like `@e => @e:>0`) can only be given to {FUNCTIONS_TAKING_A_FUNCTION}, which \
         apply it to the elements of an array."
    ))
}

/// One of the functions that apply a function wasn't given one, where `example` shows it given one.
pub fn function_arg_expected(function_name: &str, example: &str) -> Error {
    Error::compile(format!(
        "`{function_name}` needs a function to apply to each element of the array, as in \
         `{example}`."
    ))
}

/// The function given to `function_name` has the wrong number of parameters, which `expected`
/// describes.
pub fn function_arg_param_count(function_name: &str, expected: &str) -> Error {
    Error::compile(format!(
        "The function given to `{function_name}` takes {expected}."
    ))
}

/// A parameter of a function given to `map` (or the like) was used within a query of its own, such
/// as that of a path to related records, which can't see the elements of the array.
pub fn element_param_in_other_query(name: &str) -> Error {
    Error::compile(format!(
        "`@{name}` stands for an element of an array, so it can't be used within a filter of \
         related records (like `#comments{{…}}`), or anything else that's found by a query of its \
         own, which can't see the array's elements."
    ))
}

pub fn subquery_in_lambda() -> Error {
    Error::compile(format!(
        "In DuckDB, a function given to {FUNCTIONS_TAKING_A_FUNCTION} can't use a subquery \
         (`#( … )`), because it becomes a DuckDB lambda, which can't contain one."
    ))
}

pub fn aggregate_in_function_arg() -> Error {
    Error::compile(format!(
        "An aggregate of the records being queried (like `%count` or `created_at%max`) can't be \
         used within a function given to {FUNCTIONS_TAKING_A_FUNCTION}, which applies it to the \
         elements of an array, one record at a time. An aggregate of related records is fine, \
         e.g. `#comments%count`."
    ))
}

pub fn window_in_function_arg() -> Error {
    Error::compile(format!(
        "Window functions can't be used within a function given to \
         {FUNCTIONS_TAKING_A_FUNCTION}. Instead, compute the value as a result column of one \
         stage of a pipeline (`~~~`), and use it in the next stage."
    ))
}
