use chumsky::{
    extra,
    input::{Emitter, MapExtra},
    prelude::*,
    primitive::JustCfg,
};

use crate::ast::*;
use crate::labels;
use crate::parser::utils::*;
use crate::tokens::*;

/// How a flagged string is written, as given by its prefix: the context in which its body is
/// parsed.
#[derive(Debug, Clone, Copy, Default)]
struct Form {
    quote: char,
    format: bool,
    escape: bool,
}

/// The parser "extra" for a flagged string's body, which is parsed in the context of its [`Form`].
type BodyExtra<'src> = extra::Full<Rich<'src, char>, State<'src>, Form>;

/// A piece of a flagged string's body.
enum Piece {
    Char(char),
    Expr(Expr),
}

/// Parses a flagged string: `^`, any flags, then text within any of the [`FLAGGED_STRING_QUOTES`],
/// e.g. `^^Don't say "never"^`. The flags are
///
/// - `f`, which interpolates the expressions written within `{ }` (a literal brace is doubled, as
///   `{{` or `}}`), and
/// - `e`, which interprets escape sequences (`\\`, `\n`, `\t`, `\r`, `\u{…}`, and a backslash before
///   the quote character or, with `f`, a brace).
///
/// An interpolated expression is parsed by `expr`, which the caller gives in identifier mode, so a
/// bare word within it is a column reference even on the right-hand side of a comparison. The
/// string ends at the first quote character outside of any interpolation, so an interpolation may
/// contain the quote character, e.g. `^f|{name|upper}|`.
///
/// A flagged string with no interpolations is an [`ExprKind::String`]; one with any is an
/// [`ExprKind::FormatString`].
pub fn flagged_string<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, ExprKind> {
    let flags = just(STRING_FLAGS_PREFIX).ignore_then(
        any()
            .filter(char::is_ascii_alphabetic)
            .repeated()
            .to_slice()
            .validate(check_flags),
    );
    let quote = one_of(FLAGGED_STRING_QUOTES).labelled(labels::FLAGGED_STRING_QUOTE);
    flags
        .then(quote)
        .map(form)
        .ignore_with_ctx(body(expr))
        .map(into_expr_kind)
}

/// Reports any unknown or repeated flag, yielding the flags that are present.
fn check_flags<'src>(
    flags: &'src str,
    extra: &mut MapExtra<'src, '_, &'src str, Extra<'src>>,
    emitter: &mut Emitter<Rich<'src, char>>,
) -> &'src str {
    for (i, flag) in flags.char_indices() {
        let message = if ![STRING_FLAG_FORMAT, STRING_FLAG_ESCAPE].contains(&flag) {
            format!(
                "Unknown string flag `{flag}`. The flags are `{STRING_FLAG_FORMAT}` (format) and \
                 `{STRING_FLAG_ESCAPE}` (escape)"
            )
        } else if flags[..i].contains(flag) {
            format!("Repeated string flag `{flag}`")
        } else {
            continue;
        };
        let start = extra.span().start + i;
        emitter.emit(Rich::custom((start..start + 1).into(), message));
    }
    flags
}

fn form((flags, quote): (&str, char)) -> Form {
    Form {
        quote,
        format: flags.contains(STRING_FLAG_FORMAT),
        escape: flags.contains(STRING_FLAG_ESCAPE),
    }
}

fn quote_cfg(cfg: JustCfg<char>, form: &Form) -> JustCfg<char> {
    cfg.seq(form.quote)
}

fn format_enabled(_: bool, form: &Form) -> bool {
    form.format
}

fn escape_enabled(_: bool, form: &Form) -> bool {
    form.escape
}

/// The body of a flagged string, after its opening quote, through its closing quote.
fn body<'src>(
    expr: impl Psr<'src, Expr>,
) -> impl Parser<'src, &'src str, Vec<FormatStringPart>, BodyExtra<'src>> + Clone {
    let close = just(FLAGGED_STRING_QUOTES[0]).configure(quote_cfg);

    // Characters that don't stand for themselves.
    let special = choice((
        close.ignored(),
        just(STRING_ESCAPE)
            .ignored()
            .contextual()
            .configure(escape_enabled),
        one_of([STRING_INTERPOLATION_L, STRING_INTERPOLATION_R])
            .ignored()
            .contextual()
            .configure(format_enabled),
    ));
    let text = any().and_is(special.not()).map(Piece::Char);

    let unicode_escape = just("\\u{")
        .ignore_then(
            any()
                .filter(char::is_ascii_hexdigit)
                .repeated()
                .at_least(1)
                .at_most(6)
                .to_slice()
                .labelled(labels::HEX_DIGIT),
        )
        .then_ignore(just('}'))
        .validate(code_point);
    let simple_escape = just(STRING_ESCAPE)
        .ignore_then(any())
        .validate(escaped_char);
    let escape = choice((unicode_escape, simple_escape))
        .map(Piece::Char)
        .contextual()
        .configure(escape_enabled);

    let doubled_brace = choice((
        just([STRING_INTERPOLATION_L; 2]).to(STRING_INTERPOLATION_L),
        just([STRING_INTERPOLATION_R; 2]).to(STRING_INTERPOLATION_R),
    ));
    let stray_brace = just(STRING_INTERPOLATION_R).validate(stray_brace);
    let interpolation = expr
        .padded_by(pad())
        .with_ctx(())
        .delimited_by(just(STRING_INTERPOLATION_L), just(STRING_INTERPOLATION_R))
        .map(Piece::Expr);
    let format = choice((
        doubled_brace.map(Piece::Char),
        stray_brace.map(Piece::Char),
        interpolation,
    ))
    .contextual()
    .configure(format_enabled);

    choice((text, escape, format))
        .repeated()
        .collect::<Vec<Piece>>()
        .then_ignore(close)
        .map(into_parts)
}

/// Interprets the character after a backslash (in a string with the escape flag).
fn escaped_char<'src>(
    c: char,
    extra: &mut MapExtra<'src, '_, &'src str, BodyExtra<'src>>,
    emitter: &mut Emitter<Rich<'src, char>>,
) -> char {
    let form = *extra.ctx();
    match c {
        '\\' => '\\',
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        _ if c == form.quote => c,
        STRING_INTERPOLATION_L | STRING_INTERPOLATION_R if form.format => c,
        // A `\u` that the `\u{…}` parser didn't match.
        'u' => {
            emitter.emit(Rich::custom(
                extra.span(),
                "Invalid Unicode escape sequence. Write a code point as `\\u{…}` with one to six \
                 hexadecimal digits, e.g. `\\u{e9}`",
            ));
            c
        }
        _ => {
            let span = extra.span();
            let braces = match form.format {
                true => ", `\\{`, `\\}`",
                false => "",
            };
            emitter.emit(Rich::custom(
                span,
                format!(
                    "Unknown escape sequence `\\{c}`. The escape sequences are `\\\\`, `\\n`, \
                     `\\t`, `\\r`, `\\u{{…}}`{braces}, and `\\{quote}`",
                    quote = form.quote
                ),
            ));
            c
        }
    }
}

/// Interprets the hexadecimal digits of a `\u{…}` escape sequence.
fn code_point<'src>(
    digits: &'src str,
    extra: &mut MapExtra<'src, '_, &'src str, BodyExtra<'src>>,
    emitter: &mut Emitter<Rich<'src, char>>,
) -> char {
    let code = u32::from_str_radix(digits, 16).expect("at most six hex digits");
    char::from_u32(code).unwrap_or_else(|| {
        emitter.emit(Rich::custom(
            extra.span(),
            format!("`\\u{{{digits}}}` is not a valid Unicode character"),
        ));
        char::REPLACEMENT_CHARACTER
    })
}

/// Reports a lone `}` in a formatted string.
fn stray_brace<'src>(
    brace: char,
    extra: &mut MapExtra<'src, '_, &'src str, BodyExtra<'src>>,
    emitter: &mut Emitter<Rich<'src, char>>,
) -> char {
    emitter.emit(Rich::custom(
        extra.span(),
        "Unmatched `}` in a formatted string. Write `}}` for a literal `}`",
    ));
    brace
}

/// Joins consecutive characters into text.
fn into_parts(pieces: Vec<Piece>) -> Vec<FormatStringPart> {
    let mut parts = Vec::new();
    let mut text = String::new();
    for piece in pieces {
        match piece {
            Piece::Char(c) => text.push(c),
            Piece::Expr(expr) => {
                if !text.is_empty() {
                    parts.push(FormatStringPart::Text(std::mem::take(&mut text)));
                }
                parts.push(FormatStringPart::Expr(expr));
            }
        }
    }
    if !text.is_empty() {
        parts.push(FormatStringPart::Text(text));
    }
    parts
}

fn into_expr_kind(mut parts: Vec<FormatStringPart>) -> ExprKind {
    if parts.iter().any(|p| matches!(p, FormatStringPart::Expr(_))) {
        return ExprKind::FormatString(parts);
    }
    match parts.pop() {
        Some(FormatStringPart::Text(text)) => ExprKind::String(text),
        _ => ExprKind::String(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use crate::ast::*;
    use crate::parser::expr::expr;

    fn p(s: &str) -> Result<ExprKind, Vec<String>> {
        expr()
            .then_ignore(end())
            .parse(s)
            .into_result()
            .map(|e| e.kind)
            .map_err(|errs| errs.iter().map(|e| e.to_string()).collect())
    }

    fn string(s: &str) -> Result<ExprKind, Vec<String>> {
        Ok(ExprKind::String(s.to_string()))
    }

    fn column(name: &str) -> Expr {
        let part = PathPart::from(PathPartKind::Column(name.to_string()));
        ExprKind::Path(vec![part]).into()
    }

    fn text(s: &str) -> FormatStringPart {
        FormatStringPart::Text(s.to_string())
    }

    #[test]
    fn test_quotes() {
        assert_eq!(p(r#"^"foo""#), string("foo"));
        assert_eq!(p(r#"^'foo'"#), string("foo"));
        assert_eq!(p(r#"^^Don't say "never"^"#), string(r#"Don't say "never""#));
        assert_eq!(p("^#a#"), string("a"));
        assert_eq!(p("^/v\\d+/"), string("v\\d+"));
        assert_eq!(p("^|a|"), string("a"));
        assert_eq!(p("^//"), string(""));
        assert_eq!(p("^''"), string(""));
        assert_eq!(p("^\"a\nb\""), string("a\nb"));
    }

    #[test]
    fn test_raw_without_flags() {
        assert_eq!(p(r#"^"a\nb""#), string(r"a\nb"));
        assert_eq!(p(r#"^"{a}""#), string("{a}"));
    }

    #[test]
    fn test_escape_flag() {
        assert_eq!(
            p(r#"^e'Don\'t say "never"'"#),
            string(r#"Don't say "never""#)
        );
        assert_eq!(p(r#"^e"a\nb\tc\rd\\e""#), string("a\nb\tc\rd\\e"));
        assert_eq!(p(r#"^e"\u{e9}\u{1F600}""#), string("é😀"));
        assert_eq!(p(r#"^e^a\^b^"#), string("a^b"));
        // Braces are literal without the `f` flag.
        assert_eq!(p(r#"^e"{a}""#), string("{a}"));
        assert!(p(r#"^e"\d""#).is_err());
        assert!(p(r#"^e"\{""#).is_err());
        assert!(p(r#"^e'\"'"#).is_err());
        assert!(p(r#"^e"\u{110000}""#).is_err());
        assert!(p(r#"^e"\u{}""#).is_err());
    }

    #[test]
    fn test_format_flag() {
        assert_eq!(
            p(r#"^f"Hello {username}!""#),
            Ok(ExprKind::FormatString(vec![
                text("Hello "),
                FormatStringPart::Expr(column("username")),
                text("!"),
            ]))
        );
        assert_eq!(
            p(r#"^f"{a}{ b }""#),
            Ok(ExprKind::FormatString(vec![
                FormatStringPart::Expr(column("a")),
                FormatStringPart::Expr(column("b")),
            ]))
        );
        // Without interpolations, it's a plain string.
        assert_eq!(p(r#"^f"{{a}} b""#), string("{a} b"));
        assert_eq!(p(r#"^f"""#), string(""));
        // Escapes need the `e` flag.
        assert_eq!(p(r#"^f"\n{{""#), string("\\n{"));
        assert_eq!(p(r#"^fe"\n\{\}""#), string("\n{}"));
        assert_eq!(p(r#"^ef"\n""#), string("\n"));
        assert!(p(r#"^f"{}""#).is_err());
        assert!(p(r#"^f"a}b""#).is_err());
        assert!(p(r#"^f"{a""#).is_err());
    }

    #[test]
    fn test_quote_within_interpolation() {
        let Ok(ExprKind::FormatString(parts)) = p(r#"^f|{name|upper}|"#) else {
            panic!("expected a format string");
        };
        assert_eq!(parts.len(), 1);
        let Ok(ExprKind::FormatString(parts)) = p(r#"^f"{status|concat("x")}!""#) else {
            panic!("expected a format string");
        };
        assert_eq!(parts.len(), 2);
        // A nested formatted string.
        assert!(matches!(
            p(r#"^f"a{^f"b{c}"}""#),
            Ok(ExprKind::FormatString(_))
        ));
    }

    #[test]
    fn test_bare_words_in_interpolation_on_right_hand_side() {
        let Ok(ExprKind::Comparison(comparison)) = p(r#"title:^f"Re: {subject}""#) else {
            panic!("expected a comparison");
        };
        let ComparisonSide::Expr(right) = comparison.right else {
            panic!("expected an expression");
        };
        assert_eq!(
            right.kind,
            ExprKind::FormatString(vec![
                text("Re: "),
                FormatStringPart::Expr(column("subject"))
            ])
        );
    }

    #[test]
    fn test_invalid_flags() {
        assert!(p(r#"^x"a""#).is_err());
        assert!(p(r#"^F"a""#).is_err());
        assert!(p(r#"^ff"a""#).is_err());
        assert!(p("^a").is_err());
        assert!(p(r#"^ "a""#).is_err());
    }
}
