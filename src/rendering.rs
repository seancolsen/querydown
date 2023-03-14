use std::collections::{HashMap, HashSet};

use crate::{
    converters::simplify_expression,
    dialects::dialect::Dialect,
    schema::{
        chain::Chain,
        links::LinkToOne,
        schema::{Schema, Table},
    },
    sql_tree::Cte,
    syntax_tree::{Composition, Conjunction, Expression, Literal, Operator},
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
pub struct SimpleExpression {
    pub base: Literal,
    pub compositions: Vec<Composition>,
}

#[derive(Debug)]
pub struct JoinTree {
    alias: String,
    dependents: HashMap<LinkToOne, JoinTree>,
    ctes: Vec<Cte>,
}

impl JoinTree {
    pub fn new(alias: String) -> Self {
        Self {
            alias,
            dependents: HashMap::new(),
            ctes: Vec::new(),
        }
    }

    pub fn get_alias(&self) -> &str {
        &self.alias
    }

    pub fn get_dependents(&self) -> &HashMap<LinkToOne, JoinTree> {
        &self.dependents
    }

    pub fn integrate_chain(
        &mut self,
        chain_to_one: &Chain<LinkToOne>,
        mut get_alias: impl FnMut(&LinkToOne) -> String,
    ) -> String {
        let (next_link, remainder_chain_opt) = chain_to_one.with_first_link_broken_off();
        let subtree_opt = self.dependents.get_mut(next_link);
        match (subtree_opt, remainder_chain_opt) {
            // We have one more new link to add to the tree and then we're done. We add an empty
            // subtree and return its alias.
            (None, None) => {
                // TODO It seems like we can probably merge the code in the match arm with the
                // code in the next match arm
                let alias = get_alias(next_link);
                let subtree = JoinTree::new(alias.clone());
                self.dependents.insert(*next_link, subtree);
                alias
            }

            // We have multiple new links to add to the tree. We build a full subtree and return
            // the alias of its furthest child.
            (None, Some(remainder_chain)) => {
                let mut final_alias = String::new();
                let mut dependents = HashMap::<LinkToOne, JoinTree>::new();
                let links = remainder_chain.get_links().to_vec();
                for (index, link) in links.into_iter().rev().enumerate() {
                    let alias = get_alias(&link);
                    if index == 0 {
                        final_alias = alias.clone();
                    }
                    let subtree = JoinTree {
                        alias,
                        dependents: std::mem::take(&mut dependents),
                        ctes: Vec::new(),
                    };
                    dependents.insert(link, subtree);
                }
                let subtree = JoinTree {
                    alias: get_alias(next_link),
                    dependents,
                    ctes: Vec::new(),
                };
                self.dependents.insert(*next_link, subtree);
                final_alias
            }

            // We have a complete match for all links. We return the alias of the matching tree.
            (Some(subtree), None) => subtree.alias.clone(),

            // We need to continue matching the chain to the tree
            (Some(subtree), Some(remainder_chain)) => {
                subtree.integrate_chain(&remainder_chain, get_alias)
            }
        }
    }
}

pub struct RenderingContext<'a, D: Dialect> {
    pub dialect: &'a D,
    pub schema: &'a Schema,
    base_table: &'a Table,
    indentation_level: usize,
    join_tree: JoinTree,
    aliases: HashSet<String>,
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
            base_table,
            indentation_level: 0,
            join_tree: JoinTree::new(base_table.name.to_owned()),
            aliases: HashSet::new(),
        })
    }

    pub fn get_base_table(&self) -> &Table {
        self.base_table
    }

    pub fn get_join_tree(&self) -> &JoinTree {
        &self.join_tree
    }

    pub fn get_indentation(&self) -> String {
        INDENT_SPACER.repeat(self.indentation_level)
    }

    pub fn indented<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.indentation_level = self.indentation_level.saturating_add(1);
        let result = f(self);
        self.indentation_level = self.indentation_level.saturating_sub(1);
        result
    }

    pub fn join_chain_to_one(&mut self, chain: &Chain<LinkToOne>) -> String {
        let mut aliases = std::mem::take(&mut self.aliases);
        let mut try_alias = |alias: &str| -> bool {
            if !aliases.contains(alias) {
                aliases.insert(alias.to_string());
                true
            } else {
                false
            }
        };
        let get_alias = |link: &LinkToOne| -> String {
            let ideal_alias = self.schema.get_ideal_alias_for_link_to_one(link);
            if try_alias(ideal_alias) {
                return ideal_alias.to_string();
            }
            let suffix_index: usize = 1;
            loop {
                let new_alias = format!("{}_{}", ideal_alias, suffix_index);
                if try_alias(&new_alias) {
                    return new_alias;
                }
            }
        };
        let alias = self.join_tree.integrate_chain(chain, get_alias);
        self.aliases = aliases;
        alias
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
            Literal::TableColumnReference(t, c) => cx.dialect.table_column(t, c),
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
    let simple_expr = simplify_expression(expr, cx);
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
            Operator::Match => "RLIKE".to_string(),
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