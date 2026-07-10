//! Shared logic for desugaring `.( )` nesting groups, used by both result column nesting
//! ([`super::column_layout`]) and scoped sorting expressions ([`super::sorting`]). Both features
//! parse a head path followed by a parenthesized group of entries, then apply the head to the
//! leading column reference of each entry's expression — this module implements that shared
//! "find and prefix the leading column reference" step.

use crate::ast::*;

/// Prepends `head` to the leading column reference of `expr` — the position the head would occupy
/// had it been written inline, so that e.g. `$title|upper` nested under `issue` becomes
/// `$issue.title|upper`. The leading reference is found by looking through a `!` prefix, a
/// `++`/`--` quantity prefix, the piped-in value of a function application, the left operand of an
/// arithmetic expression, and the left side of a comparison. An expression with no leading column
/// reference (e.g. a literal or a standalone `%count`) cannot be nested.
pub(crate) fn prefix_leading_path(head: &[PathPart], expr: Expr) -> Result<Expr, String> {
    let prefixed =
        |parts: Vec<PathPart>| -> Vec<PathPart> { head.iter().cloned().chain(parts).collect() };
    match expr {
        Expr::Path(parts) => Ok(Expr::Path(prefixed(parts))),
        Expr::HasQuantity(has_quantity) => Ok(Expr::HasQuantity(HasQuantity {
            quantity: has_quantity.quantity,
            path_parts: prefixed(has_quantity.path_parts),
        })),
        // A piped call's leading reference is its piped-in value (the first argument). A standalone
        // call (`@@fn(...)`) and a leading aggregate (`%count`, which has no arguments) have none.
        Expr::Call(call) if call.syntax == CallSyntax::Piped && !call.args.is_empty() => {
            Ok(Expr::Call(Call {
                args: prefix_first_arg(head, call.args)?,
                ..call
            }))
        }
        // An anonymous function call always carries its piped-in value as its first argument.
        Expr::AnonymousFunctionCall(mut call) if !call.args.is_empty() => {
            call.args = prefix_first_arg(head, call.args)?;
            Ok(Expr::AnonymousFunctionCall(call))
        }
        Expr::Product(a, b) => Ok(Expr::Product(prefix_boxed(head, *a)?, b)),
        Expr::Quotient(a, b) => Ok(Expr::Quotient(prefix_boxed(head, *a)?, b)),
        Expr::Sum(a, b) => Ok(Expr::Sum(prefix_boxed(head, *a)?, b)),
        Expr::Difference(a, b) => Ok(Expr::Difference(prefix_boxed(head, *a)?, b)),
        Expr::Comparison(mut comparison) => {
            let ComparisonSide::Expr(left) = comparison.left else {
                return Err(msg_no_leading_column_reference());
            };
            comparison.left = ComparisonSide::Expr(prefix_leading_path(head, left)?);
            Ok(Expr::Comparison(comparison))
        }
        Expr::Not(inner) => Ok(Expr::Not(prefix_boxed(head, *inner)?)),
        _ => Err(msg_no_leading_column_reference()),
    }
}

fn prefix_boxed(head: &[PathPart], expr: Expr) -> Result<Box<Expr>, String> {
    Ok(Box::new(prefix_leading_path(head, expr)?))
}

/// Applies the head to the first (piped-in) argument of a call, leaving the rest untouched.
fn prefix_first_arg(head: &[PathPart], args: Vec<Expr>) -> Result<Vec<Expr>, String> {
    let mut args = args.into_iter();
    let first = prefix_leading_path(head, args.next().unwrap())?;
    Ok(std::iter::once(first).chain(args).collect())
}

pub(crate) fn msg_no_leading_column_reference() -> String {
    "An entry nested within `.( )` must begin with a column reference \
    so that the path before `.( )` can be applied to it."
        .to_string()
}
