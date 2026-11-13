//! The lexical shape of flagged strings (e.g. `^f"Hello {name}"`), shared by the scanners that
//! read source without parsing it: the highlighting lexer and the syntax-error diagnosis. The parser
//! proper reads flagged strings with its own grammar (see `parser::expr::flagged_string`), which
//! agrees with this.
//!
//! A flagged string is `^`, then any flags, then a quote character, then its body, then the same
//! quote character again. Its body is read piece by piece (see [`piece`]), because an interpolation
//! (`{ … }`, with the `f` flag) contains Querydown code — possibly including the quote character,
//! e.g. `^f|{name|upper}|` — and only the scanner's caller knows how to skip over code.

use crate::tokens::*;

/// The opening of a flagged string: its `^` prefix, flags, and opening quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Prefix {
    /// The byte length of the `^` and the flags, which the opening quote follows.
    pub flags_len: usize,
    /// The quote character, which opens and closes the string.
    pub quote: char,
    /// Whether the `f` flag is present, enabling interpolation.
    pub format: bool,
    /// Whether the `e` flag is present, enabling escape sequences.
    pub escape: bool,
}

impl Prefix {
    /// The byte length of the whole prefix, including the opening quote.
    pub fn len(&self) -> usize {
        self.flags_len + self.quote.len_utf8()
    }
}

/// Reads the prefix of the flagged string at the start of `s`, if `s` begins with one. Unknown or
/// repeated flags are accepted here (the parser reports them), so that the rest of the string is
/// still read as a string.
pub(crate) fn prefix(s: &str) -> Option<Prefix> {
    let rest = s.strip_prefix(STRING_FLAGS_PREFIX)?;
    let flags_end = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    let flags = &rest[..flags_end];
    let quote = rest[flags_end..].chars().next()?;
    FLAGGED_STRING_QUOTES.contains(&quote).then(|| Prefix {
        flags_len: STRING_FLAGS_PREFIX.len_utf8() + flags_end,
        quote,
        format: flags.contains(STRING_FLAG_FORMAT),
        escape: flags.contains(STRING_FLAG_ESCAPE),
    })
}

/// The next piece of a flagged string's body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Piece {
    /// Literal text, of the given byte length.
    Text(usize),
    /// An escape sequence of the given byte length: a backslash sequence (with the `e` flag), or a
    /// doubled brace, `{{` or `}}` (with the `f` flag). It may be invalid; the parser judges that.
    Escape(usize),
    /// A lone `}` (with the `f` flag), which must be doubled to be literal.
    StrayBrace,
    /// The `{` that opens an interpolation (with the `f` flag).
    Interpolation,
    /// The closing quote.
    Close,
    /// The end of the input, before any closing quote.
    End,
}

/// Reads the next piece of the body of a flagged string with the given `prefix`, at the start of
/// `s`.
pub(crate) fn piece(s: &str, prefix: &Prefix) -> Piece {
    let Some(c) = s.chars().next() else {
        return Piece::End;
    };
    if c == prefix.quote {
        return Piece::Close;
    }
    if prefix.escape && c == STRING_ESCAPE {
        return Piece::Escape(escape_len(s));
    }
    if prefix.format {
        let doubled = |brace: char| s[1..].starts_with(brace);
        match c {
            STRING_INTERPOLATION_L if doubled(STRING_INTERPOLATION_L) => return Piece::Escape(2),
            STRING_INTERPOLATION_R if doubled(STRING_INTERPOLATION_R) => return Piece::Escape(2),
            STRING_INTERPOLATION_L => return Piece::Interpolation,
            STRING_INTERPOLATION_R => return Piece::StrayBrace,
            _ => {}
        }
    }
    let special = |c: char| {
        c == prefix.quote
            || (prefix.escape && c == STRING_ESCAPE)
            || (prefix.format && (c == STRING_INTERPOLATION_L || c == STRING_INTERPOLATION_R))
    };
    Piece::Text(s.find(special).unwrap_or(s.len()))
}

/// The byte length of the backslash escape sequence at the start of `s`: the backslash and the
/// character after it, plus a braced code point after `\u` (e.g. `\u{e9}`).
fn escape_len(s: &str) -> usize {
    let after = &s[STRING_ESCAPE.len_utf8()..];
    match after.chars().next() {
        None => STRING_ESCAPE.len_utf8(),
        Some('u') if after[1..].starts_with('{') => {
            let digits = after[2..]
                .find(|c: char| !c.is_ascii_hexdigit())
                .unwrap_or(after.len() - 2);
            let close = usize::from(after[2 + digits..].starts_with('}'));
            STRING_ESCAPE.len_utf8() + 2 + digits + close
        }
        Some(c) => STRING_ESCAPE.len_utf8() + c.len_utf8(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix() {
        let p = prefix(r#"^fe"x""#).unwrap();
        assert_eq!(
            (p.flags_len, p.quote, p.format, p.escape),
            (3, '"', true, true)
        );
        assert_eq!(p.len(), 4);
        let p = prefix("^^x^").unwrap();
        assert_eq!(
            (p.flags_len, p.quote, p.format, p.escape),
            (1, '^', false, false)
        );
        assert_eq!(prefix("^x"), None);
        assert_eq!(prefix("^ 'x'"), None);
        assert_eq!(prefix("'x'"), None);
    }

    #[test]
    fn test_pieces() {
        let fe = prefix(r#"^fe""#).unwrap();
        assert_eq!(piece(r#"ab\n"#, &fe), Piece::Text(2));
        assert_eq!(piece(r#"\n"#, &fe), Piece::Escape(2));
        assert_eq!(piece(r#"\u{1F600}x"#, &fe), Piece::Escape(9));
        assert_eq!(piece(r#"\u{12"#, &fe), Piece::Escape(5));
        assert_eq!(piece(r"\", &fe), Piece::Escape(1));
        assert_eq!(piece("{{", &fe), Piece::Escape(2));
        assert_eq!(piece("}}", &fe), Piece::Escape(2));
        assert_eq!(piece("{x}", &fe), Piece::Interpolation);
        assert_eq!(piece("}x", &fe), Piece::StrayBrace);
        assert_eq!(piece(r#""x"#, &fe), Piece::Close);
        assert_eq!(piece("", &fe), Piece::End);

        let raw = prefix("^|").unwrap();
        assert_eq!(piece(r"a\{b}|", &raw), Piece::Text(5));
        assert_eq!(piece("|", &raw), Piece::Close);
    }
}
