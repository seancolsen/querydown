use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use super::path::path;

pub fn has_quantity<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, HasQuantity> {
    let quantity = choice((
        exactly(HAS_QUANTITY_AT_LEAST_ONE).to(Quantity::AtLeastOne),
        exactly(HAS_QUANTITY_ZERO).to(Quantity::Zero),
    ));
    quantity
        .then_ignore(pad())
        .then(path(expr))
        .map(|(quantity, path_parts)| HasQuantity {
            quantity,
            path_parts,
        })
}
