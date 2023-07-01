use querydown_parser::parse;

use crate::{
    schema::{primitive_schema::PrimitiveSchema, Schema},
    sql::tree::Select,
    Options,
};

use super::{
    expr::convert_condition_set, rendering::Render, result_columns::convert_result_columns,
    scope::Scope,
};

pub struct Compiler {
    options: Options,
    schema: Schema,
}

impl Compiler {
    pub fn new(schema_json: &str, options: Options) -> Result<Self, String> {
        let primitive_schema = serde_json::from_str::<PrimitiveSchema>(schema_json)
            .map_err(|_| "Schema input is not valid JSON.")?;
        let schema = Schema::try_from(primitive_schema)?;
        Ok(Self { options, schema })
    }

    pub fn compile(&self, input: String) -> Result<String, String> {
        let query = parse(&input)?;
        let mut scope = Scope::build(&self.options, &self.schema, &query.base_table)?;
        let mut select = Select::from(scope.get_base_table().name.clone());

        let mut transformations_iter = query.transformations.into_iter();
        let first_transformation = transformations_iter.next().unwrap_or_default();
        let second_transformation = transformations_iter.next();
        if second_transformation.is_some() {
            return Err("Pipelines not yet supported".to_string());
        }

        select.conditions = convert_condition_set(first_transformation.conditions, &mut scope)?;

        let result_columns = first_transformation.result_columns;
        (select.columns, select.sorting) = convert_result_columns(result_columns, &mut scope)?;

        (select.joins, select.ctes) = scope.decompose_join_tree();

        Ok(format!("{};", select.render(&mut scope)))
    }
}
