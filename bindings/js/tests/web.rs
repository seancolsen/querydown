//! Tests for the wasm binding, run against a real JS runtime.
//!
//! Run with `wasm-pack test --node bindings/js`. These exercise `wasm_bindgen`'s JS interop, which
//! only works on `wasm32`, so the whole module is gated to that target.
#![cfg(target_arch = "wasm32")]

use querydown_js::{compile, compile_sections};
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
    let err: JsValue =
        match compile_sections(SCHEMA, "postgres", "t".to_string(), "", "", "$id", "$id") {
            Ok(_) => panic!("stray display column in sort section should error"),
            Err(e) => e.into(),
        };
    let message = js_sys::Reflect::get(&err, &JsValue::from_str("message"))
        .ok()
        .and_then(|m| m.as_string())
        .unwrap_or_default();
    assert!(
        message.starts_with("Sort section:"),
        "error should be attributed to the sort section, got: {message}"
    );
}
