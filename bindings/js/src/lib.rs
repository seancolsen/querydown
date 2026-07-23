use querydown::ast::Query;
use querydown::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn parse_dialect(dialect: &str) -> Result<Box<dyn Dialect>, String> {
    match dialect {
        "postgres" => Ok(Box::new(Postgres())),
        "duckdb" => Ok(Box::new(DuckDB())),
        _ => Err("Invalid dialect".to_string()),
    }
}

/// TypeScript declarations for the object returned by [`compile`] and [`compile_sections`]. These are
/// appended to the generated `.d.ts` so the exported functions are typed as `CompileResult` rather
/// than `any`. `columnAnnotations` mirrors the compiler's `Vec<Option<AnnotationValue>>`: one entry
/// per output column, in order, `null` for a column that carries no annotation.
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
/**
 * A value in Querydown's JSON-like annotation sub-language, carrying arbitrary
 * application-defined annotations for result columns.
 */
export type AnnotationValue =
  | null
  | boolean
  | number
  | string
  | AnnotationValue[]
  | { [key: string]: AnnotationValue };

/** The result of compiling Querydown source: the generated SQL plus per-column annotations. */
export interface CompileResult {
  /** The generated SQL query. */
  sql: string;
  /**
   * One entry per output column, in column order; `null` where a column has no annotation.
   */
  columnAnnotations: (AnnotationValue | null)[];
}
"#;

#[wasm_bindgen]
extern "C" {
    /// The typed `CompileResult` object returned to JavaScript. Backed by a plain object produced by
    /// `serde_wasm_bindgen`; the `typescript_type` gives it an accurate `.d.ts` signature.
    #[wasm_bindgen(typescript_type = "CompileResult")]
    pub type JsCompileResult;
}

/// Builds a [`Compiler`] for the given schema and dialect, mapping any setup error to a thrown JS
/// error.
fn make_compiler(schema_json: &str, dialect: &str) -> Result<Compiler, JsError> {
    let options = Options {
        dialect: parse_dialect(dialect).map_err(|e| JsError::new(&e))?,
        identifier_resolution: IdentifierResolution::Flexible,
    };
    Compiler::new(schema_json, options).map_err(|e| JsError::new(&e))
}

/// Serializes a [`CompileResult`] into the typed JS object returned to callers.
fn to_js_result(result: &CompileResult) -> Result<JsCompileResult, JsError> {
    serde_wasm_bindgen::to_value(result)
        .map(|value| value.unchecked_into())
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Prefixes a section parser's error with the section name so a section-parse failure is
/// attributable to the input it came from (e.g. `"Sort section: …"`).
fn section_error(section: &str, err: String) -> JsError {
    JsError::new(&format!("{section} section: {err}"))
}

/// Return the static SQL query that introspects a database of the given dialect to produce the
/// schema JSON consumed by [`compile`]. The query yields a single row with a single column
/// containing the JSON.
#[wasm_bindgen]
pub fn introspection_sql(dialect: &str) -> Result<String, String> {
    Ok(parse_dialect(dialect)?.introspection_sql().to_string())
}

/// Compiles whole-query Querydown source into SQL plus column annotations, returning a typed
/// `CompileResult` object. Compilation errors are thrown as JS exceptions.
#[wasm_bindgen]
pub fn compile(
    schema_json: &str,
    dialect: &str,
    input: String,
) -> Result<JsCompileResult, JsError> {
    let compiler = make_compiler(schema_json, dialect)?;
    let result = compiler.compile(input).map_err(|e| JsError::new(&e))?;
    to_js_result(&result)
}

/// Compiles a query supplied as independently-parsed sections — definitions, conditions, sorting,
/// and display — plus a base table, returning the same typed `CompileResult` as [`compile`].
///
/// Each section is parsed with its own parser, so one section's syntax cannot leak into another (a
/// stray display `$` in the sort input is reported as a sort error rather than silently altering the
/// result set). A parse error is prefixed with its section name; a compilation error is thrown as-is.
#[wasm_bindgen]
pub fn compile_sections(
    schema_json: &str,
    dialect: &str,
    base_table: String,
    definitions: &str,
    conditions: &str,
    sorting: &str,
    display: &str,
) -> Result<JsCompileResult, JsError> {
    let query = Query::from_parts(
        base_table,
        parse_definitions(definitions).map_err(|e| section_error("Definitions", e))?,
        parse_conditions(conditions).map_err(|e| section_error("Conditions", e))?,
        parse_sorting(sorting).map_err(|e| section_error("Sort", e))?,
        parse_display(display).map_err(|e| section_error("Display", e))?,
    );
    let compiler = make_compiler(schema_json, dialect)?;
    let result = compiler
        .compile_query(query)
        .map_err(|e| JsError::new(&e))?;
    to_js_result(&result)
}
