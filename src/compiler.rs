use chumsky::Parser;

use crate::{
    engines::engine::Engine,
    parsing::query::query,
    schema::{primitive_schema::PrimitiveSchema, schema::Schema},
    sql_tree::Select,
};

pub struct Compiler<E: Engine> {
    engine: E,
    schema: Schema,
}

impl<E: Engine> Compiler<E> {
    pub fn new(schema_json: &str, engine: E) -> Result<Self, String> {
        let primitive_schema = serde_json::from_str::<PrimitiveSchema>(schema_json)
            .map_err(|_| "Schema input is not valid JSON.")?;
        let schema = Schema::try_from(primitive_schema)?;
        Ok(Self { engine, schema })
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
        Ok(select.render(&self.engine))
    }
}
