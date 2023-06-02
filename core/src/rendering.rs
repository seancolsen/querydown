use std::collections::{HashMap, HashSet};

use crate::{
    constants::CTE_ALIAS_PREFIX,
    converters::{build_cte_select, simplify_expression, ValueViaCte},
    dialects::sql,
    schema::{
        chain::Chain,
        links::{FilteredLink, Link, LinkToOne},
        schema::{Column, Schema, Table},
    },
    sql_tree::{Cte, CtePurpose},
    syntax_tree::{Composition, Conjunction, Expression, Literal, Operator},
    utils::flex_map::FlexMap,
    Options,
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
    pub const MAX: &str = "max";
    pub const MIN: &str = "min";
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

    pub fn take_dependents(&mut self) -> HashMap<LinkToOne, JoinTree> {
        std::mem::take(&mut self.dependents)
    }

    pub fn take_ctes(&mut self) -> Vec<Cte> {
        std::mem::take(&mut self.ctes)
    }

    pub fn integrate_chain(
        &mut self,
        chain_to_one_opt: Option<&Chain<LinkToOne>>,
        mut get_alias: impl FnMut(&LinkToOne) -> String,
        mut cte_to_add: Option<Cte>,
    ) -> String {
        let Some(chain_to_one) = chain_to_one_opt else {
            self.ctes.extend(cte_to_add);
            return self.alias.clone();
        };
        let (next_link, remainder_chain_opt) = chain_to_one.with_first_link_broken_off();
        let subtree_opt = self.dependents.get_mut(next_link);
        match (subtree_opt, remainder_chain_opt) {
            // We have one more new link to add to the tree and then we're done. We add an empty
            // subtree and return its alias.
            (None, None) => {
                let alias = get_alias(next_link);
                let mut subtree = JoinTree::new(alias.clone());
                subtree.ctes.extend(cte_to_add);
                self.dependents.insert(*next_link, subtree);
                alias
            }

            // We have multiple new links to add to the tree. We build a full subtree and return
            // the alias of its furthest child.
            (None, Some(remainder_chain)) => {
                let mut alias_of_furthest_subtree = String::new();
                let mut dependents = HashMap::<LinkToOne, JoinTree>::new();
                let links = remainder_chain.get_links().to_vec();
                for (index, link) in links.into_iter().rev().enumerate() {
                    let alias = get_alias(&link);
                    if index == 0 {
                        alias_of_furthest_subtree = alias.clone();
                    }
                    let mut subtree = JoinTree {
                        alias,
                        dependents: std::mem::take(&mut dependents),
                        ctes: Vec::new(),
                    };
                    if let Some(cte) = std::mem::take(&mut cte_to_add) {
                        // Take the CTE out of `cte_to_add` and add it to the subtree. This will
                        // only succeed on the first iteration of the loop, similar to the logic
                        // for `alias_of_furthest_subtree`.
                        subtree.ctes.push(cte);
                    }
                    dependents.insert(link, subtree);
                }
                let subtree = JoinTree {
                    alias: get_alias(next_link),
                    dependents,
                    ctes: Vec::new(),
                };
                self.dependents.insert(*next_link, subtree);
                alias_of_furthest_subtree
            }

            // We have a complete match for all links. We return the alias of the matching tree.
            (Some(subtree), None) => {
                subtree.ctes.extend(cte_to_add);
                subtree.alias.clone()
            }

            // We need to continue matching the chain to the tree
            (Some(subtree), Some(remainder_chain)) => {
                subtree.integrate_chain(Some(&remainder_chain), get_alias, cte_to_add)
            }
        }
    }
}

fn get_table_by_name<'a>(options: &Options, schema: &'a Schema, name: &str) -> Option<&'a Table> {
    options
        .resolve_identifier(&schema.table_lookup, name)
        .map(|id| schema.tables.get(id).unwrap())
}

fn get_column_by_name<'a>(options: &Options, table: &'a Table, name: &str) -> Option<&'a Column> {
    options
        .resolve_identifier(&table.column_lookup, name)
        .map(|id| table.columns.get(id).unwrap())
}

pub struct RenderingContext<'a> {
    pub options: &'a Options,
    pub schema: &'a Schema,
    base_table: &'a Table,
    indentation_level: usize,
    join_tree: JoinTree,
    aliases: HashSet<String>,
    cte_naming_index: usize,
}

impl<'a> RenderingContext<'a> {
    pub fn build(
        options: &'a Options,
        schema: &'a Schema,
        base_table_name: &'a str,
    ) -> Result<Self, String> {
        let base_table = get_table_by_name(options, schema, base_table_name)
            .ok_or(format!("Base table `{}` does not exist.", base_table_name))?;
        Ok(Self {
            options,
            schema,
            base_table,
            indentation_level: 0,
            join_tree: JoinTree::new(base_table.name.to_owned()),
            aliases: HashSet::new(),
            cte_naming_index: 0,
        })
    }

    pub fn get_base_table(&self) -> &Table {
        self.base_table
    }

    pub fn take_join_tree(&mut self) -> JoinTree {
        std::mem::replace(
            &mut self.join_tree,
            JoinTree::new(self.base_table.name.to_owned()),
        )
    }

    pub fn get_indentation(&self) -> String {
        INDENT_SPACER.repeat(self.indentation_level)
    }

    pub fn get_indentation_level(&self) -> usize {
        self.indentation_level
    }

    pub fn indented<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        self.indentation_level = self.indentation_level.saturating_add(1);
        let result = f(self);
        self.indentation_level = self.indentation_level.saturating_sub(1);
        result
    }

    pub fn spawn(&self, base_table: &'a Table) -> Self {
        RenderingContext {
            options: self.options,
            schema: self.schema,
            base_table,
            indentation_level: self.get_indentation_level() + 1,
            join_tree: JoinTree::new(base_table.name.to_owned()),
            aliases: HashSet::new(),
            cte_naming_index: 0,
        }
    }

    /// Returns a table alias that is unique within the context of the query.
    fn integrate_chain(&mut self, chain: Option<&Chain<LinkToOne>>, cte: Option<Cte>) -> String {
        // TODO figure out how to reduce code duplication between the logic here and
        // RenderingContext.get_alias. There are some borrowing issues with using the get_alias
        // method here. Need to find a way to structure this code so that both use-cases can share
        // it.
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
        let alias = self.join_tree.integrate_chain(chain, get_alias, cte);
        self.aliases = aliases;
        alias
    }

    pub fn join_chain_to_one(&mut self, chain: &Chain<LinkToOne>) -> String {
        self.integrate_chain(Some(chain), None)
    }

    pub fn get_alias(&mut self, ideal_alias: &str) -> String {
        let mut suffix_index: usize = 0;
        loop {
            let alias = if suffix_index == 0 {
                ideal_alias.to_owned()
            } else {
                format!("{}_{}", ideal_alias, suffix_index)
            };
            if !self.aliases.contains(&alias) {
                self.aliases.insert(alias.clone());
                return alias;
            }
            suffix_index += 1;
        }
    }

    pub fn join_chain_to_many(
        &mut self,
        head: &Option<Chain<LinkToOne>>,
        chain: Chain<FilteredLink>,
        final_column_name: Option<String>,
        compositions: Vec<Composition>,
        purpose: CtePurpose,
    ) -> Result<SimpleExpression, String> {
        let starting_reference = chain.get_first_link().get_start();
        let starting_table_id = starting_reference.table_id;
        let starting_column_id = starting_reference.column_id;
        let starting_table = self.schema.tables.get(&starting_table_id).unwrap();
        let starting_column = starting_table.columns.get(&starting_column_id).unwrap();
        let ValueViaCte {
            select,
            value_alias,
            compositions: leftover_compositions,
        } = build_cte_select(chain, final_column_name, compositions, self, purpose)?;
        let cte_alias = self.get_cte_alias();
        let cte = Cte {
            select,
            alias: cte_alias.clone(),
            purpose,
            join_column_name: starting_column.name.clone(),
        };
        self.integrate_chain(head.as_ref(), Some(cte));
        Ok(SimpleExpression {
            base: Literal::TableColumnReference(cte_alias, value_alias),
            compositions: leftover_compositions,
        })
    }

    fn get_cte_alias(&mut self) -> String {
        loop {
            let alias = format!("{}{}", CTE_ALIAS_PREFIX, self.cte_naming_index);
            self.cte_naming_index += 1;
            if !self.aliases.contains(&alias) {
                self.aliases.insert(alias.clone());
                return alias;
            }
        }
    }

    pub fn get_table_by_name(&self, name: &str) -> Option<&Table> {
        get_table_by_name(self.options, self.schema, name)
    }

    pub fn get_column_by_name<'b>(&self, table: &'b Table, name: &str) -> Option<&'b Column> {
        get_column_by_name(self.options, table, name)
    }
}

pub trait Render {
    fn render(&self, cx: &mut RenderingContext) -> String;
}

impl Render for Literal {
    fn render(&self, cx: &mut RenderingContext) -> String {
        match self {
            Literal::Date(d) => cx.options.dialect.date(d),
            Literal::Duration(d) => cx.options.dialect.duration(d),
            Literal::False => sql::FALSE.to_string(),
            Literal::Infinity => sql::INFINITY.to_string(),
            Literal::Now => sql::NOW.to_string(),
            Literal::Null => sql::NULL.to_string(),
            Literal::Number(n) => n.clone(),
            Literal::String(s) => cx.options.dialect.quote_string(s),
            Literal::True => sql::TRUE.to_string(),
            Literal::TableColumnReference(t, c) => cx.options.dialect.table_column(t, c),
        }
    }
}

fn render_composition(
    function_name: &str,
    base: &str,
    arg: Option<String>,
    _: &mut RenderingContext,
) -> String {
    let operator = |o: &'static str| match &arg {
        None => base.to_owned(),
        Some(a) => format!("{} {} {}", base, o, a),
    };
    let sql_fn = |f: &'static str| match &arg {
        None => format!("{}({})", f, base),
        Some(a) => format!("{}({}, {})", f, base, a),
    };
    match function_name {
        PLUS => operator(sql::PLUS),
        MINUS => operator(sql::MINUS),
        TIMES => operator(sql::TIMES),
        DIVIDE => operator(sql::DIVIDE),
        AGO => format!("({} {} {})", sql::NOW, sql::MINUS, base),
        FROM_NOW => format!("({} {} {})", sql::NOW, sql::PLUS, base),
        MAX => sql_fn(sql::MAX),
        MIN => sql_fn(sql::MIN),
        // TODO give error here instead of falling through
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

fn render_expression(expr: &Expression, cx: &mut RenderingContext) -> ExpressionRenderingOutput {
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
    fn render(&self, cx: &mut RenderingContext) -> String {
        render_expression(self, cx).rendered
    }
}

impl Render for Operator {
    fn render(&self, _: &mut RenderingContext) -> String {
        match self {
            Operator::Eq => sql::EQ.to_string(),
            Operator::Gt => sql::GT.to_string(),
            Operator::Gte => sql::GTE.to_string(),
            Operator::Lt => sql::LT.to_string(),
            Operator::Lte => sql::LTE.to_string(),
            Operator::Like => sql::LIKE.to_string(),
            Operator::Neq => sql::NEQ.to_string(),
            Operator::NLike => sql::NOT_LIKE.to_string(),
            Operator::Match => sql::RLIKE.to_string(),
            Operator::NRLike => sql::NOT_RLIKE.to_string(),
        }
    }
}

impl Render for Conjunction {
    fn render(&self, _: &mut RenderingContext) -> String {
        match self {
            Conjunction::And => sql::AND.to_string(),
            Conjunction::Or => sql::OR.to_string(),
        }
    }
}
