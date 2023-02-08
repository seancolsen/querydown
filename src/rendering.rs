use crate::{
    dialects::dialect::Dialect,
    syntax_tree::{Composition, Expression, Value},
};

mod functions {
    pub const AGO: &str = "ago";
    pub const FROM_NOW: &str = "from_now";
    pub const MINUS: &str = "minus";
    pub const PLUS: &str = "plus";
    pub const TIMES: &str = "times";
    pub const DIVIDE: &str = "divide";
}

use functions::*;

pub struct RenderingContext<'a, D: Dialect> {
    pub dialect: &'a D,
}

pub trait Render {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String;
}

impl Render for Value {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        match self {
            Value::Date(d) => cx.dialect.date(d),
            Value::Duration(d) => cx.dialect.duration(d),
            Value::False => "FALSE".to_string(),
            Value::Infinity => "INFINITY".to_string(),
            Value::Now => "NOW()".to_string(),
            Value::Null => "NULL".to_string(),
            Value::Number(n) => n.clone(),
            Value::Path(_) => todo!(),
            Value::Slot => todo!(),
            Value::String(s) => cx.dialect.quote_string(s),
            Value::True => "TRUE".to_string(),
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
        AGO => format!("{} - {}", Value::Now.render(cx), base),
        FROM_NOW => format!("{} + {}", Value::Now.render(cx), base),
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
    let mut rendered = expr.base.render(cx);
    let mut last_composition: Option<&Composition> = None;
    for composition in expr.compositions.iter() {
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
