use chumsky::prelude::*;

pub fn exactly(s: &str) -> impl Parser<char, String, Error = Simple<char>> + Clone {
    just(s.chars().collect::<Vec<char>>()).collect::<String>()
}
