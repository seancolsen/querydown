use crate::{
    compiling::{
        conversion::expressions::simplify_expression,
        scope::Scope,
        sql_tree::{
            Column, Cte, Join, JoinType, Select, SortEntry, SqlConditionSet, SqlConditionSetEntry,
        },
    },
    dialects::sql,
    syntax_tree::{Composition, Conjunction, Expression, Literal, NullsSort, SortDirection},
};

use super::functions::{self, render_composition};

pub trait Render {
    fn render(&self, scope: &mut Scope) -> String;
}

impl Render for Literal {
    fn render(&self, scope: &mut Scope) -> String {
        match self {
            Literal::Date(d) => scope.options.dialect.date(d),
            Literal::Duration(d) => scope.options.dialect.duration(d),
            Literal::False => sql::FALSE.to_string(),
            Literal::Infinity => sql::INFINITY.to_string(),
            Literal::Now => sql::NOW.to_string(),
            Literal::Null => sql::NULL.to_string(),
            Literal::Number(n) => n.clone(),
            Literal::String(s) => scope.options.dialect.quote_string(s),
            Literal::True => sql::TRUE.to_string(),
            Literal::TableColumnReference(t, c) => scope.options.dialect.table_column(t, c),
        }
    }
}

struct ExpressionRenderingOutput {
    rendered: String,
    last_applied_function: Option<String>,
}

fn needs_parens(outer_fn: &str, inner_fn: Option<&str>) -> bool {
    use functions::*;
    match inner_fn {
        None => false,
        Some(i) => (i == PLUS || i == MINUS) && (outer_fn == TIMES || outer_fn == DIVIDE),
    }
}

fn render_expression(expr: &Expression, scope: &mut Scope) -> ExpressionRenderingOutput {
    let simple_expr = simplify_expression(expr, scope);
    let mut rendered = simple_expr.base.render(scope);
    let mut last_composition: Option<&Composition> = None;
    for composition in simple_expr.compositions.iter() {
        let outer_fn = &composition.function.name;
        let argument = composition.argument.as_ref().map(|arg_expr| {
            let mut output = render_expression(arg_expr, scope);
            let inner_fn = output.last_applied_function.as_ref().map(|s| s.as_str());
            if needs_parens(outer_fn, inner_fn) {
                output.rendered = format!("({})", output.rendered);
            }
            output.rendered
        });
        if needs_parens(outer_fn, last_composition.map(|c| c.function.name.as_str())) {
            rendered = format!("({})", rendered);
        }
        rendered = render_composition(outer_fn, &rendered, argument, scope);
        last_composition = Some(composition);
    }
    ExpressionRenderingOutput {
        rendered,
        last_applied_function: last_composition.map(|c| c.function.name.clone()),
    }
}

impl Render for Expression {
    fn render(&self, scope: &mut Scope) -> String {
        render_expression(self, scope).rendered
    }
}

impl Render for Conjunction {
    fn render(&self, _: &mut Scope) -> String {
        match self {
            Conjunction::And => sql::AND.to_string(),
            Conjunction::Or => sql::OR.to_string(),
        }
    }
}

impl Render for Select {
    fn render(&self, scope: &mut Scope) -> String {
        let indentation = scope.get_indentation();
        let mut rendered = String::new();
        rendered.push_str(&self.ctes.render(scope));
        rendered.push_str(&indentation);
        rendered.push_str("SELECT\n");
        scope.indented(|scope| rendered.push_str(&self.columns.render(scope)));
        rendered.push_str(&indentation);
        rendered.push_str("FROM ");
        rendered.push_str(
            scope
                .options
                .dialect
                .quote_identifier(&self.base_table)
                .as_str(),
        );
        rendered.push_str(&self.joins.render(scope));
        if self.condition_set.entries.len() > 0 {
            rendered.push_str("\n");
            rendered.push_str(&indentation);
            rendered.push_str("WHERE\n");
            scope.indented(|scope| rendered.push_str(&self.condition_set.render(scope)))
        }
        if self.grouping.len() > 0 {
            rendered.push_str("\n");
            rendered.push_str(&indentation);
            rendered.push_str("GROUP BY ");
            rendered.push_str(&self.grouping.join(", "));
        }
        if self.sorting.len() > 0 {
            rendered.push_str("\n");
            rendered.push_str(&indentation);
            rendered.push_str("ORDER BY ");
            rendered.push_str(&self.sorting.render(scope));
        }
        rendered
    }
}

impl Render for Vec<Cte> {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        if self.len() == 0 {
            return rendered;
        }
        rendered.push_str("WITH ");
        let mut is_first = true;
        for cte in self {
            if !is_first {
                rendered.push_str(",\n");
            }
            rendered.push_str(&scope.options.dialect.quote_identifier(&cte.alias));
            rendered.push_str(" AS (\n");
            scope.indented(|scope| rendered.push_str(&cte.select.render(scope)));
            rendered.push_str("\n");
            rendered.push_str(")");
            is_first = false;
        }
        rendered.push_str("\n");
        rendered
    }
}

impl Render for Vec<Join> {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        for join in self.iter() {
            rendered.push_str("\n");
            rendered.push_str(scope.get_indentation().as_str());
            rendered.push_str(&join.render(scope));
        }
        rendered
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
        let condition_set = scope.indented(|scope| self.condition_set.render(scope));
        let join_type = match self.join_type {
            JoinType::Inner => "JOIN",
            JoinType::LeftOuter => "LEFT JOIN",
        };
        format!("{join_type} {table_expr} ON\n{condition_set}")
    }
}

impl Render for Vec<Column> {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        if self.len() == 0 {
            rendered.push_str(scope.get_indentation().as_str());
            rendered.push_str(
                &scope
                    .options
                    .dialect
                    .quote_identifier(&scope.get_base_table().name),
            );
            rendered.push_str(".*");
            rendered.push('\n');
            return rendered;
        }
        for (i, column) in self.iter().enumerate() {
            if i > 0 {
                rendered.push_str(",\n");
            }
            rendered.push_str(scope.get_indentation().as_str());
            rendered.push_str(&column.render(scope));
        }
        rendered.push('\n');
        rendered
    }
}

impl Render for Column {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        rendered.push_str(&self.expression);
        if let Some(alias) = &self.alias {
            rendered.push_str(" AS ");
            rendered.push_str(scope.options.dialect.quote_identifier(alias).as_str());
        }
        rendered
    }
}

impl Render for SqlConditionSet {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        if self.entries.len() == 0 {
            return rendered;
        }
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                rendered.push(' ');
                rendered.push_str(self.conjunction.render(scope).as_str());
                rendered.push('\n');
            }
            rendered.push_str(scope.get_indentation().as_str());
            rendered.push_str(&entry.render(scope));
        }
        rendered
    }
}

impl Render for SqlConditionSetEntry {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        match self {
            SqlConditionSetEntry::Expression(expression) => {
                rendered.push_str(expression.as_str());
            }
            SqlConditionSetEntry::ConditionSet(condition_set) => {
                if condition_set.entries.len() == 0 {
                    return rendered;
                }
                rendered.push_str("(\n");
                scope.indented(|scope| rendered.push_str(&condition_set.render(scope)));
                rendered.push('\n');
                rendered.push_str(scope.get_indentation().as_str());
                rendered.push_str(")\n");
            }
        }
        rendered
    }
}

impl Render for Vec<SortEntry> {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        for (i, entry) in self.iter().enumerate() {
            if i > 0 {
                rendered.push_str(", ");
            }
            rendered.push_str(&entry.render(scope));
        }
        rendered
    }
}

impl Render for SortEntry {
    fn render(&self, scope: &mut Scope) -> String {
        let mut rendered = String::new();
        rendered.push_str(&self.expression);
        rendered.push(' ');
        rendered.push_str(&self.direction.render(scope));
        rendered.push(' ');
        rendered.push_str(&self.nulls_sort.render(scope));
        rendered
    }
}

impl Render for SortDirection {
    fn render(&self, _: &mut Scope) -> String {
        match self {
            SortDirection::Asc => sql::ASC.to_string(),
            SortDirection::Desc => sql::DESC.to_string(),
        }
    }
}

impl Render for NullsSort {
    fn render(&self, _: &mut Scope) -> String {
        match self {
            NullsSort::First => sql::NULLS_FIRST.to_string(),
            NullsSort::Last => sql::NULLS_LAST.to_string(),
        }
    }
}
