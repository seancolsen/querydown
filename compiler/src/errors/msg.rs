pub fn no_current_table() -> String {
    "Non-FK columns can only appear at the end of a path.".to_string()
}

pub fn col_not_in_table(column_name: &str, table_name: &str) -> String {
    format!("Column `{column_name}` not found within table `{table_name}`.")
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

pub fn unknown_variable(variable_name: &str) -> String {
    format!("Unknown variable `{}`.", variable_name)
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

pub fn aggregate_fn_applied_to_path_to_one() -> String {
    "Aggregate functions can only be applied to data that joins many records.".to_string()
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
    "Schema has multiple foreign keys from the same column".to_string()
}
