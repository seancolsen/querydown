pub mod ast;
mod compiler;
mod context;
mod error;
mod parsing;
mod tests;
mod tokens;

use chumsky::Parser;

use crate::{
    compiler::compile,
    context::{Context, Engine},
    parsing::query::query,
    tests::mocks::library_schema::library_schema,
};
use std::io::{self, Read};

fn main() -> () {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let context = Context {
        engine: Engine::Postgres,
        schema: library_schema(),
    };
    let ast = query().parse(input).unwrap();
    println!("{:#?}", ast);
    let sql = compile(ast, context);
    println!("{}", sql);
}
