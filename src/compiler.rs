use chumsky::Parser;

use crate::{
    converters::{convert_condition_set, convert_join_tree},
    dialects::dialect::Dialect,
    parsing::query::query,
    rendering::{Render, RenderingContext},
    schema::{primitive_schema::PrimitiveSchema, schema::Schema},
    sql_tree::{Column, Select, Simplify},
};

pub struct Compiler<D: Dialect> {
    dialect: D,
    schema: Schema,
}

impl<D: Dialect> Compiler<D> {
    pub fn new(schema_json: &str, dialect: D) -> Result<Self, String> {
        let primitive_schema = serde_json::from_str::<PrimitiveSchema>(schema_json)
            .map_err(|_| "Schema input is not valid JSON.")?;
        let schema = Schema::try_from(primitive_schema)?;
        Ok(Self { dialect, schema })
    }

    pub fn compile(&self, input: String) -> Result<String, String> {
        let mut query = query()
            .parse(input)
            // TODO_ERR improve error handling
            .map_err(|_| "Invalid querydown code".to_string())?;
        let base_table_name = std::mem::take(&mut query.base_table);
        let mut select = Select::from(base_table_name.clone());
        let mut cx = RenderingContext::build(&self.dialect, &self.schema, &base_table_name)?;
        let mut transformations_iter = query.transformations.into_iter();
        let first_transformation = transformations_iter.next().unwrap_or_default();
        let second_transformation = transformations_iter.next();
        if second_transformation.is_some() {
            return Err("Pipelines not yet supported".to_string());
        }

        select.condition_set = convert_condition_set(&first_transformation.condition_set, &mut cx);

        for column_spec in first_transformation.column_layout.column_specs {
            let expression = column_spec.expression.render(&mut cx);
            let alias = column_spec.alias;
            select.columns.push(Column { expression, alias });
        }

        (select.joins, select.ctes) = convert_join_tree(cx.take_join_tree(), &cx);

        select.simplify();
        let mut result = select.render(&mut cx);
        result.push(';');
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::dialects::postgres::Postgres;
    use crate::tests::test_utils::get_test_resource;
    use std::path::PathBuf;
    use testcase_markdown::*;

    use super::*;

    fn compile(input: &str) -> Result<String, String> {
        let schema_json = &get_test_resource("issue_schema.json");
        let compiler = Compiler::new(&schema_json, Postgres()).unwrap();
        compiler.compile(input.to_owned())
    }

    /// Removes spaces so that it's easy to compare two SQL strings without worrying about
    /// whitespace
    fn clean(s: String) -> String {
        s.replace("\n", "").replace("\t", "").replace(" ", "")
    }

    /// Run a full test case, expecting success
    fn run(input: &str, expected_result: &str) {
        let result = compile(input).unwrap();
        println!("{}", result);
        assert_eq!(clean(result), clean(expected_result.to_owned()));
    }

    #[test]
    fn test_issues_id() {
        run(
            "issues $id->id",
            r#"
            SELECT "issues"."id" AS "id" FROM "issues";
            "#,
        );
    }

    #[derive(Default, Clone)]
    struct Options();

    impl MergeSerialized for Options {
        fn merge_serialized(&self, source: String) -> Result<Self, String> {
            Ok(Options())
        }
    }

    #[test]
    fn test_corpus() {
        let path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), "src", "tests", "corpus.md"]);
        let content = std::fs::read_to_string(path).unwrap();
        let test_cases = get_test_cases(content, Options::default());
        for test_case in test_cases {
            println!("{}", test_case.args[0]);
            println!("{}", test_case.args[1]);
            run(&test_case.args[0], &test_case.args[1]);
        }
    }
}
