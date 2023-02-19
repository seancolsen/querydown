use crate::{
    converters::{combine_expression_with_slot, simplify_expression},
    dialects::dialect::Dialect,
    schema::schema::{Schema, Table},
    sql_tree::{Cte, Join, SqlConditionSetEntry},
    syntax_tree::{Composition, Conjunction, Expression, Literal, Operator, Path, Value},
};

/// We may eventually make this configurable
const INDENT_SPACER: &str = "  ";

mod functions {
    pub const AGO: &str = "ago";
    pub const FROM_NOW: &str = "from_now";
    pub const MINUS: &str = "minus";
    pub const PLUS: &str = "plus";
    pub const TIMES: &str = "times";
    pub const DIVIDE: &str = "divide";
}

use functions::*;

#[derive(Debug, Clone, PartialEq)]
pub struct DecontextualizedExpression {
    pub base: Value,
    pub compositions: Vec<Composition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleExpression {
    pub base: Literal,
    pub compositions: Vec<Composition>,
}

pub struct RenderingContext<'a, D: Dialect> {
    pub dialect: &'a D,
    pub schema: &'a Schema,
    base_table_name: &'a str,
    base_table: &'a Table,
    indentation_level: usize,
    slot_value: Option<DecontextualizedExpression>,
    ctes: Vec<Cte>,
    joins: Vec<Join>,
}

impl<'a, D: Dialect> RenderingContext<'a, D> {
    pub fn build(
        dialect: &'a D,
        schema: &'a Schema,
        base_table_name: &'a str,
    ) -> Result<Self, String> {
        let base_table = schema
            .get_table(base_table_name)
            .ok_or(format!("Base table `{}` does not exist.", base_table_name))?;
        Ok(Self {
            dialect,
            schema,
            base_table_name,
            base_table,
            indentation_level: 0,
            slot_value: None,
            ctes: vec![],
            joins: vec![],
        })
    }

    pub fn get_base_table_name(&self) -> &str {
        self.base_table_name
    }

    pub fn get_base_table(&self) -> &Table {
        self.base_table
    }

    pub fn get_indentation(&self) -> String {
        INDENT_SPACER.repeat(self.indentation_level)
    }

    pub fn with_slot_value<T>(
        &mut self,
        expr: DecontextualizedExpression,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let old_slot_value = std::mem::replace(&mut self.slot_value, Some(expr));
        // TODO_CODE we also need to set the base table in the case that a slot establishes a new
        // table context. If the last PathPart within `expr` is a FK column or a `TableWithOne`,
        // then we need to determine the name of the table, as joined, and set that as the
        // self.base_table.
        let result = f(self);
        self.slot_value = old_slot_value;
        result
    }

    pub fn get_slot_value(&self) -> Option<&DecontextualizedExpression> {
        self.slot_value.as_ref()
    }

    pub fn indented<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.indentation_level = self.indentation_level.saturating_add(1);
        let result = f(self);
        self.indentation_level = self.indentation_level.saturating_sub(1);
        result
    }
}

const SQL_NOW: &str = "NOW()";

pub trait Render {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String;
}

impl Render for Literal {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        match self {
            Literal::Date(d) => cx.dialect.date(d),
            Literal::Duration(d) => cx.dialect.duration(d),
            Literal::False => "FALSE".to_string(),
            Literal::Infinity => "INFINITY".to_string(),
            Literal::Now => SQL_NOW.to_string(),
            Literal::Null => "NULL".to_string(),
            Literal::Number(n) => n.clone(),
            Literal::String(s) => cx.dialect.quote_string(s),
            Literal::True => "TRUE".to_string(),
            Literal::TableColumnReference(table, column) => {
                let quoted_table = cx.dialect.quote_identifier(table);
                let quoted_column = cx.dialect.quote_identifier(column);
                format!("{}.{}", quoted_table, quoted_column)
            }
        }
    }
}

fn render_composition<D: Dialect>(
    function_name: &str,
    base: &str,
    arg: Option<String>,
    cx: &mut RenderingContext<D>,
) -> String {
    let operator = |o: &'static str| match &arg {
        None => base.to_owned(),
        Some(a) => format!("{} {} {}", base, o, a),
    };
    match function_name {
        PLUS => operator("+"),
        MINUS => operator("-"),
        TIMES => operator("*"),
        DIVIDE => operator("/"),
        AGO => format!("{} - {}", SQL_NOW.to_string(), base),
        FROM_NOW => format!("{} + {}", SQL_NOW.to_string(), base),
        _ => base.to_owned(),
    }
}

struct ExpressionRenderingOutput {
    rendered: String,
    last_applied_function: Option<String>,
}

fn needs_parens(outer_fn: &str, inner_fn: Option<&str>) -> bool {
    match inner_fn {
        None => false,
        Some(i) => (i == PLUS || i == MINUS) && (outer_fn == TIMES || outer_fn == DIVIDE),
    }
}

fn render_expression<D: Dialect>(
    expr: &Expression,
    cx: &mut RenderingContext<D>,
) -> ExpressionRenderingOutput {
    // TODO_ERR handle error when attempting to read an empty slot value
    let decontextualized_expr = combine_expression_with_slot(expr, cx).unwrap();
    let simple_expr = simplify_expression(decontextualized_expr, cx);
    let mut rendered = simple_expr.base.render(cx);
    let mut last_composition: Option<&Composition> = None;
    for composition in simple_expr.compositions.iter() {
        let outer_fn = &composition.function.name;
        let argument = composition.argument.as_ref().map(|arg_expr| {
            let mut output = render_expression(arg_expr, cx);
            let inner_fn = output.last_applied_function.as_ref().map(|s| s.as_str());
            if needs_parens(outer_fn, inner_fn) {
                output.rendered = format!("({})", output.rendered);
            }
            output.rendered
        });
        if needs_parens(outer_fn, last_composition.map(|c| c.function.name.as_str())) {
            rendered = format!("({})", rendered);
        }
        rendered = render_composition(outer_fn, &rendered, argument, cx);
        last_composition = Some(composition);
    }
    ExpressionRenderingOutput {
        rendered,
        last_applied_function: last_composition.map(|c| c.function.name.clone()),
    }
}

impl Render for Expression {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        render_expression(self, cx).rendered
    }
}

impl Render for Operator {
    fn render<D: Dialect>(&self, _: &mut RenderingContext<D>) -> String {
        match self {
            Operator::Eq => "=".to_string(),
            Operator::Gt => ">".to_string(),
            Operator::Gte => ">=".to_string(),
            Operator::Lt => "<".to_string(),
            Operator::Lte => "<=".to_string(),
            Operator::Like => "LIKE".to_string(),
            Operator::Neq => "<>".to_string(),
            Operator::NLike => "NOT LIKE".to_string(),
            Operator::RLike => "RLIKE".to_string(),
            Operator::NRLike => "NOT RLIKE".to_string(),
        }
    }
}

impl Render for Conjunction {
    fn render<D: Dialect>(&self, _: &mut RenderingContext<D>) -> String {
        match self {
            Conjunction::And => "AND".to_string(),
            Conjunction::Or => "OR".to_string(),
        }
    }
}
