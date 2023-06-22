mod parser;

pub mod ast;
pub mod tokens;

use chumsky::Parser;
use parser::query;

pub fn parse(input: &str) -> Result<ast::Query, String> {
    query()
        .parse(input)
        // TODO_ERR improve error handling
        .map_err(|_| "Invalid querydown code".to_string())
}
