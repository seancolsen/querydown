//! Tests for the wasm binding, run against a real JS runtime.
//!
//! Run with `wasm-pack test --node bindings/js`. These exercise `wasm_bindgen`'s JS interop, which
//! only works on `wasm32`, so the whole module is gated to that target.
#![cfg(target_arch = "wasm32")]

use querydown_js::{compile, compile_sections, tokenize, Completer as QuerydownCompleter};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

const SCHEMA: &str = r#"{
  "tables": [
    {
      "name": "t",
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "name", "type": "text" }
      ]
    }
  ],
  "links": []
}"#;

/// Reads the string `sql` property off a value returned by `compile`.
fn read_sql(value: &JsValue) -> String {
    assert!(value.is_object(), "compile result should be a JS object");
    js_sys::Reflect::get(value, &JsValue::from_str("sql"))
        .expect("result has a `sql` property")
        .as_string()
        .expect("`sql` is a string")
}

#[wasm_bindgen_test]
fn compile_returns_typed_object() {
    let result = compile(SCHEMA, "postgres", "#t $id".to_string()).expect("compiles");
    let value: JsValue = result.unchecked_into();

    // The return value is an object, not the JSON string it used to be.
    assert!(value.is_object());
    assert!(value.as_string().is_none());

    let sql = read_sql(&value);
    assert!(!sql.is_empty(), "sql should be non-empty");
    assert!(sql.to_lowercase().contains("select"));
}

#[wasm_bindgen_test]
fn compile_sections_matches_whole_query() {
    // A sectioned query compiles to the same SQL as the equivalent whole-query source.
    let whole = compile(SCHEMA, "postgres", "#t $id".to_string()).expect("whole compiles");
    let sectioned = compile_sections(SCHEMA, "postgres", "t".to_string(), "", "", "", "$id")
        .expect("sections compile");

    let whole_sql = read_sql(&whole.unchecked_into());
    let sectioned_sql = read_sql(&sectioned.unchecked_into());
    assert_eq!(whole_sql, sectioned_sql);
}

#[wasm_bindgen_test]
fn compile_sections_isolates_sections() {
    // A display column (`$`) does not belong in the sort section, so it is rejected as a sort error
    // rather than silently affecting the result set.
    let err = thrown(compile_sections(
        SCHEMA,
        "postgres",
        "t".to_string(),
        "",
        "",
        "$id",
        "$id",
    ));
    let message = js_sys::Reflect::get(&err, &JsValue::from_str("message"))
        .ok()
        .and_then(|m| m.as_string())
        .unwrap_or_default();
    assert!(
        message.starts_with("Sort section:"),
        "error should be attributed to the sort section, got: {message}"
    );
}

/// Calls `f`, expecting it to throw, and returns the thrown value.
fn thrown<T>(result: Result<T, JsValue>) -> JsValue {
    match result {
        Ok(_) => panic!("expected an error to be thrown"),
        Err(e) => e,
    }
}

fn get(value: &JsValue, path: &str) -> JsValue {
    path.split('.').fold(value.clone(), |v, key| {
        js_sys::Reflect::get(&v, &JsValue::from_str(key)).expect("property exists")
    })
}

fn get_str(value: &JsValue, path: &str) -> String {
    get(value, path)
        .as_string()
        .unwrap_or_else(|| panic!("`{path}` should be a string"))
}

fn get_num(value: &JsValue, path: &str) -> f64 {
    get(value, path)
        .as_f64()
        .unwrap_or_else(|| panic!("`{path}` should be a number"))
}

#[wasm_bindgen_test]
fn syntax_error_is_a_located_querydown_error() {
    let err = thrown(compile(SCHEMA, "postgres", "#t\n  name:)".to_string()));

    // A real `Error`, recognizable by name and by the exported type guard.
    assert!(err.is_instance_of::<js_sys::Error>());
    assert_eq!(get_str(&err, "name"), "QuerydownError");
    assert!(querydown_js::is_querydown_error(&err));

    assert_eq!(get_str(&err, "kind"), "syntax");
    assert!(get(&err, "section").is_null());
    assert_eq!(
        get_str(&err, "reason"),
        "Unexpected `)`, which has no matching `(`. Expected a value."
    );
    assert_eq!(
        get_str(&err, "message"),
        "Unexpected `)`, which has no matching `(`. Expected a value. (line 2, column 8)"
    );
    assert_eq!(get_num(&err, "span.start.line"), 2.0);
    assert_eq!(get_num(&err, "span.start.column"), 8.0);
    assert_eq!(get_num(&err, "span.start.offset"), 10.0);
    assert_eq!(get_num(&err, "span.end.offset"), 11.0);
    assert_eq!(get_str(&err, "found"), "`)`");
    let expected = js_sys::Array::from(&get(&err, "expected"));
    assert_eq!(expected.length(), 1);
    assert_eq!(expected.get(0).as_string().unwrap(), "a value");
    assert!(get_str(&err, "hint").contains("Remove this `)`"));
    assert_eq!(js_sys::Array::from(&get(&err, "labels")).length(), 0);
}

#[wasm_bindgen_test]
fn span_offsets_and_columns_are_utf16_code_units() {
    // `😀` is one char in Rust but two UTF-16 code units in JS, so everything after it is shifted
    // by one relative to a char count (and by three relative to a byte count).
    let source = "#t name:'😀' [id:1";
    let err = thrown(compile(SCHEMA, "postgres", source.to_string()));
    assert_eq!(
        get_str(&err, "reason"),
        "Unclosed `[`. Expected a matching `]` before the end of input."
    );
    let start = get_num(&err, "span.start.offset") as usize;
    let utf16: Vec<u16> = source.encode_utf16().collect();
    assert_eq!(String::from_utf16(&utf16[start..start + 1]).unwrap(), "[");
    assert_eq!(get_num(&err, "span.start.column"), (start + 1) as f64);
    // The secondary label marks where the `]` was expected: the end of the input.
    let labels = js_sys::Array::from(&get(&err, "labels"));
    assert_eq!(labels.length(), 1);
    assert_eq!(
        get_num(&labels.get(0), "span.start.offset"),
        utf16.len() as f64
    );
}

#[wasm_bindgen_test]
fn section_syntax_error_is_attributed_to_its_section() {
    let err = thrown(compile_sections(
        SCHEMA,
        "postgres",
        "t".to_string(),
        "",
        "name:1",
        r"\\id $id",
        "",
    ));
    assert_eq!(get_str(&err, "kind"), "syntax");
    assert_eq!(get_str(&err, "section"), "sorting");
    // The span is relative to the section's own text.
    assert_eq!(get_num(&err, "span.start.offset"), 5.0);
    assert!(get_str(&err, "message").starts_with("Sort section: Unexpected `$`."));
    assert!(get_str(&err, "message").ends_with("(line 1, column 6)"));
}

#[wasm_bindgen_test]
fn compile_error_has_no_span() {
    let err = thrown(compile(SCHEMA, "postgres", "#t $nope".to_string()));
    assert!(querydown_js::is_querydown_error(&err));
    assert_eq!(get_str(&err, "kind"), "compile");
    assert_eq!(
        get_str(&err, "message"),
        "Column `nope` not found within table `t`."
    );
    // Absent values are `null`, not `undefined`, to match the TypeScript declarations.
    assert!(get(&err, "span").is_null());
    assert!(get(&err, "hint").is_null());
    assert!(get(&err, "found").is_null());
}

#[wasm_bindgen_test]
fn schema_and_argument_errors_have_their_own_kinds() {
    let err = thrown(compile("{", "postgres", "#t".to_string()));
    assert_eq!(get_str(&err, "kind"), "schema");
    assert!(get_str(&err, "message").starts_with("The schema is not valid"));

    let err = thrown(compile(SCHEMA, "mysql", "#t".to_string()));
    assert_eq!(get_str(&err, "kind"), "argument");
    assert!(get_str(&err, "message").contains("mysql"));

    let err = thrown(querydown_js::introspection_sql("mysql"));
    assert!(querydown_js::is_querydown_error(&err));
    assert_eq!(get_str(&err, "kind"), "argument");
}

#[wasm_bindgen_test]
fn is_querydown_error_rejects_other_values() {
    assert!(!querydown_js::is_querydown_error(
        &js_sys::Error::new("plain").into()
    ));
    assert!(!querydown_js::is_querydown_error(&JsValue::from_str(
        "QuerydownError"
    )));
    assert!(!querydown_js::is_querydown_error(&JsValue::NULL));
}

#[wasm_bindgen_test]
fn tokenize_returns_tokens_with_utf16_offsets() {
    let source = "#t name:'😀' $id";
    let tokens = js_sys::Array::from(&tokenize(source, "query").expect("tokenizes"));
    let utf16: Vec<u16> = source.encode_utf16().collect();
    let rendered: Vec<String> = tokens
        .iter()
        .map(|token| {
            let from = get_num(&token, "from") as usize;
            let to = get_num(&token, "to") as usize;
            let text = String::from_utf16(&utf16[from..to]).unwrap();
            format!("{text}:{}", get_str(&token, "kind"))
        })
        .collect();
    assert_eq!(
        rendered,
        [
            "#t:base-table",
            "name:column",
            "::comparison-operator",
            "'😀':string",
            "$:result-column",
            "id:column",
        ]
    );
}

#[wasm_bindgen_test]
fn tokenize_tolerates_invalid_source_but_not_an_unknown_grammar() {
    let tokens = js_sys::Array::from(&tokenize("name:)", "conditions").expect("tokenizes"));
    assert_eq!(tokens.length(), 3);
    let err = thrown(tokenize("", "nope"));
    assert_eq!(get_str(&err, "kind"), "argument");
}

fn completer() -> QuerydownCompleter {
    QuerydownCompleter::new(SCHEMA).expect("the schema is valid")
}

/// The labels of the items in a value returned by `complete`.
fn item_labels(completions: &JsValue) -> Vec<String> {
    js_sys::Array::from(&get(completions, "items"))
        .iter()
        .map(|item| get_str(&item, "label"))
        .collect()
}

/// A JS object with the given string properties.
fn object(properties: &[(&str, &str)]) -> querydown_js::JsCompletionOptionsObject {
    let object = js_sys::Object::new();
    for (key, value) in properties {
        js_sys::Reflect::set(&object, &(*key).into(), &(*value).into()).unwrap();
    }
    JsValue::from(object).unchecked_into()
}

#[wasm_bindgen_test]
fn completer_offers_columns_with_utf16_offsets() {
    // `😀` is 2 UTF-16 code units, so the word `na` starts at offset 14 (not 16, as in bytes).
    let source = "#t name:'😀' $na";
    let utf16_len = source.encode_utf16().count();
    let completions = completer()
        .complete(source, utf16_len, None)
        .expect("completes");
    assert_eq!(get_num(&completions, "from"), 14.0);
    assert_eq!(get_num(&completions, "to"), 16.0);
    assert_eq!(item_labels(&completions), ["id", "name"]);
    let item = js_sys::Array::from(&get(&completions, "items")).get(1);
    assert_eq!(get_str(&item, "kind"), "column");
    assert_eq!(get_str(&item, "detail"), "text");
    // Absent values are `null`, not `undefined`, to match the TypeScript declarations.
    assert!(get(&item, "insert").is_null());
    assert!(get(&completions, "annotation").is_null());
}

#[wasm_bindgen_test]
fn completer_accepts_section_options() {
    let options = object(&[
        ("grammar", "display"),
        ("baseTable", "t"),
        ("definitions", "#t.upper_name = name|uppercase"),
    ]);
    let completions = completer()
        .complete("$", 1, Some(options))
        .expect("completes");
    assert_eq!(item_labels(&completions), ["id", "name", "upper_name"]);
}

#[wasm_bindgen_test]
fn completer_reports_annotation_positions() {
    let source = "#t $id @{width:100 ";
    let completions = completer()
        .complete(source, source.len(), None)
        .expect("completes");
    assert_eq!(item_labels(&completions), Vec::<String>::new());
    assert_eq!(get_str(&completions, "annotation.slot"), "key");
    let keys = js_sys::Array::from(&get(&completions, "annotation.keys"));
    assert_eq!(keys.get(0).as_string().as_deref(), Some("width"));
}

#[wasm_bindgen_test]
fn completer_errors() {
    let err = thrown(QuerydownCompleter::new("{"));
    assert_eq!(get_str(&err, "kind"), "schema");
    let err = thrown(completer().complete("", 0, Some(object(&[("grammar", "nope")]))));
    assert_eq!(get_str(&err, "kind"), "argument");
}
