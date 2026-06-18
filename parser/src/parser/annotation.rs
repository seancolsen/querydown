use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::utils::*;

/// Parses a column-level annotation: a top-level object introduced by `@`, e.g. `@{width:100}`.
///
/// The annotation sub-language is a JSON-like variant in which:
/// - object entries and array elements are whitespace-delimited (no commas),
/// - strings may be left unquoted if they are identifiers,
/// - and the values `null`, `true`, and `false` are written with a `@` sigil.
///
/// The top level must be an object. Nested objects do not take the `@` sigil.
pub fn annotation<'src>() -> impl Psr<'src, AnnotationValue> {
    just(CONST_SIGIL).ignore_then(object(value()))
}

/// A recursive parser for any annotation value (object, array, number, string, or `@`-constant).
fn value<'src>() -> impl Psr<'src, AnnotationValue> {
    recursive(|value| {
        // `@true`, `@false`, and `@null`. Note that only these three constants are permitted here
        // — unlike elsewhere in the language, `@`-prefixed dates, durations, and other constants
        // are intentionally not accepted, because annotations must be JSON-representable.
        let constant = just(CONST_SIGIL).ignore_then(choice((
            exactly(LITERAL_TRUE).to(AnnotationValue::Bool(true)),
            exactly(LITERAL_FALSE).to(AnnotationValue::Bool(false)),
            exactly(LITERAL_NULL).to(AnnotationValue::Null),
        )));

        let number = just('-')
            .or_not()
            .then(int(10))
            .then(just('.').then(digits(10)).or_not())
            .to_slice()
            .map(|s: &str| AnnotationValue::Number(s.to_string()));

        let quoted_string = quoted(STRING_QUOTE_SINGLE)
            .or(quoted(STRING_QUOTE_DOUBLE))
            .map(AnnotationValue::String);

        // An unquoted identifier is treated as a string. This is also why bare `true`/`false`/
        // `null` (without the `@` sigil) parse as the strings `"true"`/`"false"`/`"null"`.
        let ident_string = ident().map(|s: &str| AnnotationValue::String(s.to_string()));

        let array = value
            .clone()
            .padded_by(pad())
            .repeated()
            .collect::<Vec<AnnotationValue>>()
            .delimited_by(
                just(CONDITION_SET_OR_BRACE_L),
                just(CONDITION_SET_OR_BRACE_R),
            )
            .map(AnnotationValue::Array);

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

/// Parses an annotation object `{ key:value ... }` given a parser for its values. Used both for the
/// top-level object and for nested objects.
fn object<'src>(value: impl Psr<'src, AnnotationValue>) -> impl Psr<'src, AnnotationValue> {
    let key = ident()
        .map(|s: &str| s.to_string())
        .or(quoted(STRING_QUOTE_SINGLE))
        .or(quoted(STRING_QUOTE_DOUBLE));

    let entry = key
        .then_ignore(just(ANNOTATION_KEY_VALUE_SEPARATOR).padded_by(pad()))
        .then(value);

    entry
        .padded_by(pad())
        .repeated()
        .collect::<Vec<(String, AnnotationValue)>>()
        .delimited_by(
            just(CONDITION_SET_AND_BRACE_L),
            just(CONDITION_SET_AND_BRACE_R),
        )
        .map(AnnotationValue::Object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<AnnotationValue, ()> {
        annotation()
            .then_ignore(end())
            .parse(s)
            .into_result()
            .map_err(|_| ())
    }

    fn obj(entries: Vec<(&str, AnnotationValue)>) -> AnnotationValue {
        AnnotationValue::Object(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    fn num(n: &str) -> AnnotationValue {
        AnnotationValue::Number(n.to_string())
    }

    fn s(v: &str) -> AnnotationValue {
        AnnotationValue::String(v.to_string())
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
        assert_eq!(
            p("@{a:@true}"),
            Ok(obj(vec![("a", AnnotationValue::Bool(true))]))
        );
        assert_eq!(
            p("@{a:@false}"),
            Ok(obj(vec![("a", AnnotationValue::Bool(false))]))
        );
        assert_eq!(p("@{a:@null}"), Ok(obj(vec![("a", AnnotationValue::Null)])));
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
                AnnotationValue::Array(vec![
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
