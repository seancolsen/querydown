pub mod ast;
mod context;
mod error;
mod parsing;
mod tokens;

use chumsky::Parser;

use crate::parsing::query::query;
use std::io::{self, Read};

fn main() -> () {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    println!("{:#?}", query().parse(input));
}
