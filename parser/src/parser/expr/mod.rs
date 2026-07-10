mod case;
mod comparison;
mod condition_set;
mod date;
mod duration;
mod has_quantity;
mod number;
mod path;
mod pipe;
mod window;

pub use comparison::operator as comparison_operator;
pub use condition_set::condition_entry;
pub use path::{path, path_to_one};

use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use self::{
    case::case,
    comparison::{comparison, left_comparison_side, right_comparison_side},
    condition_set::condition_set,
    date::date,
    duration::duration,
    has_quantity::has_quantity,
    number::number,
    pipe::pipe,
    window::window,
};

pub fn expr<'src>() -> impl Psr<'src, Expr> {
    // A bit about how this works:
    //
    // - `prec` is short for "precedence".
    //
    // - `prec_atom` is the highest precedence rule. These kinds of expressions can be parsed
    //   unambiguously because they are entirely self-contained, for example, a string. Some of
    //   these expressions also include other expressions, e.g. `parenthetical`, and in that case,
    //   we pass the recursive rule to the sub-parser so that it can parse its own sub-expressions
    //   using the most generalized expression parser which includes rules for all levels of
    //   precedence.
    //
    // - As we move to lower precedence rules, we build them up by composing the higher precedence
    //   rules.
    //
    // - In total, it's a circular system of composition. The lower precedence rules compose the
    //   higher precedence rules, and the highest precedence rule recursively composes the lowest
    //   precedence rule.
    //
    // I took this approach from [Chumsky's example code][1].
    //
    // [1]: https://github.com/zesterer/chumsky/blob/0.9/examples/foo.rs#L33
    //
    // There are actually *two* such circular systems here, built as a pair of mutually recursive
    // parsers:
    //
    // - The **identifier-mode** chain (`id_*`), in which a bare (unquoted) word is a column
    //   reference. This is the default mode and the parser returned to callers.
    //
    // - The **string-mode** chain (`str_*`), in which a bare word — at any depth — is a string
    //   literal instead. This mode applies to the entire right-hand side of a comparison, so that
    //   e.g. `title:~[color colour]` searches for the literal text "color" or "colour" rather than
    //   referencing columns named `color`/`colour`. See `comparison_rhs_value` for the rationale.
    //
    // The two chains are identical in structure and differ only in how a bare word resolves at the
    // atom level. They meet at the `comparison` rule, which is shared by both: a comparison's left
    // side is always parsed in identifier mode and its right side always in string mode, regardless
    // of which mode the comparison itself appears in. That shared rule is what makes the two chains
    // mutually recursive, so each is `declare`d up front and `define`d once both are built.

    // Each precedence level is `.boxed()` to type-erase it. Chumsky combinator types grow
    // multiplicatively as they nest, and this parser nests deeply; without these boxing
    // "firewalls" the fully monomorphized type — and rustc's memory use compiling it — balloons
    // far enough to OOM the build. Boxing is semantically transparent and also speeds up compiles.
    let mut id = Recursive::declare();
    let mut string_mode = Recursive::declare();

    // --- Identifier-mode chain: a bare word is a column reference. ---
    let id_atom = choice((
        // `duration` is tried before `number` because both begin with a digit. A duration
        // requires a trailing unit (e.g. `6m`), so a unit-less number like `6` falls through.
        duration().map(Expr::Duration),
        number().map(Expr::Number),
        date().map(Expr::Date),
        string().map(Expr::String),
        // `function_call` is tried before `variable` because both begin with `@`; the `@@`
        // prefix of a call would otherwise be misread as the start of a `@`-prefixed variable.
        function_call(id.clone()),
        variable().map(Expr::Variable),
        window(id.clone()),
        path(id.clone()).map(Expr::Path),
        has_quantity(id.clone()).map(Expr::HasQuantity),
        case(id.clone()).map(Expr::Case),
        condition_set(id.clone()).map(Expr::ConditionSet),
        parenthetical(id.clone()),
    ))
    .boxed();
    let id_pipe = pipe(id_atom.clone(), id.clone()).boxed();
    let id_multiplication = multiplication(id_pipe).boxed();
    let id_addition = addition(id_multiplication).boxed();

    // --- String-mode chain: a bare word — at any depth — is a string literal. ---
    //
    // This mirrors the identifier-mode chain above in every respect except its atom: bare words
    // resolve via `comparison_rhs_value` (string literal) instead of `path` (column reference), and
    // every sub-parser recurses back into this same string-mode chain so the behavior holds at any
    // depth. `case` and `window` are omitted because neither is meaningful as a comparison value.
    let str_atom = choice((
        // See the ordering note on `id_atom` above: `duration` before `number`.
        duration().map(Expr::Duration),
        number().map(Expr::Number),
        date().map(Expr::Date),
        string().map(Expr::String),
        // See the ordering note on `id_atom` above: `function_call` before `variable`.
        function_call(string_mode.clone()),
        variable().map(Expr::Variable),
        comparison_rhs_value(string_mode.clone()),
        has_quantity(string_mode.clone()).map(Expr::HasQuantity),
        condition_set(string_mode.clone()).map(Expr::ConditionSet),
        parenthetical(string_mode.clone()),
    ))
    .boxed();
    let str_pipe = pipe(str_atom.clone(), string_mode.clone()).boxed();
    let str_multiplication = multiplication(str_pipe).boxed();
    let str_addition = addition(str_multiplication).boxed();

    // The shared comparison rule, drawing its left side from the identifier-mode chain and its right
    // side from the string-mode chain. This is the single point at which the two chains meet.
    let comparison_expr = comparison(
        left_comparison_side(id.clone(), id_atom, id_addition.clone()),
        right_comparison_side(string_mode.clone(), str_atom, str_addition.clone()),
    )
    .map(|c| Expr::Comparison(Box::new(c)))
    .boxed();

    id.define(or_condition_set(negation(comparison_expr.clone().or(id_addition)).boxed()).boxed());
    string_mode
        .define(or_string_shorthand(negation(comparison_expr.or(str_addition)).boxed()).boxed());

    id
}

/// Negates an expression with a `!` prefix, producing [`Expr::Not`]. This binds more loosely than
/// comparison (so `!foo:2` negates the whole comparison) but more tightly than the comma shorthand.
/// The prefix may be repeated, e.g. `!!foo`. When no `!` is present, the expression passes through
/// unchanged, so this rule is transparent.
fn negation<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    just(NEGATE)
        .then_ignore(pad())
        .repeated()
        .foldr(e, |_, expr| Expr::Not(Box::new(expr)))
}

/// One operand of the comma "OR" shorthand: either a bare word or a general expression. A bare word
/// is held as a string so it can be resolved differently depending on whether it stands alone (a
/// column reference) or is part of a multi-operand `OR` (a default text search term).
enum OrOperand {
    Bare(String),
    Expr(Expr),
}

impl OrOperand {
    /// How a bare word resolves when it is the _sole_ operand (no comma): as a column reference,
    /// exactly as if it had been parsed as a path. This keeps a lone bare word — e.g. a result
    /// column `$foo` or a sort expression `\\foo` — a column reference rather than a search term.
    fn into_lone_expr(self) -> Expr {
        match self {
            OrOperand::Bare(word) => Expr::Path(vec![PathPart::Column(word)]),
            OrOperand::Expr(expr) => expr,
        }
    }

    /// How a bare word resolves when it is _one of several_ comma-separated operands: as a default
    /// text search term (an `Expr::String`), the same as a bare word standing alone in a boolean
    /// condition.
    fn into_or_entry(self) -> Expr {
        match self {
            OrOperand::Bare(word) => Expr::String(word),
            OrOperand::Expr(expr) => expr,
        }
    }
}

/// Joins comma-separated expressions into an "OR" condition set, e.g. `foo:1,bar:2`. This is the
/// lowest-precedence operator in the language. A single expression with no trailing comma is passed
/// through unchanged, so this rule is transparent when no comma is present.
///
/// Each operand is boolean, so a bare word operand of a multi-operand `OR` is a default text search
/// term (e.g. `foo,bar` searches for "foo" or "bar"). A bare word that turns out to stand alone
/// remains a column reference, so this rule is transparent for non-condition uses like a result
/// column `$foo`.
fn or_condition_set<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let operand = choice((
        condition_set::bare_search_operand().map(OrOperand::Bare),
        e.map(OrOperand::Expr),
    ));
    operand
        .clone()
        .then(
            just(CONDITION_SET_OR_SHORTHAND)
                .padded_by(pad())
                .ignore_then(operand)
                .repeated()
                .collect::<Vec<OrOperand>>(),
        )
        .map(|(first, rest)| {
            if rest.is_empty() {
                first.into_lone_expr()
            } else {
                let entries = std::iter::once(first)
                    .chain(rest)
                    .map(OrOperand::into_or_entry)
                    .collect();
                Expr::ConditionSet(ConditionSet::via_or(entries))
            }
        })
}

/// The string-mode counterpart to [`or_condition_set`], used on the right-hand side of a comparison.
/// It joins comma-separated expressions into an "OR" condition set in the same way, but needs none of
/// the bare-word special-casing: in string mode a bare word is already a string literal at the atom
/// level (see `comparison_rhs_value`), so a lone operand and a multi-operand entry resolve
/// identically. A single expression with no trailing comma passes through unchanged.
fn or_string_shorthand<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    e.clone()
        .then(
            just(CONDITION_SET_OR_SHORTHAND)
                .padded_by(pad())
                .ignore_then(e)
                .repeated()
                .collect::<Vec<Expr>>(),
        )
        .map(|(first, rest)| {
            if rest.is_empty() {
                first
            } else {
                let entries = std::iter::once(first).chain(rest).collect();
                Expr::ConditionSet(ConditionSet::via_or(entries))
            }
        })
}

fn operator<'src>(
    c: char,
    expr_enum_constructor: fn(Box<Expr>, Box<Expr>) -> Expr,
) -> impl Psr<'src, fn(Box<Expr>, Box<Expr>) -> Expr> {
    just(c).padded_by(pad()).to(expr_enum_constructor)
}

fn variable<'src>() -> impl Psr<'src, String> {
    just(CONST_SIGIL).ignore_then(ident().map(|s: &str| s.to_string()))
}

/// Parses a direct function call, written with the `@@` sigil, e.g. `@@max(due_date|age|days 0)`.
/// This applies a (built-in or user-defined) scalar function without using a pipe. The arguments are
/// space-separated (no commas) and enclosed in parentheses; the parentheses are required even for a
/// call with no arguments.
fn function_call<'src>(args_expr: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let args = args_expr
        .padded_by(pad())
        .repeated()
        .collect::<Vec<Expr>>()
        .delimited_by(
            just(COMPOSITION_ARGUMENT_BRACE_L),
            just(COMPOSITION_ARGUMENT_BRACE_R),
        );
    exactly(FUNCTION_SIGIL)
        .ignore_then(ident().map(|s: &str| s.to_string()))
        .then(args)
        .map(|(name, args)| {
            Expr::Call(Call {
                name,
                dimension: FunctionDimension::Scalar,
                syntax: CallSyntax::Standalone,
                args,
                order_by: vec![],
            })
        })
}

fn string<'src>() -> impl Psr<'src, String> {
    quoted(STRING_QUOTE_SINGLE).or(quoted(STRING_QUOTE_DOUBLE))
}

/// The bare-word atom of the string-mode chain: an unquoted word here is interpreted as a string
/// literal rather than a column reference. This matches the behavior users expect from tools like
/// GitHub, where e.g. `description:title` searches for the literal text "title". To refer to a column
/// on the right-hand side, the identifier can either be quoted with backticks (e.g. `` `title` ``) or
/// written as a multi-part path (e.g. `foo.bar`).
///
/// Because the whole right-hand side of a comparison is parsed in string mode, this applies to bare
/// words at *any* depth there — inside expansions (`title:~[color colour]`), pipe arguments
/// (`title:foo|concat(bar)`), parentheses, ranges, and so on — not just to a bare word standing
/// alone. Everywhere outside the right-hand side of a comparison, bare words continue to be parsed as
/// column references.
///
/// A bare word is treated as a string only when it stands alone. If it is the head of a longer path
/// (e.g. `foo.bar`), the whole path is parsed as a column reference instead.
fn comparison_rhs_value<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let bare_string = ident()
        .then_ignore(pad().then(just(PATH_SEPARATOR)).not())
        .map(|s: &str| Expr::String(s.to_string()));
    choice((bare_string, path(expr).map(Expr::Path)))
}

fn parenthetical<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    e.padded_by(pad())
        .delimited_by(just(EXPR_PAREN_L), just(EXPR_PAREN_R))
}

fn multiplication<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let op = choice((
        operator(EXPR_TIMES, Expr::Product),
        operator(EXPR_DIVIDE, Expr::Quotient),
    ));
    e.clone().foldl(op.then(e).repeated(), |lhs, (f, rhs)| {
        f(Box::new(lhs), Box::new(rhs))
    })
}

fn addition<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let op = choice((
        operator(EXPR_PLUS, Expr::Sum),
        operator(EXPR_MINUS, Expr::Difference),
    ));
    e.clone().foldl(op.then(e).repeated(), |lhs, (f, rhs)| {
        f(Box::new(lhs), Box::new(rhs))
    })
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use chumsky::prelude::*;

    use super::expr;

    #[test]
    fn test_parse_expr() {
        let p = |s: &str| {
            expr()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };

        assert_eq!(p("8"), Ok(Expr::Number("8".to_string())));
        assert_eq!(
            p("@2000-01-01"),
            Ok(Expr::Date(Date {
                year: 2000,
                day: 1,
                month: 1
            }))
        );
        assert_eq!(
            p("1y"),
            Ok(Expr::Duration(Duration {
                years: 1.0,
                ..Default::default()
            }))
        );
        // A digit-initial token with no unit is a plain number, not a duration.
        assert_eq!(p("6"), Ok(Expr::Number("6".to_string())));
        assert_eq!(p("'foo'"), Ok(Expr::String("foo".to_string())));
        assert_eq!(p("\"foo\""), Ok(Expr::String("foo".to_string())));
        assert_eq!(p("@foo"), Ok(Expr::Variable("foo".to_string())));
        assert_eq!(p("@null"), Ok(Expr::Variable("null".to_string())));
        assert_eq!(
            p("foo"),
            Ok(Expr::Path(vec![PathPart::Column("foo".to_string())]))
        );
        assert_eq!(
            p("++#foo"),
            Ok(Expr::HasQuantity(HasQuantity {
                quantity: Quantity::AtLeastOne,
                path_parts: vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    condition_set: ConditionSet::default(),
                    linking_column: None
                })]
            }))
        );
        assert_eq!(
            p("++#foo{a:2}"),
            Ok(Expr::HasQuantity(HasQuantity {
                quantity: Quantity::AtLeastOne,
                path_parts: vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    condition_set: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![Expr::Comparison(Box::new(Comparison {
                            left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                                "a".to_string()
                            )])),
                            operator: Operator::Match,
                            right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                        }))]
                    },
                    linking_column: None
                })]
            }))
        );
        // Within a boolean condition set, a bare word is a default text search term (an
        // `Expr::String`), not a column reference. A backtick-quoted identifier would instead be a
        // column reference.
        assert_eq!(
            p("[a b]"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![Expr::String("a".to_string()), Expr::String("b".to_string()),]
            }))
        );
        assert_eq!(
            p("{a b}"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::And,
                entries: vec![Expr::String("a".to_string()), Expr::String("b".to_string()),]
            }))
        );

        // The comma is shorthand for an "OR" condition set. Each operand is boolean, so a bare word
        // operand is a default text search term (an `Expr::String`), not a column reference.
        assert_eq!(
            p("a,b"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![Expr::String("a".to_string()), Expr::String("b".to_string()),]
            }))
        );

        // Whitespace is allowed around the comma, and it joins three or more expressions.
        assert_eq!(
            p("a , b ,c"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::String("a".to_string()),
                    Expr::String("b".to_string()),
                    Expr::String("c".to_string()),
                ]
            }))
        );

        // The comma has lower precedence than comparison, so each comparison becomes its own entry.
        assert_eq!(
            p("foo:1,bar:2"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "foo".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("1".to_string())),
                    })),
                    Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "bar".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                    })),
                ]
            }))
        );

        // The comma also has lower precedence than addition.
        assert_eq!(
            p("1+2,3"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Sum(
                        Box::new(Expr::Number("1".to_string())),
                        Box::new(Expr::Number("2".to_string()))
                    ),
                    Expr::Number("3".to_string()),
                ]
            }))
        );

        // A trailing comma is not allowed.
        assert!(p("a,").is_err());

        assert_eq!(
            p("5*7"),
            Ok(Expr::Product(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        assert_eq!(
            p("@a/@b"),
            Ok(Expr::Quotient(
                Box::new(Expr::Variable("a".to_string())),
                Box::new(Expr::Variable("b".to_string()))
            ))
        );

        assert_eq!(
            p("5+7"),
            Ok(Expr::Sum(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        assert_eq!(
            p("@a-@b"),
            Ok(Expr::Difference(
                Box::new(Expr::Variable("a".to_string())),
                Box::new(Expr::Variable("b".to_string()))
            ))
        );

        assert_eq!(
            p("5 - 7"),
            Ok(Expr::Difference(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        assert_eq!(
            p("5 -7"),
            Ok(Expr::Difference(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        // This is two expressions, not one.
        assert!(p("5 (-7)").is_err());

        assert_eq!(
            p("5*7+3"),
            Ok(Expr::Sum(
                Box::new(Expr::Product(
                    Box::new(Expr::Number("5".to_string())),
                    Box::new(Expr::Number("7".to_string()))
                )),
                Box::new(Expr::Number("3".to_string()))
            ))
        );

        assert_eq!(
            p("3+5*7"),
            Ok(Expr::Sum(
                Box::new(Expr::Number("3".to_string())),
                Box::new(Expr::Product(
                    Box::new(Expr::Number("5".to_string())),
                    Box::new(Expr::Number("7".to_string()))
                ))
            ))
        );

        assert_eq!(
            p("( 3 + 5 )"),
            Ok(Expr::Sum(
                Box::new(Expr::Number("3".to_string())),
                Box::new(Expr::Number("5".to_string()))
            ))
        );

        assert_eq!(
            p("(3+5)*7"),
            Ok(Expr::Product(
                Box::new(Expr::Sum(
                    Box::new(Expr::Number("3".to_string())),
                    Box::new(Expr::Number("5".to_string()))
                )),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        // Comparisons don't fold
        assert!(p("1:2:3").is_err());

        // Nested comparisons can be done via parentheses
        assert_eq!(
            p("(1:2):3"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Comparison(Box::new(Comparison {
                    left: ComparisonSide::Expr(Expr::Number("1".to_string())),
                    operator: Operator::Match,
                    right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                }))),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Number("3".to_string())),
            })))
        );

        assert_eq!(
            p("x:@a..@b"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("x".to_string())])),
                operator: Operator::Match,
                right: ComparisonSide::Range(Range {
                    lower: RangeBound {
                        expr: Expr::Variable("a".to_string()),
                        exclusivity: Exclusivity::Inclusive,
                    },
                    upper: RangeBound {
                        expr: Expr::Variable("b".to_string()),
                        exclusivity: Exclusivity::Inclusive,
                    },
                }),
            })))
        );

        assert_eq!(
            p("x:@a<..<@b"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("x".to_string())])),
                operator: Operator::Match,
                right: ComparisonSide::Range(Range {
                    lower: RangeBound {
                        expr: Expr::Variable("a".to_string()),
                        exclusivity: Exclusivity::Exclusive,
                    },
                    upper: RangeBound {
                        expr: Expr::Variable("b".to_string()),
                        exclusivity: Exclusivity::Exclusive,
                    },
                }),
            })))
        );

        assert_eq!(
            p("1|a|b(2)|c(3 4)"),
            Ok(Expr::Call(Call {
                name: "c".to_string(),
                dimension: FunctionDimension::Scalar,
                syntax: CallSyntax::Piped,
                args: vec![
                    Expr::Call(Call {
                        name: "b".to_string(),
                        dimension: FunctionDimension::Scalar,
                        syntax: CallSyntax::Piped,
                        args: vec![
                            Expr::Call(Call {
                                name: "a".to_string(),
                                dimension: FunctionDimension::Scalar,
                                syntax: CallSyntax::Piped,
                                args: vec![Expr::Number("1".to_string())],
                                order_by: vec![],
                            }),
                            Expr::Number("2".to_string())
                        ],
                        order_by: vec![],
                    }),
                    Expr::Number("3".to_string()),
                    Expr::Number("4".to_string()),
                ],
                order_by: vec![],
            }))
        );

        assert_eq!(
            p("[a b]: 2 + foo * @bar | baz"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expansion(ConditionSet {
                    entries: vec![
                        Expr::Path(vec![PathPart::Column("a".to_string())]),
                        Expr::Path(vec![PathPart::Column("b".to_string())]),
                    ],
                    conjunction: Conjunction::Or,
                }),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Sum(
                    Box::new(Expr::Number("2".to_string())),
                    Box::new(Expr::Product(
                        // A bare word on the right-hand side of a comparison is a string literal.
                        Box::new(Expr::String("foo".to_string())),
                        Box::new(Expr::Call(Call {
                            name: "baz".to_string(),
                            dimension: FunctionDimension::Scalar,
                            syntax: CallSyntax::Piped,
                            args: vec![Expr::Variable("bar".to_string())],
                            order_by: vec![],
                        })),
                    )),
                )),
            })))
        );

        // A standalone aggregate `%count` has no `arg0`, so its `args` vector is empty. This is how
        // `count(*)` is written.
        assert_eq!(
            p("%count"),
            Ok(Expr::Call(Call {
                name: "count".to_string(),
                dimension: FunctionDimension::Aggregate,
                syntax: CallSyntax::Piped,
                args: vec![],
                order_by: vec![],
            }))
        );

        // A piped aggregate `created_at%max` carries the piped-in expression as its single argument.
        assert_eq!(
            p("created_at%max"),
            Ok(Expr::Call(Call {
                name: "max".to_string(),
                dimension: FunctionDimension::Aggregate,
                syntax: CallSyntax::Piped,
                args: vec![Expr::Path(vec![PathPart::Column("created_at".to_string())])],
                order_by: vec![],
            }))
        );

        // A `!` prefix negates a bare expression.
        assert_eq!(
            p("!foo"),
            Ok(Expr::Not(Box::new(Expr::Path(vec![PathPart::Column(
                "foo".to_string()
            )]))))
        );

        // Negation binds more loosely than comparison, so `!foo:2` negates the whole comparison.
        assert_eq!(
            p("!foo:2"),
            Ok(Expr::Not(Box::new(Expr::Comparison(Box::new(
                Comparison {
                    left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                        "foo".to_string()
                    )])),
                    operator: Operator::Match,
                    right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                }
            )))))
        );

        // The `!` prefix can be repeated.
        assert_eq!(
            p("!!foo"),
            Ok(Expr::Not(Box::new(Expr::Not(Box::new(Expr::Path(vec![
                PathPart::Column("foo".to_string())
            ]))))))
        );

        // Negation binds more tightly than the comma shorthand, so only the first entry is negated.
        assert_eq!(
            p("!foo:1,bar:2"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Not(Box::new(Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "foo".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("1".to_string())),
                    })))),
                    Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "bar".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                    })),
                ]
            }))
        );
    }

    #[test]
    fn test_parse_function_call() {
        let p = |s: &str| {
            expr()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };

        // A direct function call with `@@`: scalar dimension, standalone syntax, space-separated
        // arguments (no commas).
        assert_eq!(
            p("@@max(foo 0)"),
            Ok(Expr::Call(Call {
                name: "max".to_string(),
                dimension: FunctionDimension::Scalar,
                syntax: CallSyntax::Standalone,
                args: vec![
                    Expr::Path(vec![PathPart::Column("foo".to_string())]),
                    Expr::Number("0".to_string()),
                ],
                order_by: vec![],
            }))
        );

        // The parentheses are required, but may be empty for a no-argument call.
        assert_eq!(
            p("@@now()"),
            Ok(Expr::Call(Call {
                name: "now".to_string(),
                dimension: FunctionDimension::Scalar,
                syntax: CallSyntax::Standalone,
                args: vec![],
                order_by: vec![],
            }))
        );

        // Arguments may themselves be pipes.
        assert_eq!(
            p("@@max(foo|bar 0)"),
            Ok(Expr::Call(Call {
                name: "max".to_string(),
                dimension: FunctionDimension::Scalar,
                syntax: CallSyntax::Standalone,
                args: vec![
                    Expr::Call(Call {
                        name: "bar".to_string(),
                        dimension: FunctionDimension::Scalar,
                        syntax: CallSyntax::Piped,
                        args: vec![Expr::Path(vec![PathPart::Column("foo".to_string())])],
                        order_by: vec![],
                    }),
                    Expr::Number("0".to_string()),
                ],
                order_by: vec![],
            }))
        );
    }

    #[test]
    fn test_parse_anonymous_function() {
        let p = |s: &str| {
            expr()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };

        // An anonymous function applied via a pipe. The piped-in value becomes the sole argument,
        // and the body references the parameter.
        assert_eq!(
            p("foo|(@x => @x * 2)"),
            Ok(Expr::AnonymousFunctionCall(Box::new(
                AnonymousFunctionCall {
                    params: vec!["x".to_string()],
                    body: FunctionBody {
                        assignments: vec![],
                        expr: Expr::Product(
                            Box::new(Expr::Variable("x".to_string())),
                            Box::new(Expr::Number("2".to_string())),
                        ),
                    },
                    args: vec![Expr::Path(vec![PathPart::Column("foo".to_string())])],
                }
            )))
        );

        // An anonymous function may take multiple parameters and contain a local assignment. The
        // piped-in value is the first argument; parenthesized values supply the rest.
        assert_eq!(
            p("foo|(@a @b => @s = @a + @b @s * 2)(3)"),
            Ok(Expr::AnonymousFunctionCall(Box::new(
                AnonymousFunctionCall {
                    params: vec!["a".to_string(), "b".to_string()],
                    body: FunctionBody {
                        assignments: vec![Assignment {
                            name: "s".to_string(),
                            expr: Expr::Sum(
                                Box::new(Expr::Variable("a".to_string())),
                                Box::new(Expr::Variable("b".to_string())),
                            ),
                        }],
                        expr: Expr::Product(
                            Box::new(Expr::Variable("s".to_string())),
                            Box::new(Expr::Number("2".to_string())),
                        ),
                    },
                    args: vec![
                        Expr::Path(vec![PathPart::Column("foo".to_string())]),
                        Expr::Number("3".to_string()),
                    ],
                }
            )))
        );
    }

    #[test]
    fn test_bare_text_on_comparison_rhs_is_a_string() {
        let p = |s: &str| {
            expr()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };

        // A bare word on the left-hand side of a comparison is a column reference, while a bare
        // word on the right-hand side is a string literal.
        assert_eq!(
            p("description:title"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "description".to_string()
                )])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::String("title".to_string())),
            })))
        );

        // A bare word on the right-hand side behaves identically to a quoted string.
        assert_eq!(p("description:title"), p("description:\"title\""));

        // Backtick-quoting the right-hand side designates it as a column reference instead.
        assert_eq!(
            p("description:`title`"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "description".to_string()
                )])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "title".to_string()
                )])),
            })))
        );

        // A multi-part path on the right-hand side remains a column reference, since it is not a
        // bare, standalone word.
        assert_eq!(
            p("description:foo.bar"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "description".to_string()
                )])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Path(vec![
                    PathPart::Column("foo".to_string()),
                    PathPart::Column("bar".to_string()),
                ])),
            })))
        );

        // The rule applies to other comparison operators too, not just match.
        assert_eq!(
            p("status:=open"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "status".to_string()
                )])),
                operator: Operator::Eq,
                right: ComparisonSide::Expr(Expr::String("open".to_string())),
            })))
        );
    }

    #[test]
    fn test_bare_text_on_comparison_rhs_is_a_string_at_any_depth() {
        let p = |s: &str| {
            expr()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };

        // Bare words inside an expansion set on the right-hand side are string literals, so the
        // unquoted form is equivalent to quoting each term.
        assert_eq!(
            p("title:~[color colour]"),
            p("title:~[\"color\" \"colour\"]")
        );
        assert_eq!(
            p("title:~[color colour]"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("title".to_string())])),
                operator: Operator::RegexMatch,
                right: ComparisonSide::Expansion(ConditionSet {
                    conjunction: Conjunction::Or,
                    entries: vec![
                        Expr::String("color".to_string()),
                        Expr::String("colour".to_string()),
                    ],
                }),
            })))
        );

        // Bare words within a pipe on the right-hand side — both the piped-in value and the extra
        // arguments — are string literals.
        assert_eq!(
            p("title:foo|concat(bar)"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("title".to_string())])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Call(Call {
                    name: "concat".to_string(),
                    dimension: FunctionDimension::Scalar,
                    syntax: CallSyntax::Piped,
                    args: vec![
                        Expr::String("foo".to_string()),
                        Expr::String("bar".to_string()),
                    ],
                    order_by: vec![],
                })),
            })))
        );

        // The same pipe *outside* a comparison treats bare words as column references, illustrating
        // that the string-mode behavior is confined to the right-hand side.
        assert_eq!(
            p("status|concat(title)"),
            Ok(Expr::Call(Call {
                name: "concat".to_string(),
                dimension: FunctionDimension::Scalar,
                syntax: CallSyntax::Piped,
                args: vec![
                    Expr::Path(vec![PathPart::Column("status".to_string())]),
                    Expr::Path(vec![PathPart::Column("title".to_string())]),
                ],
                order_by: vec![],
            }))
        );

        // Bare words inside parentheses on the right-hand side are string literals too.
        assert_eq!(p("title:(foo)"), p("title:foo"),);

        // A nested comparison on the right-hand side resets to identifier mode for *its* left side:
        // the left side of a comparison is always a column reference, even within another
        // comparison's right side, while its own right side remains a string literal.
        assert_eq!(
            p("description:(title:foo)"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "description".to_string()
                )])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Comparison(Box::new(Comparison {
                    left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                        "title".to_string()
                    )])),
                    operator: Operator::Match,
                    right: ComparisonSide::Expr(Expr::String("foo".to_string())),
                }))),
            })))
        );
    }
}
