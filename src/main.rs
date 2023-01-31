mod compiler;
mod context;
mod error;
mod parsing;
mod schema;
mod sql_tree;
pub mod syntax_tree;
mod tests;
mod tokens;

use chumsky::Parser;

use crate::parsing::query::query;
use std::io::{self, Read};

fn main() -> () {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let ast = query().parse(input).unwrap();
    println!("{:#?}", ast);
}
