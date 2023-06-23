use itertools::Itertools;
use querydown_parser::ast::{NullsSort, SortDirection};

use crate::{compiler::scope::Scope, sql::tree::*};

use super::constants::INDENT_SPACER;

pub trait Render {
    fn render(&self, scope: &mut Scope) -> String;
}

impl Render for SqlExpr {
    fn render(&self, _: &mut Scope) -> String {
        self.to_string()
    }
}

impl Render for Select {
    fn render(&self, scope: &mut Scope) -> String {
        let base_table_name = scope.options.dialect.quote_identifier(&self.base_table);

        let select = "SELECT".to_string();
        let columns = indent(self.columns.render(scope));
        let from = format!("FROM {}", base_table_name);
        let joins = self.joins.render(scope);

        let ctes = self.ctes.render(scope);
        let main = [select, columns, from, joins]
            .into_iter()
            .filter(|s| !s.is_empty())
            .join("\n");
        let where_ = if self.conditions.is_empty() {
            String::new()
        } else {
            let conditions = indent(self.conditions.render(scope));
            format!("WHERE\n{conditions}")
        };
        let group = if self.grouping.is_empty() {
            String::new()
        } else {
            let grouping = self
                .grouping
                .iter()
                .map(|g| g.render(scope))
                .filter(|s| !s.is_empty())
                .join(", ");
            format!("GROUP BY {grouping}")
        };
        let order = if self.sorting.is_empty() {
            String::new()
        } else {
            let sorting = indent(self.sorting.render(scope));
            format!("ORDER BY\n{sorting}")
        };
        [ctes, main, where_, group, order]
            .into_iter()
            .filter(|s| !s.is_empty())
            .join("\n")
    }
}

impl Render for Vec<Cte> {
    fn render(&self, scope: &mut Scope) -> String {
        if self.len() == 0 {
            return String::new();
        }
        let ctes = indent(
            self.iter()
                .map(|cte| cte.render(scope))
                .filter(|s| !s.is_empty())
                .join(",\n"),
        );
        format!("WITH\n{ctes}")
    }
}

impl Render for Cte {
    fn render(&self, scope: &mut Scope) -> String {
        let alias = scope.options.dialect.quote_identifier(&self.alias);
        let select = indent(self.select.render(scope));
        format!("{alias} AS (\n{select}\n)")
    }
}

impl Render for Vec<Join> {
    fn render(&self, scope: &mut Scope) -> String {
        self.iter()
            .map(|j| j.render(scope))
            .filter(|s| !s.is_empty())
            .join("\n")
    }
}

impl Render for Join {
    fn render(&self, scope: &mut Scope) -> String {
        let quoted_table = scope.options.dialect.quote_identifier(&self.table);
        let table_expr = if self.alias == self.table {
            quoted_table
        } else {
            let quoted_alias = scope.options.dialect.quote_identifier(&self.alias);
            format!("{} AS {}", quoted_table, quoted_alias)
        };
        let condition_set = indent(self.conditions.render(scope));
        let join_type = match self.join_type {
            JoinType::Inner => "JOIN",
            JoinType::LeftOuter => "LEFT JOIN",
        };
        format!("{join_type} {table_expr} ON\n{condition_set}")
    }
}

impl Render for Vec<Column> {
    fn render(&self, scope: &mut Scope) -> String {
        if self.len() == 0 {
            let base_table_name = scope
                .options
                .dialect
                .quote_identifier(&scope.get_base_table().name);
            format!("{base_table_name}.*")
        } else {
            self.iter()
                .map(|c| c.render(scope))
                .filter(|s| !s.is_empty())
                .join(",\n")
        }
    }
}

impl Render for Column {
    fn render(&self, scope: &mut Scope) -> String {
        let alias = self
            .alias
            .as_ref()
            .map(|a| scope.options.dialect.quote_identifier(a))
            .map(|alias| format!(" AS {}", alias))
            .unwrap_or_default();
        format!("{}{}", self.expr, alias)
    }
}

impl Render for Vec<SortEntry> {
    fn render(&self, scope: &mut Scope) -> String {
        self.iter()
            .map(|s| s.render(scope))
            .filter(|s| !s.is_empty())
            .join(",\n")
    }
}

impl Render for SortEntry {
    fn render(&self, _: &mut Scope) -> String {
        let direction = match self.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        let nulls_sort = match self.nulls_sort {
            NullsSort::First => "NULLS FIRST",
            NullsSort::Last => "NULLS LAST",
        };
        format!("{} {} {}", self.expr, direction, nulls_sort)
    }
}

fn indent(s: String) -> String {
    s.lines()
        .map(|line| format!("{}{}", INDENT_SPACER, line))
        .join("\n")
}
