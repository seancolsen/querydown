use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use super::path::path;

pub fn has_quantity(expr: impl Psr<Expr>) -> impl Psr<HasQuantity> {
    let quantity = choice((
        exactly(HAS_QUANTITY_AT_LEAST_ONE).to(Quantity::AtLeastOne),
        exactly(HAS_QUANTITY_ZERO).to(Quantity::Zero),
    ));
    quantity
        .then_ignore(whitespace())
        .then(path(expr))
        .map(|(quantity, path_parts)| HasQuantity {
            quantity,
            path_parts,
        })
}
