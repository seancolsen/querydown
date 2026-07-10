use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

/// Characters that, when one immediately follows a bare word, mean the word is the start of a larger
/// expression (a path, pipe, aggregate, arithmetic operator, or comparison operator) or part of a
/// longer identifier — rather than a standalone search term. All comparison operators begin with
/// `:`, and the range operator begins with `.`, so the leading character of each suffices. The
/// underscore is included because a bare search word is strictly alphanumeric, so an underscore marks
/// a longer (non-search) identifier.
const OPERAND_CONTINUATION_CHARS: &str = "._|%*/+-:";

/// Like [`OPERAND_CONTINUATION_CHARS`], but also treats the comma "OR" shorthand as a continuation.
/// A condition-set _entry_ defers a comma run to the comma operator (see [`condition_entry`]), so a
/// bare word immediately followed by a comma is not consumed here as a lone search term.
const ENTRY_CONTINUATION_CHARS: &str = "._|%*/+-:,";

/// Parses a boolean condition set (`{ ... }` for `AND`, `[ ... ]` for `OR`). Its entries may be bare
/// **default text search** terms (see [`condition_entry`]).
pub fn condition_set<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, ConditionSet> {
    set_of_entries(condition_entry(expr))
}

/// Parses a condition set used as a comparison **expansion** (e.g. the `[ ... ]` in
/// `title:~[color colour]`). Unlike a boolean condition set, its entries are plain values fed into
/// a comparison rather than default text search terms. How a bare word entry resolves therefore
/// follows the side it appears on: the caller supplies an identifier-mode `expr` for a left-side
/// expansion (bare word is a column reference) or a string-mode `expr` for a right-side expansion
/// (bare word is a string literal). See `comparison_rhs_value` for the rationale.
pub fn expansion_set<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, ConditionSet> {
    set_of_entries(expr)
}

/// Parses a single entry of a boolean condition set: either a bare default text search term or a
/// general expression. The search term is tried first so that a lone bare word becomes a search term
/// rather than a column reference. A bare word immediately followed by a comma is left for the comma
/// "OR" shorthand to handle (see the `or_condition_set` parser), which keeps each of its operands
/// search-aware.
pub fn condition_entry<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    choice((bare_word(ENTRY_CONTINUATION_CHARS).map(Expr::String), expr))
}

/// Parses a bare word used as an operand of the comma "OR" shorthand, returning the raw word. Unlike
/// [`condition_entry`]'s search term, a trailing comma does _not_ disqualify it — the comma is the
/// operand separator. Whether the word becomes a search term or a column reference is decided by the
/// caller, depending on whether it stands alone or is part of a multi-operand `OR` (see
/// `or_condition_set`).
pub fn bare_search_operand<'src>() -> impl Psr<'src, String> {
    bare_word(OPERAND_CONTINUATION_CHARS)
}

/// A bare word — an unquoted, letter-initial, strictly-alphanumeric identifier — that stands alone,
/// i.e. is not immediately followed by any of `continuation_chars`. This mirrors the way a bare word
/// on the right-hand side of a comparison is read as a string literal (see `comparison_rhs_value`);
/// a backtick-quoted identifier is a column reference, not a bare word, so it is not matched here.
fn bare_word<'src>(continuation_chars: &'static str) -> impl Psr<'src, String> {
    let word = any()
        .filter(|c: &char| c.is_ascii_alphabetic())
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric())
                .repeated(),
        )
        .to_slice()
        .map(|s: &str| s.to_string());
    let continuation = any().filter(move |c: &char| continuation_chars.contains(*c));
    // A condition-set brace *immediately* following the word (no space) marks a scoped comparison
    // (`issue{...}` / `issue[...]`, see `scoped_comparison`), so the word is not a lone search term
    // but the head of a larger expression. Unlike the `continuation` check, this deliberately does
    // not allow padding before the brace: a *spaced* brace (`issue {...}`) keeps the word standalone.
    let scoped_comparison_brace = one_of([CONDITION_SET_AND_BRACE_L, CONDITION_SET_OR_BRACE_L]);
    word.then_ignore(pad().then(continuation).not())
        .then_ignore(scoped_comparison_brace.not())
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
