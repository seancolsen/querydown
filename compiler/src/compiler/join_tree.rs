use std::collections::HashMap;

use crate::schema::links::{Link, Reference};
use crate::schema::{chain::Chain, links::LinkToOne};
use crate::sql::expr::build::*;
use crate::sql::tree::{Cte, Join, JoinType, SqlExpr};

use super::constants::CTE_PK_COLUMN_ALIAS;
use super::scope::Scope;

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

    /// Destroy this JoinTree and return the component parts needed to render a query.
    pub fn decompose(mut self, scope: &Scope) -> (Vec<Join>, Vec<Cte>) {
        let mut ctes = self.take_ctes();
        let mut joins: Vec<Join> = ctes
            .iter()
            .map(|cte| build_join_for_cte(cte, self.get_alias().to_owned(), scope))
            .collect();
        for (link, subtree) in self.take_dependents() {
            let starting_alias = self.get_alias();
            let ending_alias = subtree.get_alias();
            let join_type = JoinType::LeftOuter;
            let join = make_join_from_link(
                link.get_start(),
                starting_alias,
                link.get_end(),
                ending_alias,
                join_type,
                scope,
            );
            joins.push(join);
            let (new_joins, new_ctes) = subtree.decompose(scope);
            joins.extend(new_joins);
            ctes.extend(new_ctes);
        }
        (joins, ctes)
    }
}

fn build_join_for_cte(cte: &Cte, table: String, scope: &Scope) -> Join {
    Join {
        table: cte.alias.clone(),
        alias: cte.alias.clone(),
        conditions: cmp::eq(
            scope.table_column_expr(&table, &cte.join_column_name),
            scope.table_column_expr(&cte.alias, CTE_PK_COLUMN_ALIAS),
        ),
        join_type: JoinType::LeftOuter,
    }
}

pub fn make_join_from_link(
    start: Reference,
    starting_alias: &str,
    end: Reference,
    ending_alias: &str,
    join_type: JoinType,
    scope: &Scope,
) -> Join {
    let starting_table_id = start.table_id;
    let starting_table = scope.schema.tables.get(&starting_table_id).unwrap();
    let starting_column_id = start.column_id;
    let starting_column = starting_table.columns.get(&starting_column_id).unwrap();

    let ending_table_id = end.table_id;
    let ending_table = scope.schema.tables.get(&ending_table_id).unwrap();
    let ending_column_id = end.column_id;
    let ending_column = ending_table.columns.get(&ending_column_id).unwrap();

    Join {
        table: scope
            .schema
            .tables
            .get(&ending_table_id)
            .unwrap()
            .name
            .clone(),
        alias: ending_alias.to_owned(),
        conditions: cmp::eq(
            scope.table_column_expr(starting_alias, &starting_column.name),
            scope.table_column_expr(ending_alias, &ending_column.name),
        ),
        join_type,
    }
}
