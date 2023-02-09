use chumsky::Parser;

use crate::{
    dialects::dialect::Dialect,
    parsing::query::query,
    rendering::{Render, RenderingContext},
    schema::{primitive_schema::PrimitiveSchema, schema::Schema},
    sql_tree::{Column, Select},
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
            // TODO improve error handling
            .map_err(|_| "Invalid LQL".to_string())?;
        let base_table = std::mem::take(&mut query.base_table);
        if !self.schema.has_table(&base_table) {
            return Err(format!("Base table `{}` does not exist.", base_table));
        }
        let mut select = Select::from(base_table);
        let mut transformations_iter = query.transformations.into_iter();
        let first_transformation = transformations_iter.next().unwrap_or_default();
        let second_transformation = transformations_iter.next();
        if second_transformation.is_some() {
            return Err("Pipelines not yet supported".to_string());
        }
        let mut cx = RenderingContext::new(&self.dialect, &self.schema, &select.base_table);
        for column_spec in first_transformation.column_layout.column_specs {
            let expression = column_spec.expression.render(&mut cx);
            let alias = column_spec.alias;
            select.columns.push(Column { expression, alias });
        }
        let mut result = select.render(&mut cx);
        result.push(';');
        Ok(result)
    }
}
