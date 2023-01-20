use crate::{context::Context, syntax_tree::Query};

pub fn compile(query: Query, context: Context) -> String {
    "SELECT * FROM todo;".to_string()
}
