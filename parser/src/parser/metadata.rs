use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::utils::*;

/// Parses column-level metadata: a top-level object introduced by `@`, e.g. `@{width:100}`.
///
/// The metadata sub-language is a JSON-like variant in which:
/// - object entries and array elements are whitespace-delimited (no commas),
/// - strings may be left unquoted if they are identifiers,
/// - and the values `null`, `true`, and `false` are written with a `@` sigil.
///
/// The top level must be an object. Nested objects do not take the `@` sigil.
pub fn metadata<'src>() -> impl Psr<'src, MetaValue> {
    just(CONST_SIGIL).ignore_then(object(value()))
}

/// A recursive parser for any metadata value (object, array, number, string, or `@`-constant).
fn value<'src>() -> impl Psr<'src, MetaValue> {
    recursive(|value| {
        // `@true`, `@false`, and `@null`. Note that only these three constants are permitted here
        // — unlike elsewhere in the language, `@`-prefixed dates, durations, and other constants
        // are intentionally not accepted, because metadata must be JSON-representable.
        let constant = just(CONST_SIGIL).ignore_then(choice((
            exactly(LITERAL_TRUE).to(MetaValue::Bool(true)),
            exactly(LITERAL_FALSE).to(MetaValue::Bool(false)),
            exactly(LITERAL_NULL).to(MetaValue::Null),
        )));

        let number = just('-')
            .or_not()
            .then(int(10))
            .then(just('.').then(digits(10)).or_not())
            .to_slice()
            .map(|s: &str| MetaValue::Number(s.to_string()));

        let quoted_string = quoted(STRING_QUOTE_SINGLE)
            .or(quoted(STRING_QUOTE_DOUBLE))
            .map(MetaValue::String);

        // An unquoted identifier is treated as a string. This is also why bare `true`/`false`/
        // `null` (without the `@` sigil) parse as the strings `"true"`/`"false"`/`"null"`.
        let ident_string = ident().map(|s: &str| MetaValue::String(s.to_string()));

        let array = value
            .clone()
            .padded()
            .repeated()
            .collect::<Vec<MetaValue>>()
            .delimited_by(
                just(CONDITION_SET_OR_BRACE_L),
                just(CONDITION_SET_OR_BRACE_R),
            )
            .map(MetaValue::Array);

        choice((
            object(value),
            array,
            constant,
            number,
            quoted_string,
            ident_string,
        ))
    })
}

/// Parses a metadata object `{ key:value ... }` given a parser for its values. Used both for the
/// top-level object and for nested objects.
fn object<'src>(value: impl Psr<'src, MetaValue>) -> impl Psr<'src, MetaValue> {
    let key = ident()
        .map(|s: &str| s.to_string())
        .or(quoted(STRING_QUOTE_SINGLE))
        .or(quoted(STRING_QUOTE_DOUBLE));

    let entry = key
        .then_ignore(just(METADATA_KEY_VALUE_SEPARATOR).padded())
        .then(value);

    entry
        .padded()
        .repeated()
        .collect::<Vec<(String, MetaValue)>>()
        .delimited_by(
            just(CONDITION_SET_AND_BRACE_L),
            just(CONDITION_SET_AND_BRACE_R),
        )
        .map(MetaValue::Object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<MetaValue, ()> {
        metadata()
            .then_ignore(end())
            .parse(s)
            .into_result()
            .map_err(|_| ())
    }

    fn obj(entries: Vec<(&str, MetaValue)>) -> MetaValue {
        MetaValue::Object(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    fn num(n: &str) -> MetaValue {
        MetaValue::Number(n.to_string())
    }

    fn s(v: &str) -> MetaValue {
        MetaValue::String(v.to_string())
    }

    #[test]
    fn test_scalar_values() {
        assert_eq!(p("@{width:100}"), Ok(obj(vec![("width", num("100"))])));
        assert_eq!(p("@{x:-2.5}"), Ok(obj(vec![("x", num("-2.5"))])));
        assert_eq!(
            p("@{formatter:timeElapsed}"),
            Ok(obj(vec![("formatter", s("timeElapsed"))]))
        );
        assert_eq!(p("@{a:'foo'}"), Ok(obj(vec![("a", s("foo"))])));
        assert_eq!(p("@{a:\"foo\"}"), Ok(obj(vec![("a", s("foo"))])));
        assert_eq!(p("@{a:@true}"), Ok(obj(vec![("a", MetaValue::Bool(true))])));
        assert_eq!(
            p("@{a:@false}"),
            Ok(obj(vec![("a", MetaValue::Bool(false))]))
        );
        assert_eq!(p("@{a:@null}"), Ok(obj(vec![("a", MetaValue::Null)])));
        // Bare `true` (without the sigil) is a string.
        assert_eq!(p("@{a:true}"), Ok(obj(vec![("a", s("true"))])));
    }

    #[test]
    fn test_quoted_key() {
        assert_eq!(p("@{'a b':1}"), Ok(obj(vec![("a b", num("1"))])));
    }

    #[test]
    fn test_multiple_entries_preserve_order() {
        assert_eq!(
            p("@{formatter:timeElapsed textColor:light}"),
            Ok(obj(vec![
                ("formatter", s("timeElapsed")),
                ("textColor", s("light")),
            ]))
        );
    }

    #[test]
    fn test_nested_object_and_array() {
        assert_eq!(
            p("@{formattingConditions:[\n  {gte:10 bg:'#fbc9ff'}\n  {gte:5 bg:'#d5d2ff'}\n]}"),
            Ok(obj(vec![(
                "formattingConditions",
                MetaValue::Array(vec![
                    obj(vec![("gte", num("10")), ("bg", s("#fbc9ff"))]),
                    obj(vec![("gte", num("5")), ("bg", s("#d5d2ff"))]),
                ])
            )]))
        );
    }

    #[test]
    fn test_empty_object() {
        assert_eq!(p("@{}"), Ok(obj(vec![])));
    }

    #[test]
    fn test_rejects_non_object_top_level() {
        // The top level must be an object, not a bare value.
        assert!(p("@true").is_err());
        assert!(p("@5").is_err());
    }

    #[test]
    fn test_rejects_non_json_constants() {
        assert!(p("@{a:@now}").is_err());
        assert!(p("@{a:@2y}").is_err());
    }

    #[test]
    fn test_rejects_unterminated() {
        assert!(p("@{a:1").is_err());
        assert!(p("@{a:[1 2}").is_err());
    }
}
