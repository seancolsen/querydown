use std::collections::{HashMap, HashSet};

use crate::{
    schema::{
        chain::Chain,
        links::{FilteredLink, Link, LinkToOne},
        Schema, Table,
    },
    sql::tree::{Cte, CtePurpose, Join, SqlExpr},
    Options,
};

use super::{
    constants::*,
    functions::{get_standard_aggregate_functions, get_standard_scalar_functions, Func, FuncMap},
    join_tree::JoinTree,
    paths::{build_cte_select, AggregateExprTemplate, ValueViaCte},
};

pub struct Scope<'a, 'b> {
    parent: Option<&'b Scope<'a, 'b>>,
    pub options: &'a Options,
    pub schema: &'a Schema,
    base_table: &'a Table,
    // indentation_level: usize,
    join_tree: JoinTree,
    aliases: HashSet<String>,
    cte_naming_index: usize,
    scalar_functions: FuncMap,
    aggregate_functions: FuncMap,
}

impl<'a, 'b> Scope<'a, 'b> {
    pub fn build(
        options: &'a Options,
        schema: &'a Schema,
        base_table_name: &'a str,
    ) -> Result<Self, String> {
        let base_table = get_table_by_name(options, schema, base_table_name)
            .ok_or(format!("Base table `{}` does not exist.", base_table_name))?;
        Ok(Self {
            parent: None,
            options,
            schema,
            base_table,
            // indentation_level: 0,
            join_tree: JoinTree::new(base_table.name.to_owned()),
            aliases: HashSet::new(),
            cte_naming_index: 0,
            scalar_functions: get_standard_scalar_functions(options, schema),
            aggregate_functions: get_standard_aggregate_functions(options, schema),
        })
    }

    pub fn get_base_table(&self) -> &Table {
        self.base_table
    }

    pub fn decompose_join_tree(&mut self) -> (Vec<Join>, Vec<Cte>) {
        let join_tree = std::mem::replace(
            &mut self.join_tree,
            JoinTree::new(self.base_table.name.to_owned()),
        );
        join_tree.decompose(self)
    }

    // pub fn get_indentation(&self) -> String {
    //     INDENT_SPACER.repeat(self.indentation_level)
    // }

    // pub fn get_indentation_level(&self) -> usize {
    //     self.indentation_level
    // }

    // pub fn indented<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
    //     self.indentation_level = self.indentation_level.saturating_add(1);
    //     let result = f(self);
    //     self.indentation_level = self.indentation_level.saturating_sub(1);
    //     result
    // }

    pub fn spawn(&'b self, base_table: &'a Table) -> Self {
        Scope {
            parent: Some(self),
            options: self.options,
            schema: self.schema,
            base_table,
            // indentation_level: self.get_indentation_level() + 1,
            join_tree: JoinTree::new(base_table.name.to_owned()),
            aliases: HashSet::new(),
            cte_naming_index: 0,
            scalar_functions: HashMap::new(),
            aggregate_functions: HashMap::new(),
        }
    }

    pub fn table_column_expr(&self, table_name: &str, column_name: &str) -> SqlExpr {
        SqlExpr::atom(self.options.dialect.table_column(table_name, column_name))
    }

    /// Returns a table alias that is unique within the context of the query.
    fn integrate_chain(&mut self, chain: Option<&Chain<LinkToOne>>, cte: Option<Cte>) -> String {
        // TODO figure out how to reduce code duplication between the logic here and
        // Scope::get_alias. There are some borrowing issues with using the get_alias method here.
        // Need to find a way to structure this code so that both use-cases can share it.
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
        aggregate_expr_template_opt: Option<AggregateExprTemplate>,
        purpose: CtePurpose,
    ) -> Result<SqlExpr, String> {
        let starting_reference = chain.get_first_link().get_start();
        let starting_table_id = starting_reference.table_id;
        let starting_column_id = starting_reference.column_id;
        let starting_table = self.schema.tables.get(&starting_table_id).unwrap();
        let starting_column = starting_table.columns.get(&starting_column_id).unwrap();
        let ValueViaCte {
            select,
            value_alias,
        } = build_cte_select(chain, aggregate_expr_template_opt, self, purpose)?;
        let cte_alias = self.get_cte_alias();
        let cte = Cte {
            select,
            alias: cte_alias.clone(),
            purpose,
            join_column_name: starting_column.name.clone(),
        };
        self.integrate_chain(head.as_ref(), Some(cte));
        Ok(self.table_column_expr(&cte_alias, &value_alias))
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

    pub fn get_scalar_function(&self, name: &str) -> Option<&Func> {
        self.scalar_functions.get(name).or_else(|| {
            self.parent
                .and_then(|parent| parent.get_scalar_function(name))
        })
    }

    pub fn get_aggregate_function(&self, name: &str) -> Option<&Func> {
        self.aggregate_functions.get(name).or_else(|| {
            self.parent
                .and_then(|parent| parent.get_aggregate_function(name))
        })
    }
}

fn get_table_by_name<'a>(options: &Options, schema: &'a Schema, name: &str) -> Option<&'a Table> {
    options
        .resolve_identifier(&schema.table_lookup, name)
        .map(|id| schema.tables.get(id).unwrap())
}
