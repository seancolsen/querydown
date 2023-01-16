use chumsky::prelude::*;

use crate::ast::*;

use super::values::value;

pub fn expression<C>(condition_set: C) -> impl Parser<char, Expression, Error = Simple<char>>
where
    C: Parser<char, ConditionSet, Error = Simple<char>> + Clone,
{
    value(condition_set).map(|base| Expression { base })
}
