//! Tests for tables in schemas other than the current one: `#/schema/table`.
//!
//! The schema here has tables of the same name in three schemas: the current one, `archive`, and
//! `main` nested within `warehouse` (as DuckDB nests a database's schemas within the database).
//! The links between them cross from one schema to another.

use crate::{Compiler, DuckDB, Error, IdentifierResolution, Options, Postgres};

const SCHEMA: &str = r#"{
  "tables": [
    {
      "name": "customers",
      "columns": [{ "name": "id", "type": "integer" }, { "name": "name", "type": "text" }]
    },
    {
      "name": "orders",
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "customer", "type": "integer" },
        { "name": "total", "type": "numeric" }
      ]
    },
    {
      "name": "orders",
      "schema": "archive",
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "customer", "type": "integer" },
        { "name": "original", "type": "integer" },
        { "name": "total", "type": "numeric" }
      ]
    },
    {
      "name": "shipments",
      "schema": ["warehouse", "main"],
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "order", "type": "integer" },
        { "name": "shipped_on", "type": "date" }
      ]
    }
  ],
  "links": [
    {
      "from": { "table": "orders", "column": "customer" },
      "to": { "table": "customers", "column": "id" },
      "unique": false
    },
    {
      "from": { "table": "orders", "schema": "archive", "column": "customer" },
      "to": { "table": "customers", "column": "id" },
      "unique": false
    },
    {
      "from": { "table": "orders", "schema": ["archive"], "column": "original" },
      "to": { "table": "orders", "column": "id" },
      "unique": true
    },
    {
      "from": { "table": "shipments", "schema": ["warehouse", "main"], "column": "order" },
      "to": { "table": "orders", "schema": "archive", "column": "id" },
      "unique": false
    }
  ]
}"#;

fn compiler_with(schema: &str, identifier_resolution: IdentifierResolution) -> Compiler {
    let options = Options {
        dialect: Box::new(Postgres()),
        identifier_resolution,
        limit: None,
        offset: None,
    };
    Compiler::new(schema, options).unwrap()
}

fn compile(query: &str) -> Result<String, Error> {
    compiler_with(SCHEMA, IdentifierResolution::Flexible)
        .compile(query.to_string())
        .map(|result| result.sql)
}

/// The SQL of `query`, with each line trimmed, so that it can be checked for whole lines.
fn sql(query: &str) -> Vec<String> {
    compile(query)
        .unwrap_or_else(|e| panic!("{query} should compile: {e:?}"))
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

#[track_caller]
fn assert_lines(query: &str, expected: &[&str]) {
    let actual = sql(query);
    for line in expected {
        assert!(
            actual.iter().any(|l| l == line),
            "the SQL of {query} lacks the line {line:?}:\n{}",
            actual.join("\n")
        );
    }
}

fn compile_error(query: &str) -> Error {
    match compile(query) {
        Ok(sql) => panic!("{query} should fail to compile, but compiled to:\n{sql}"),
        Err(error) => error,
    }
}

#[test]
fn base_table_in_another_schema() {
    // The table is addressed by its qualified name, and its rows by its own name.
    assert_eq!(
        sql("#/archive/orders $id"),
        [
            "SELECT",
            "\"orders\".\"id\"",
            "FROM \"archive\".\"orders\" AS \"orders\";"
        ]
    );
    assert_lines(
        "#/warehouse/main/shipments $id",
        &["FROM \"warehouse\".\"main\".\"shipments\" AS \"shipments\";"],
    );
    // A table in the current schema is as it always was.
    assert_lines("#orders $id", &["FROM \"orders\";"]);
}

#[test]
fn every_name_is_resolved_flexibly() {
    assert_lines(
        "#/Archive/ORDERS $id",
        &["FROM \"archive\".\"orders\" AS \"orders\";"],
    );
    assert_lines(
        "#/WAREHOUSE/Main/shipments $id",
        &["FROM \"warehouse\".\"main\".\"shipments\" AS \"shipments\";"],
    );
    // But not with strict resolution.
    let strict = compiler_with(SCHEMA, IdentifierResolution::Strict);
    assert!(strict.compile("#/Archive/orders".to_string()).is_err());
    assert!(strict.compile("#/archive/orders".to_string()).is_ok());
}

#[test]
fn links_cross_schemas() {
    // From a table in another schema to tables in the current schema, including one of the same
    // name, which needs an alias of its own.
    assert_lines(
        "#/archive/orders $original.total $customer.name",
        &[
            "\"orders_1\".\"total\",",
            "FROM \"archive\".\"orders\" AS \"orders\"",
            "LEFT JOIN \"orders\" AS \"orders_1\" ON",
            "\"orders\".\"original\" = \"orders_1\".\"id\"",
            "LEFT JOIN \"customers\" ON",
        ],
    );
    // Through a nested schema and another schema to the current one.
    assert_lines(
        "#/warehouse/main/shipments $order.customer.name",
        &[
            "LEFT JOIN \"archive\".\"orders\" AS \"orders\" ON",
            "\"shipments\".\"order\" = \"orders\".\"id\"",
            "LEFT JOIN \"customers\" ON",
            "\"orders\".\"customer\" = \"customers\".\"id\";",
        ],
    );
    // To the many related records in another schema.
    assert_lines(
        "#customers $#orders $#/archive/orders $#/warehouse/main/shipments.shipped_on%max",
        &[
            "FROM \"orders\"",
            "FROM \"archive\".\"orders\" AS \"orders\"",
            "JOIN \"warehouse\".\"main\".\"shipments\" AS \"shipments\" ON",
            "max(\"shipments\".\"shipped_on\") AS \"v1\"",
        ],
    );
    assert_lines(
        "#customers ++#/archive/orders{total:>100}",
        &[
            "FROM \"archive\".\"orders\" AS \"orders\"",
            "\"orders\".\"total\" > 100",
        ],
    );
}

#[test]
fn tables_of_the_same_name_stay_apart() {
    // A table in the current schema isn't the one in another schema, whatever their names.
    let error = compile_error("#customers $#/archive/customers");
    assert_eq!(error.message, "Table `/archive/customers` not found.");
    // Each has its own computed columns.
    assert_lines(
        "#/archive/orders.big = total:>100\n\
         #orders.big = total:>1000\n\
         #customers $#orders{big:@true} $#/archive/orders{big:@true}",
        &[
            "(\"orders\".\"total\" > 1000) = TRUE",
            "(\"orders\".\"total\" > 100) = TRUE",
        ],
    );
    assert_eq!(
        compile_error("#/archive/orders.x = 1\n#orders $x").message,
        "Column `x` not found within table `orders`."
    );
    // And its own custom comparisons.
    assert_lines(
        "#/archive/orders.costs:@x = total:@x\n#/archive/orders costs:5",
        &["\"orders\".\"total\" = 5;"],
    );
    assert!(compile("#/archive/orders.costs:@x = total:@x\n#orders costs:5").is_err());
}

#[test]
fn unions_pipelines_and_windows() {
    assert_lines(
        "#orders $id +++ #/archive/orders $id",
        &[
            "FROM \"orders\"",
            "UNION ALL",
            "FROM \"archive\".\"orders\" AS \"orders\";",
        ],
    );
    assert_lines(
        "#/archive/orders $customer \\g $%count -> n ~~~ n:>1",
        &["FROM \"archive\".\"orders\" AS \"orders\""],
    );
    // A window condition reads the base table from a CTE, under the table's own name.
    assert_lines(
        "#/archive/orders %%(customer\\p)%row_number:1 $id",
        &[
            "FROM \"archive\".\"orders\" AS \"orders\"",
            "FROM \"qdwin_src\" AS \"orders\"",
        ],
    );
    // A user-defined table is never in a schema.
    assert_lines(
        "#recent = #( #/archive/orders $id $total )\n#recent $id",
        &[
            "FROM \"archive\".\"orders\" AS \"orders\"",
            "FROM \"recent\";",
        ],
    );
    assert!(compile("#recent = #( #/archive/orders $id )\n#/archive/recent").is_err());
}

#[test]
fn quoting_follows_the_dialect() {
    let options = Options {
        dialect: Box::new(DuckDB()),
        identifier_resolution: IdentifierResolution::Flexible,
        limit: None,
        offset: None,
    };
    let schema = r#"{
      "tables": [{ "name": "t x", "schema": ["a \"b\"", "c"], "columns": [{ "name": "id" }] }],
      "links": []
    }"#;
    let compiler = Compiler::new(schema, options).unwrap();
    let sql = compiler
        .compile("#/`a \"b\"`/c/`t x`".to_string())
        .unwrap()
        .sql;
    assert!(
        sql.contains("FROM \"a \"\"b\"\"\".\"c\".\"t x\" AS \"t x\""),
        "{sql}"
    );
}

#[test]
fn errors_name_tables_with_their_schemas() {
    assert_eq!(
        compile_error("#/archive/nope").message,
        "Base table `/archive/nope` does not exist."
    );
    assert_eq!(
        compile_error("#/archive/orders $nope").message,
        "Column `nope` not found within table `/archive/orders`."
    );
    assert_eq!(
        compile_error("#/archive/nope.x = 1\n#orders").message,
        "Computed column refers to unknown table `/archive/nope`."
    );
    assert_eq!(
        compile_error("#/archive/nope.x:@v = id:@v\n#orders").message,
        "Custom comparison refers to unknown table `/archive/nope`."
    );
    assert_eq!(
        compile_error("#/archive/orders $#/warehouse/main/shipments(id)").message,
        "The table `/warehouse/main/shipments` is not related to the table `/archive/orders` via \
         a foreign key column named `id`, so `#/warehouse/main/shipments(id)` can't be used here."
    );
}

#[test]
fn a_table_missing_from_its_schema_is_found_in_another() {
    // A table that's only in another schema is suggested, with its schema.
    let error = compile_error("#shipments");
    assert_eq!(error.message, "Base table `shipments` does not exist.");
    assert_eq!(
        error.hint.as_deref(),
        Some(
            "A table of that name is in another schema, which you can name before it, as in \
             `#/warehouse/main/shipments`."
        )
    );
    let error = compile_error("#customers ++#/warehouse/orders");
    assert_eq!(error.message, "Table `/warehouse/orders` not found.");
    assert_eq!(
        error.hint.as_deref(),
        Some(
            "A table of that name is in another schema, which you can name before it, as in \
             `#/archive/orders` or `#orders`."
        )
    );
    // There's nothing to suggest for a table that's nowhere.
    assert_eq!(compile_error("#nope").hint, None);
}

#[test]
fn schema_json_links_name_tables_with_their_schemas() {
    let compiler = |schema: &str| {
        let options = Options {
            dialect: Box::new(Postgres()),
            identifier_resolution: IdentifierResolution::Flexible,
            limit: None,
            offset: None,
        };
        Compiler::new(schema, options)
    };
    // A link's table is looked up in its schema, exactly.
    let error = compiler(
        r#"{
          "tables": [{ "name": "t", "schema": "s", "columns": [{ "name": "id" }] }],
          "links": [{ "from": { "table": "t", "column": "id" }, "to": { "table": "t", "schema": "s", "column": "id" }, "unique": false }]
        }"#,
    )
    .err()
    .unwrap();
    assert_eq!(
        error.message,
        "The schema has a link involving the table `t`, which is not among the schema's tables."
    );
    let error = compiler(
        r#"{
          "tables": [{ "name": "t", "schema": "s", "columns": [{ "name": "id" }] }],
          "links": [{ "from": { "table": "t", "schema": "s", "column": "x" }, "to": { "table": "t", "schema": "s", "column": "id" }, "unique": false }]
        }"#,
    )
    .err()
    .unwrap();
    assert_eq!(
        error.message,
        "The schema has a link involving the column `x`, which is not among the columns of the \
         table `/s/t`."
    );
    // The schema can be written as a string or a list, and an empty one is the current schema.
    let compiler = compiler(
        r#"{
          "tables": [
            { "name": "a", "schema": "", "columns": [{ "name": "id" }] },
            { "name": "b", "schema": [], "columns": [{ "name": "id" }, { "name": "a" }] }
          ],
          "links": [{ "from": { "table": "b", "schema": null, "column": "a" }, "to": { "table": "a", "column": "id" }, "unique": false }]
        }"#,
    )
    .unwrap();
    assert!(compiler.compile("#b $a.id".to_string()).is_ok());
}
