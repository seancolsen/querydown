use crate::{
    dialects::dialect::Dialect,
    schema::schema::Schema,
    sql_tree::{Cte, Join},
    syntax_tree::{Composition, Conjunction, Expression, Operator, Path, Value},
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

pub struct RenderingContext<'a, D: Dialect> {
    pub dialect: &'a D,
    pub schema: &'a Schema,
    pub base_table: &'a str,
    pub indentation_level: usize,
    slot_value: Option<ExpressionWithCachedRender>,
    ctes: Vec<Cte>,
    joins: Vec<Join>,
}

impl<'a, D: Dialect> RenderingContext<'a, D> {
    pub fn new(dialect: &'a D, schema: &'a Schema, base_table: &'a str) -> Self {
        Self {
            dialect,
            schema,
            base_table,
            indentation_level: 0,
            slot_value: None,
            ctes: vec![],
            joins: vec![],
        }
    }

    pub fn get_indentation(&self) -> String {
        INDENT_SPACER.repeat(self.indentation_level)
    }

    pub fn render_path(&mut self, path: &Path) -> String {
        // Reminder: need to look at the slot value within the `self` and see if it establishes a
        // new base table. If so we need to prepend `path` with the path from the slot value.
        "TODO".to_string()
    }

    pub fn with_slot_value<T>(&mut self, expr: Expression, f: impl FnOnce(&mut Self) -> T) -> T {
        // We render the slot value enclosed in parentheses as a sort of hacky way of dealing with
        // precedence issues that arise in Querydown code like this:
        //
        // ```qd
        // users "bar"|plus(2) ? {#|times(3) < 4}
        // ```
        //
        // If we don't use parentheses, then we end up with SQL `'bar' + 2 * 3 < 4` which has
        // incorrect precedence. We might want to fix this in a cleaner way by moving away from
        // pre-rendering the slot value and instead doing some more complex logic which merges the
        // slot expression with its surrounding expression.
        let new_slot_value =
            ExpressionWithCachedRender::new(format!("({})", expr.render(self)), expr);
        let old_slot_value = std::mem::replace(&mut self.slot_value, Some(new_slot_value));
        let result = f(self);
        self.slot_value = old_slot_value;
        result
    }

    pub fn indented<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.indentation_level = self.indentation_level.saturating_add(1);
        let result = f(self);
        self.indentation_level = self.indentation_level.saturating_sub(1);
        result
    }

    pub fn render_slot(&self) -> Option<String> {
        self.slot_value.as_ref().map(|i| i.render())
    }
}

struct ExpressionWithCachedRender {
    value: Expression,
    rendered: String,
}

impl ExpressionWithCachedRender {
    fn new(rendered: String, value: Expression) -> Self {
        Self { value, rendered }
    }

    fn render(&self) -> String {
        self.rendered.clone()
    }
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
            Value::Path(p) => cx.render_path(p),
            Value::Slot => cx.render_slot().unwrap(), // TODO handle error
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
