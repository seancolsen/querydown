use chumsky::Parser;

use crate::{
    dialects::dialect::Dialect,
    parsing::query::query,
    schema::{primitive_schema::PrimitiveSchema, schema::Schema},
    sql_tree::Select,
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
        let select = Select::from(base_table);
        Ok(select.render(&self.dialect))
    }
}
