use crate::{ast::Query, context::Context};

pub fn compile(query: Query, context: Context) -> String {
    "SELECT * FROM todo;".to_string()
}
