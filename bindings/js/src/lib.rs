use querydown::ast::Query;
use querydown::*;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// TypeScript declarations for the objects returned and thrown by the exported functions. These are
/// appended to the generated `.d.ts` so the exported functions are typed as `CompileResult` rather
/// than `any`, and so that consumers can work with thrown `QuerydownError`s in a typed way.
/// `columnAnnotations` mirrors the compiler's `Vec<Option<AnnotationValue>>`: one entry per output
/// column, in order, `null` for a column that carries no annotation.
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

/**
 * The broad category of a {@link QuerydownError}:
 *
 * - `"syntax"`: the Querydown source is not syntactically valid. Always has a `span`.
 * - `"compile"`: the source is valid Querydown but can't be compiled against the schema (e.g. it
 *   refers to a column that doesn't exist). Has no `span` (yet).
 * - `"schema"`: the schema JSON is invalid. Has no `span`.
 * - `"argument"`: some other argument is invalid (e.g. an unknown dialect). Has no `span`.
 *
 * More kinds may be added in the future.
 */
export type QuerydownErrorKind = "syntax" | "compile" | "schema" | "argument";

/** The section of a sectioned query (see `compile_sections`) that an error occurred in. */
export type QuerydownSection = "definitions" | "conditions" | "sorting" | "display";

/**
 * A location within the source text. `offset` and `column` are measured in UTF-16 code units —
 * the same units as JavaScript string indices — so `source.slice(start.offset, end.offset)`
 * extracts a span's text. `line` and `column` are 1-based.
 */
export interface SourcePosition {
  offset: number;
  line: number;
  column: number;
}

/**
 * A region of the source text, from `start` (inclusive) to `end` (exclusive). It may be empty
 * (`start.offset === end.offset`), e.g. when the input ended unexpectedly.
 */
export interface SourceSpan {
  start: SourcePosition;
  end: SourcePosition;
}

/** A secondary location relevant to an error, e.g. where an unclosed bracket was opened. */
export interface QuerydownErrorLabel {
  span: SourceSpan;
  message: string;
}

/**
 * The error thrown by every exported function on failure. It is a real `Error` (so it has a
 * `stack`, and `instanceof Error` holds), with `name` set to `"QuerydownError"` and the extra
 * properties below. Use {@link is_querydown_error} to narrow a caught value to this type.
 */
export interface QuerydownError extends Error {
  name: "QuerydownError";
  /**
   * A one-line description including the section (if any) and location (if any), e.g.
   * ``"Sort section: Unexpected `$`. Expected a sort expression (`\\`) or the end of the input.
   * (line 1, column 6)"``.
   */
  message: string;
  kind: QuerydownErrorKind;
  /** The description of the problem alone, without the section or location. */
  reason: string;
  /** For `compile_sections`, the section whose text the error (and its `span`) refers to. */
  section: QuerydownSection | null;
  /**
   * Where in the source the problem is. Relative to the whole query for `compile`, or to the
   * section's text for `compile_sections`.
   */
  span: SourceSpan | null;
  /** Secondary locations relevant to the problem, each with a note. */
  labels: QuerydownErrorLabel[];
  /** For syntax errors, descriptions of what would have been accepted at `span`. */
  expected: string[];
  /** For syntax errors, a description of what was found at `span` instead. */
  found: string | null;
  /** A suggestion on how to fix the problem, if one is available. */
  hint: string | null;
}

/** Returns true if `value` is a {@link QuerydownError} thrown by this package. */
export function is_querydown_error(value: unknown): value is QuerydownError;
"#;

#[wasm_bindgen]
extern "C" {
    /// The typed `CompileResult` object returned to JavaScript. Backed by a plain object produced by
    /// `serde_wasm_bindgen`; the `typescript_type` gives it an accurate `.d.ts` signature.
    #[wasm_bindgen(typescript_type = "CompileResult")]
    pub type JsCompileResult;
}

/// The `name` given to every error thrown by this package.
const ERROR_NAME: &str = "QuerydownError";

/// A section of a sectioned query, used to attribute an error to the input it came from.
#[derive(Clone, Copy)]
enum Section {
    Definitions,
    Conditions,
    Sorting,
    Display,
}

impl Section {
    fn id(self) -> &'static str {
        match self {
            Section::Definitions => "definitions",
            Section::Conditions => "conditions",
            Section::Sorting => "sorting",
            Section::Display => "display",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Section::Definitions => "Definitions",
            Section::Conditions => "Conditions",
            Section::Sorting => "Sort",
            Section::Display => "Display",
        }
    }
}

/// A [`Position`] re-expressed in UTF-16 code units, as JavaScript measures strings.
#[derive(Serialize)]
struct JsPosition {
    offset: usize,
    line: usize,
    column: usize,
}

#[derive(Serialize)]
struct JsSpan {
    start: JsPosition,
    end: JsPosition,
}

#[derive(Serialize)]
struct JsLabel {
    span: JsSpan,
    message: String,
}

/// The extra properties copied onto the thrown JS `Error`. See `QuerydownError` in the TypeScript
/// declarations above.
#[derive(Serialize)]
struct JsErrorDetails {
    kind: &'static str,
    reason: String,
    section: Option<&'static str>,
    span: Option<JsSpan>,
    labels: Vec<JsLabel>,
    expected: Vec<String>,
    found: Option<String>,
    hint: Option<String>,
}

fn to_js_position(source: &str, position: &Position) -> JsPosition {
    // `position.offset` is a byte offset at a char boundary (guaranteed by `Position`), so these
    // slices are safe.
    let before = &source[..position.offset.min(source.len())];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    JsPosition {
        offset: before.encode_utf16().count(),
        line: position.line,
        column: before[line_start..].encode_utf16().count() + 1,
    }
}

fn to_js_span(source: &str, span: &Span) -> JsSpan {
    JsSpan {
        start: to_js_position(source, &span.start),
        end: to_js_position(source, &span.end),
    }
}

/// Builds a JS `Error` carrying the given details, with `name` set to [`ERROR_NAME`].
fn make_js_error(message: &str, details: JsErrorDetails) -> JsValue {
    let error = js_sys::Error::new(message);
    error.set_name(ERROR_NAME);
    // `null` (rather than `undefined`) for absent values, to match the TypeScript declarations.
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    if let Ok(details) = details.serialize(&serializer) {
        js_sys::Object::assign(&error, &details.unchecked_into());
    }
    error.into()
}

/// Converts a Querydown [`Error`] into a thrown `QuerydownError`. `source` is the text the error's
/// spans refer to (the whole query, or one section of it).
fn to_js_error(err: &Error, source: &str, section: Option<Section>) -> JsValue {
    let kind = match err.kind {
        ErrorKind::Syntax => "syntax",
        ErrorKind::Schema => "schema",
        _ => "compile",
    };
    let message = match section {
        Some(section) => format!("{} section: {err}", section.title()),
        None => err.to_string(),
    };
    let details = JsErrorDetails {
        kind,
        reason: err.message.clone(),
        section: section.map(Section::id),
        span: err.span.as_ref().map(|span| to_js_span(source, span)),
        labels: err
            .labels
            .iter()
            .map(|label| JsLabel {
                span: to_js_span(source, &label.span),
                message: label.message.clone(),
            })
            .collect(),
        expected: err.expected.clone(),
        found: err.found.clone(),
        hint: err.hint.clone(),
    };
    make_js_error(&message, details)
}

/// Builds a `QuerydownError` of kind `"argument"`, for invalid arguments other than the query and
/// schema.
fn argument_error(message: String) -> JsValue {
    let details = JsErrorDetails {
        kind: "argument",
        reason: message.clone(),
        section: None,
        span: None,
        labels: Vec::new(),
        expected: Vec::new(),
        found: None,
        hint: None,
    };
    make_js_error(&message, details)
}

fn parse_dialect(dialect: &str) -> Result<Box<dyn Dialect>, JsValue> {
    match dialect {
        "postgres" => Ok(Box::new(Postgres())),
        "duckdb" => Ok(Box::new(DuckDB())),
        _ => Err(argument_error(format!(
            "Invalid dialect `{dialect}`. Expected \"postgres\" or \"duckdb\"."
        ))),
    }
}

/// Builds a [`Compiler`] for the given schema and dialect, mapping any setup error to a thrown
/// `QuerydownError`.
fn make_compiler(schema_json: &str, dialect: &str) -> Result<Compiler, JsValue> {
    let options = Options {
        dialect: parse_dialect(dialect)?,
        identifier_resolution: IdentifierResolution::Flexible,
    };
    Compiler::new(schema_json, options).map_err(|e| to_js_error(&e, schema_json, None))
}

/// Serializes a [`CompileResult`] into the typed JS object returned to callers.
fn to_js_result(result: &CompileResult) -> Result<JsCompileResult, JsValue> {
    serde_wasm_bindgen::to_value(result)
        .map(|value| value.unchecked_into())
        .map_err(|e| JsError::new(&e.to_string()).into())
}

/// Parses one section of a sectioned query, attributing any error to that section.
fn parse_section<T>(
    section: Section,
    source: &str,
    parse: fn(&str) -> Result<T, Error>,
) -> Result<T, JsValue> {
    parse(source).map_err(|e| to_js_error(&e, source, Some(section)))
}

/// Return the static SQL query that introspects a database of the given dialect to produce the
/// schema JSON consumed by [`compile`]. The query yields a single row with a single column
/// containing the JSON. Throws a `QuerydownError` of kind `"argument"` for an unknown dialect.
#[wasm_bindgen]
pub fn introspection_sql(dialect: &str) -> Result<String, JsValue> {
    Ok(parse_dialect(dialect)?.introspection_sql().to_string())
}

/// Compiles whole-query Querydown source into SQL plus column annotations, returning a typed
/// `CompileResult` object. Failures are thrown as `QuerydownError`s, whose spans refer to `input`.
#[wasm_bindgen]
pub fn compile(
    schema_json: &str,
    dialect: &str,
    input: String,
) -> Result<JsCompileResult, JsValue> {
    let compiler = make_compiler(schema_json, dialect)?;
    let result = compiler
        .compile(input.clone())
        .map_err(|e| to_js_error(&e, &input, None))?;
    to_js_result(&result)
}

/// Compiles a query supplied as independently-parsed sections — definitions, conditions, sorting,
/// and display — plus a base table, returning the same typed `CompileResult` as [`compile`].
///
/// Each section is parsed with its own parser, so one section's syntax cannot leak into another (a
/// stray display `$` in the sort input is reported as a sort error rather than silently altering the
/// result set). A syntax error's `section` names the section it occurred in, its `span` is relative
/// to that section's text, and its `message` is prefixed with the section's name. A compile error
/// (which can't be attributed to a single section) has no section.
#[wasm_bindgen]
pub fn compile_sections(
    schema_json: &str,
    dialect: &str,
    base_table: String,
    definitions: &str,
    conditions: &str,
    sorting: &str,
    display: &str,
) -> Result<JsCompileResult, JsValue> {
    let query = Query::from_parts(
        base_table,
        parse_section(Section::Definitions, definitions, parse_definitions)?,
        parse_section(Section::Conditions, conditions, parse_conditions)?,
        parse_section(Section::Sorting, sorting, parse_sorting)?,
        parse_section(Section::Display, display, parse_display)?,
    );
    let compiler = make_compiler(schema_json, dialect)?;
    let result = compiler
        .compile_query(query)
        // A compile error can't be attributed to a single section; compile errors don't carry spans
        // (yet), so no source text is needed to convert one.
        .map_err(|e| to_js_error(&e, "", None))?;
    to_js_result(&result)
}

/// Returns true if `value` is a `QuerydownError` thrown by this package. (Its TypeScript signature,
/// a type guard, is declared in the custom section above.)
#[wasm_bindgen(skip_typescript)]
pub fn is_querydown_error(value: &JsValue) -> bool {
    value
        .dyn_ref::<js_sys::Error>()
        .map_or(false, |error| error.name() == ERROR_NAME)
}
