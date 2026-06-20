use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

/// Characters that, when one of them immediately follows a bare word, mean the word is the start of
/// a larger expression (a path, pipe, aggregate, arithmetic operator, comparison operator, or the
/// comma "OR" shorthand) or part of a longer identifier — rather than a standalone search term. All
/// comparison operators begin with `:`, and both the range and expansion operators begin with `.`,
/// so the leading character of each suffices here. The underscore is included because a bare search
/// word is strictly alphanumeric, so an underscore marks a longer (non-search) identifier.
const SEARCH_TERM_CONTINUATION_CHARS: &str = "._|%*/+-:,";

/// Parses a boolean condition set (`{ ... }` for `AND`, `[ ... ]` for `OR`). Its entries may be bare
/// **default text search** terms (see [`condition_entry`]).
pub fn condition_set<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, ConditionSet> {
    set_of_entries(condition_entry(expr))
}

/// Parses a condition set used as a comparison **expansion** (e.g. the `[ ... ]` in
/// `title:~..["color" "colour"]`). Unlike a boolean condition set, its entries are plain values fed
/// into a comparison, so a bare word is a column reference rather than a default text search term.
pub fn expansion_set<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, ConditionSet> {
    set_of_entries(expr)
}

/// Parses a single entry of a boolean condition set: either a bare default text search term or a
/// general expression. The search term is tried first so that a lone bare word becomes a search term
/// rather than a column reference.
pub fn condition_entry<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    choice((bare_search_term(), expr))
}

/// A standalone bare word used as a default text search term within a boolean condition. This mirrors
/// the way a bare word on the right-hand side of a comparison is read as a string literal (see
/// `comparison_rhs_value`): an unquoted, letter-initial, strictly-alphanumeric word becomes an
/// [`Expr::String`]. The trailing lookahead ensures the word truly stands alone — that it is not the
/// left-hand side of a comparison, nor the head of a path, pipe, or arithmetic expression — so that
/// e.g. `title:open` still parses `title` as a column reference. A backtick-quoted identifier is a
/// column reference, not a bare word, so it is deliberately not matched here.
fn bare_search_term<'src>() -> impl Psr<'src, Expr> {
    let word = any()
        .filter(|c: &char| c.is_ascii_alphabetic())
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric())
                .repeated(),
        )
        .to_slice()
        .map(|s: &str| Expr::String(s.to_string()));
    let continuation = any().filter(|c: &char| SEARCH_TERM_CONTINUATION_CHARS.contains(*c));
    word.then_ignore(pad().then(continuation).not())
}

fn set_of_entries<'src>(entry: impl Psr<'src, Expr>) -> impl Psr<'src, ConditionSet> {
    choice((
        specific_condition_set(Conjunction::And, entry.clone()),
        specific_condition_set(Conjunction::Or, entry),
    ))
}

fn specific_condition_set<'src>(
    conjunction: Conjunction,
    entry: impl Psr<'src, Expr>,
) -> impl Psr<'src, ConditionSet> {
    let (brace_l, brace_r) = match conjunction {
        Conjunction::And => (CONDITION_SET_AND_BRACE_L, CONDITION_SET_AND_BRACE_R),
        Conjunction::Or => (CONDITION_SET_OR_BRACE_L, CONDITION_SET_OR_BRACE_R),
    };
    entry
        .padded_by(pad())
        .repeated()
        .collect::<Vec<Expr>>()
        .delimited_by(just(brace_l), just(brace_r))
        .map(move |entries| ConditionSet {
            conjunction,
            entries,
        })
}
