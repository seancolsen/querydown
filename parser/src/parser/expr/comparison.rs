use chumsky::{
    input::{Emitter, MapExtra},
    prelude::*,
};

use crate::ast::*;
use crate::highlight::TokenKind;
use crate::labels;
use crate::parser::expr::condition_set::expansion_set;
use crate::parser::utils::*;
use crate::tokens::*;

/// Combines a left side, an operator, and a right side into a [`Comparison`]. The two sides are
/// passed in pre-built (see [`left_comparison_side`] and [`right_comparison_side`]) because they draw
/// from different parsing modes: a bare word on the left is a column reference, while a bare word on
/// the right — at any depth — is a string literal. See `comparison_rhs_value` for the rationale.
pub fn comparison<'src>(
    left: impl Psr<'src, ComparisonSide>,
    right: impl Psr<'src, ComparisonSide>,
) -> impl Psr<'src, Comparison> {
    left.then(
        operator()
            .labelled(labels::COMPARISON_OPERATOR)
            .padded_by(pad()),
    )
    .then(right)
    .map(|((left, operator), right)| Comparison {
        left,
        operator,
        right,
    })
}

/// The left side of a comparison. A condition set here (`{ ... }` or `[ ... ]`) is automatically an
/// **expansion**: the comparison is distributed across the set's entries, joined by the set's
/// conjunction. Because this is the left side, bare words within `expansion_set`, `range_atom`, and
/// `expr` are column references; the caller supplies the column-reference ("identifier-mode") parsers.
pub fn left_comparison_side<'src>(
    expansion_set_expr: impl Psr<'src, Expr>,
    range_atom: impl Psr<'src, Expr>,
    expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, ComparisonSide> {
    choice((
        expansion_set(expansion_set_expr).map(ComparisonSide::Expansion),
        range(range_atom).map(ComparisonSide::Range),
        expr.map(ComparisonSide::Expr),
    ))
}

/// The right side of a comparison. A condition set here (`{ ... }` or `[ ... ]`) is automatically an
/// **expansion**, just as on the left. Because this is the right side, bare words within
/// `expansion_set`, `range_atom`, and `expr` — at any depth — are string literals; the caller supplies
/// the string-literal ("string-mode") parsers.
pub fn right_comparison_side<'src>(
    expansion_set_expr: impl Psr<'src, Expr>,
    range_atom: impl Psr<'src, Expr>,
    expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, ComparisonSide> {
    choice((
        expansion_set(expansion_set_expr).map(ComparisonSide::Expansion),
        range(range_atom).map(ComparisonSide::Range),
        expr.map(ComparisonSide::Expr),
    ))
}

fn range<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Range> {
    let exclusivity = just(COMPARISON_RANGE_BOUND_EXCLUSIVE)
        .or_not()
        .map(|b| match b {
            Some(_) => Exclusivity::Exclusive,
            None => Exclusivity::Inclusive,
        });

    let lower = expr
        .clone()
        .then_ignore(pad())
        .then(exclusivity)
        .map(|(expr, exclusivity)| RangeBound { expr, exclusivity });

    let upper = exclusivity
        .then_ignore(pad())
        .then(expr.clone())
        .map(|(exclusivity, expr)| RangeBound { expr, exclusivity });

    lower
        .then_ignore(just(COMPARISON_RANGE_BOUND_SEPARATOR).padded_by(pad()))
        .then(upper)
        .map(|(lower, upper)| Range { lower, upper })
}

pub fn operator<'src>() -> impl Psr<'src, Operator> {
    choice((
        flagged_regex_operator(),
        // Three character
        exactly(COMPARE_EQ_CASE_SENSITIVE).to(Operator::Eq),
        exactly(COMPARE_GTE).to(Operator::Gte),
        exactly(COMPARE_LTE).to(Operator::Lte),
        exactly(COMPARE_LIKE).to(Operator::Like),
        // Two character
        exactly(COMPARE_REGEX).to(Operator::RegexMatch(RegexFlags::default())),
        exactly(COMPARE_EQ).to(Operator::EqCi),
        exactly(COMPARE_GT).to(Operator::Gt),
        exactly(COMPARE_LT).to(Operator::Lt),
        // One character
        exactly(COMPARE_MATCH).to(Operator::Match),
    ))
}

/// A regular-expression match operator with flags, e.g. `:\cm~`: a `:`, then a backslash and the
/// flags, then a `~`. See [`RegexFlags`] for the flags. An unknown or repeated flag is reported but
/// otherwise ignored, as is a trailing `~` that makes it look like a flagged `:~~` (LIKE) operator.
fn flagged_regex_operator<'src>() -> impl Psr<'src, Operator> {
    let flags = just(REGEX_FLAGS_PREFIX).ignore_then(
        any()
            .filter(char::is_ascii_alphabetic)
            .repeated()
            .to_slice()
            .validate(regex_flags),
    );
    let tilde = &COMPARE_REGEX[COMPARE_MATCH.len()..];
    let like = just(tilde).validate(|_, extra, emitter: &mut Emitter<Rich<'src, char>>| {
        let message = format!(
            "Regex flags can't be used with the `{COMPARE_LIKE}` (LIKE) operator, only with \
             `{COMPARE_REGEX}`"
        );
        emitter.emit(Rich::custom(extra.span(), message));
    });
    just(COMPARE_MATCH)
        .ignore_then(mark(TokenKind::Flag, flags))
        .then_ignore(just(tilde))
        .then_ignore(like.or_not())
        .map(Operator::RegexMatch)
}

/// Reads the letters of a flagged regex operator into [`RegexFlags`], reporting any that are
/// unknown or repeated, or that there are none.
fn regex_flags<'src>(
    letters: &'src str,
    extra: &mut MapExtra<'src, '_, &'src str, Extra<'src>>,
    emitter: &mut Emitter<Rich<'src, char>>,
) -> RegexFlags {
    let span: SimpleSpan = extra.span();
    if letters.is_empty() {
        // Point at the backslash, which precedes the (empty) flags.
        let message = format!(
            "Expected regex flags after `{REGEX_FLAGS_PREFIX}`. {}",
            regex_flags_hint()
        );
        emitter.emit(Rich::custom((span.start - 1..span.start).into(), message));
    }
    let mut flags = RegexFlags::default();
    for (i, letter) in letters.char_indices() {
        let flag = match letter {
            REGEX_FLAG_CASE_SENSITIVE => &mut flags.case_sensitive,
            REGEX_FLAG_MULTILINE => &mut flags.multiline,
            REGEX_FLAG_DOT_ALL => &mut flags.dot_all,
            _ => {
                let message = if letter == 'i' {
                    format!(
                        "Regex matching is already case-insensitive, so there is no `i` flag. Use \
                         `{REGEX_FLAG_CASE_SENSITIVE}` to make it case-sensitive"
                    )
                } else {
                    format!("Unknown regex flag `{letter}`. {}", regex_flags_hint())
                };
                let start = span.start + i;
                emitter.emit(Rich::custom(
                    (start..start + letter.len_utf8()).into(),
                    message,
                ));
                continue;
            }
        };
        if *flag {
            let start = span.start + i;
            let message = format!("Repeated regex flag `{letter}`");
            emitter.emit(Rich::custom((start..start + 1).into(), message));
        }
        *flag = true;
    }
    flags
}

fn regex_flags_hint() -> String {
    format!(
        "The flags are `{REGEX_FLAG_CASE_SENSITIVE}` (case-sensitive), `{REGEX_FLAG_MULTILINE}` \
         (`^` and `$` match at line breaks), and `{REGEX_FLAG_DOT_ALL}` (`.` matches a newline)"
    )
}
