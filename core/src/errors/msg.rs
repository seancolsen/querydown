pub fn no_current_table() -> String {
    "Non-FK columns can only appear at the end of a path.".to_string()
}

pub fn col_not_in_table(column_name: &str, table_name: &str) -> String {
    format!("Column `{column_name}` not found within table `{table_name}`.")
}

pub fn no_path_parts() -> String {
    "Cannot build a ClarifiedPath without any path parts".to_string()
}

pub fn no_column_name_or_chain() -> String {
    "Cannot build a ClarifiedPathTail without a column name or chain".to_string()
}

pub fn multiple_agg_fns() -> String {
    "Cannot apply more than one aggregate function to the same expression.".to_string()
}

pub fn pre_aggregate_composition_without_column() -> String {
    "Functions can only be applied before aggregation when a column is specified.".to_string()
}

pub fn special_aggregate_composition_applied_without_column(function_name: String) -> String {
    format!(
        "Aggregate function `{}` can only be applied to a column.",
        function_name
    )
}

pub fn multiple_fk_from_col() -> String {
    "Schema has multiple foreign keys from the same column".to_string()
}
