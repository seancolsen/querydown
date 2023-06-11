use std::collections::HashMap;

use crate::{
    compiling::sql_tree::Cte,
    schema::{chain::Chain, links::LinkToOne},
};

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
