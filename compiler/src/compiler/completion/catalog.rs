//! The built-in functions and constants, described for completion. Tests check that these lists
//! stay in step with the functions the compiler actually provides.

use super::{item, CompletionItem, CompletionKind};

/// A built-in function or constant: its name, how it is used, and what it does.
struct Entry {
    name: &'static str,
    usage: &'static str,
    description: &'static str,
}

const fn entry(name: &'static str, usage: &'static str, description: &'static str) -> Entry {
    Entry {
        name,
        usage,
        description,
    }
}

/// The built-in scalar functions, applied with `|` (or called with `@@`).
#[rustfmt::skip]
const SCALAR_FUNCTIONS: [Entry; 38] = [
    entry("abs", "x|abs", "The absolute value of a number."),
    entry("age", "x|age", "The interval from a timestamp until now (now − x). Chain with `days`, `years`, etc."),
    entry("and", "x|and(y…)", "True when all of the arguments are true."),
    entry("ceil", "x|ceil", "Rounds a number up to the nearest integer."),
    entry("concat", "x|concat(y…)", "Joins strings together."),
    entry("countdown", "x|countdown", "The interval from now until a timestamp (x − now). Chain with `days`, `hours`, etc."),
    entry("day", "x|day", "The day of the month (1–31) of a date or timestamp."),
    entry("days", "x|days", "Converts an interval into a number of days."),
    entry("divide", "x|divide(y)", "Divides the value by the argument."),
    entry("exp", "x|exp", "Raises e to the power of the value."),
    entry("floor", "x|floor", "Rounds a number down to the nearest integer."),
    entry("hours", "x|hours", "Converts an interval into a number of hours."),
    entry("if_null", "x|if_null(fallback…)", "The first of its arguments that isn't null (SQL `COALESCE`)."),
    entry("keep_above", "x|keep_above(min)", "The greater of the value and the argument: a lower bound."),
    entry("keep_below", "x|keep_below(max)", "The lesser of the value and the argument: an upper bound."),
    entry("length", "x|length", "The number of characters in a string."),
    entry("lowercase", "x|lowercase", "Converts a string to lower case."),
    entry("max", "x|max(y…)", "The greatest of its arguments, within one row. (For the greatest of many rows, use `%max`.)"),
    entry("md5", "x|md5", "The MD5 hash of a string."),
    entry("min", "x|min(y…)", "The least of its arguments, within one row. (For the least of many rows, use `%min`.)"),
    entry("minus", "x|minus(y)", "Subtracts the argument from the value."),
    entry("minutes", "x|minutes", "Converts an interval into a number of minutes."),
    entry("mod", "x|mod(y)", "The remainder after dividing the value by the argument."),
    entry("month", "x|month", "The month (1–12) of a date or timestamp."),
    entry("not", "x|not", "Negates a boolean."),
    entry("or", "x|or(y…)", "True when any of the arguments is true."),
    entry("plus", "x|plus(y)", "Adds the argument to the value."),
    entry("pow", "x|pow(y)", "Raises the value to the power of the argument."),
    entry("seconds", "x|seconds", "Converts an interval into a number of seconds."),
    entry("sqrt", "x|sqrt", "The square root of a number."),
    entry("times", "x|times(y)", "Multiplies the value by the argument."),
    entry("trim", "x|trim", "Removes whitespace from the start and end of a string."),
    entry("unit_hash", "x|unit_hash", "Hashes a value to a number between 0 and 1, e.g. for stable sampling."),
    entry("uppercase", "x|uppercase", "Converts a string to upper case."),
    entry("xor", "x|xor(y…)", "True when an odd number of the arguments are true."),
    entry("year", "x|year", "The year of a date or timestamp."),
    entry("year_month", "x|year_month", "The year and month of a date or timestamp, as a `YYYY-MM` string."),
    entry("years", "x|years", "Converts an interval into a number of years."),
];

/// The built-in aggregate functions, applied with `%`.
#[rustfmt::skip]
const AGGREGATE_FUNCTIONS: [Entry; 12] = [
    entry("all_true", "x%all_true", "True when every value is true."),
    entry("any_true", "x%any_true", "True when at least one value is true."),
    entry("avg", "x%avg", "The average of the values."),
    entry("count", "x%count", "The number of values that aren't null. Alone, `%count` counts rows."),
    entry("distinct", "x%distinct", "The number of distinct values."),
    entry("list", "x%list(\\\\sort…)", "The distinct values, as an array. Sort them with `\\\\expr` or `\\s` entries."),
    entry("list_all", "x%list_all(\\\\sort…)", "All of the values, including duplicates, as an array."),
    entry("max", "x%max", "The largest value."),
    entry("min", "x%min", "The smallest value."),
    entry("product", "x%product", "The values multiplied together."),
    entry("sum", "x%sum", "The values added together."),
    entry("top", "x%top(\\\\sort…)", "The distinct values that tie for first place under the sorting entries, as an array."),
];

/// The window functions, applied after a `%%( … )` window definition.
#[rustfmt::skip]
const WINDOW_FUNCTIONS: [Entry; 16] = [
    entry("avg", "%%(…)%avg(x)", "A running average of the values within the window."),
    entry("count", "%%(…)%count(x?)", "A running count of the rows (or of a column's non-null values) within the window."),
    entry("cume_dist", "%%(…)%cume_dist", "The cumulative distribution of the row within its partition."),
    entry("dense_rank", "%%(…)%dense_rank", "The row's rank within its partition, without gaps after ties."),
    entry("first_value", "%%(…)%first_value(x)", "The value from the first row of the window frame."),
    entry("lag", "%%(…)%lag(x offset? default?)", "The value from a row before the current one (by default, the previous row)."),
    entry("last_value", "%%(…)%last_value(x)", "The value from the last row of the window frame."),
    entry("lead", "%%(…)%lead(x offset? default?)", "The value from a row after the current one (by default, the next row)."),
    entry("max", "%%(…)%max(x)", "The largest value within the window."),
    entry("min", "%%(…)%min(x)", "The smallest value within the window."),
    entry("nth_value", "%%(…)%nth_value(x n)", "The value from the nth row of the window frame."),
    entry("ntile", "%%(…)%ntile(n)", "Divides the partition into n buckets and gives the row's bucket number."),
    entry("percent_rank", "%%(…)%percent_rank", "The row's relative rank within its partition, between 0 and 1."),
    entry("rank", "%%(…)%rank", "The row's rank within its partition, with gaps after ties."),
    entry("row_number", "%%(…)%row_number", "The row's position within its partition, starting at 1."),
    entry("sum", "%%(…)%sum(x)", "A running total of the values within the window."),
];

/// The built-in constants, written with `@`.
#[rustfmt::skip]
const CONSTANTS: [Entry; 5] = [
    entry("now", "@now", "The current date and time."),
    entry("infinity", "@infinity", "Positive infinity."),
    entry("true", "@true", "Boolean true."),
    entry("false", "@false", "Boolean false."),
    entry("null", "@null", "The absence of a value."),
];

fn items(entries: &[Entry], kind: CompletionKind) -> Vec<CompletionItem> {
    entries
        .iter()
        .map(|entry| CompletionItem {
            detail: Some(entry.usage.to_string()),
            info: Some(entry.description.to_string()),
            ..item(entry.name, kind)
        })
        .collect()
}

pub(super) fn scalar_function_items() -> Vec<CompletionItem> {
    items(&SCALAR_FUNCTIONS, CompletionKind::Function)
}

pub(super) fn aggregate_function_items() -> Vec<CompletionItem> {
    items(&AGGREGATE_FUNCTIONS, CompletionKind::AggregateFunction)
}

pub(super) fn window_function_items() -> Vec<CompletionItem> {
    items(&WINDOW_FUNCTIONS, CompletionKind::WindowFunction)
}

pub(super) fn constant_items() -> Vec<CompletionItem> {
    items(&CONSTANTS, CompletionKind::Constant)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use querydown_parser::highlight::BUILTIN_CONSTANTS;

    use super::*;
    use crate::compiler::functions::{
        get_standard_aggregate_functions, get_standard_scalar_functions,
    };
    use crate::compiler::window::window_fn_return_type;

    fn names(entries: &[Entry]) -> BTreeSet<&str> {
        entries.iter().map(|entry| entry.name).collect()
    }

    #[test]
    fn test_scalar_functions_match_the_compiler() {
        let compiler = get_standard_scalar_functions();
        assert_eq!(
            names(&SCALAR_FUNCTIONS),
            compiler.keys().map(String::as_str).collect()
        );
    }

    #[test]
    fn test_aggregate_functions_match_the_compiler() {
        let compiler = get_standard_aggregate_functions();
        assert_eq!(
            names(&AGGREGATE_FUNCTIONS),
            compiler.keys().map(String::as_str).collect()
        );
    }

    #[test]
    fn test_window_functions_are_known_to_the_compiler() {
        for entry in &WINDOW_FUNCTIONS {
            assert!(
                window_fn_return_type(entry.name).is_some(),
                "`{}` isn't a window function",
                entry.name
            );
        }
        assert_eq!(names(&WINDOW_FUNCTIONS).len(), WINDOW_FUNCTIONS.len());
    }

    #[test]
    fn test_constants_match_the_parser() {
        assert_eq!(names(&CONSTANTS), BUILTIN_CONSTANTS.into_iter().collect());
    }
}
