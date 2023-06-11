use crate::{
    compiling::{scope::Scope, sql_tree::CtePurpose},
    syntax_tree::{Composition, Expression, Literal, PathPart, Value},
};

use super::paths::{clarify_path, ClarifiedPathTail};

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleExpression {
    pub base: Literal,
    pub compositions: Vec<Composition>,
}

pub fn simplify_expression(expr: &Expression, scope: &mut Scope) -> SimpleExpression {
    let expr = expr.clone();
    match expr.base {
        Value::Literal(literal) => SimpleExpression {
            base: literal,
            compositions: expr.compositions,
        },
        // TODO_ERR handle error
        Value::Path(path) => simplify_path_expression(path, expr.compositions, scope).unwrap(),
    }
}

fn simplify_path_expression(
    parts: Vec<PathPart>,
    compositions: Vec<Composition>,
    scope: &mut Scope,
) -> Result<SimpleExpression, String> {
    let clarified_path = clarify_path(parts, scope)?;
    match (clarified_path.head, clarified_path.tail) {
        (None, ClarifiedPathTail::Column(column_name)) => {
            let table_name = scope.get_base_table().name.clone();
            Ok(SimpleExpression {
                base: Literal::TableColumnReference(table_name, column_name),
                compositions,
            })
        }
        (Some(chain_to_one), ClarifiedPathTail::Column(column_name)) => {
            let table_name = scope.join_chain_to_one(&chain_to_one);
            Ok(SimpleExpression {
                base: Literal::TableColumnReference(table_name, column_name),
                compositions,
            })
        }
        (head, ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt))) => scope
            .join_chain_to_many(
                &head,
                chain_to_many,
                column_name_opt,
                compositions,
                CtePurpose::AggregateValue,
            ),
    }
}
