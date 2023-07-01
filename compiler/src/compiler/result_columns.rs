use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use querydown_parser::ast::*;

use crate::{
    compiler::paths::{clarify_path, ClarifiedPathTail},
    errors::msg,
    sql::{
        expr::SqlExpr,
        tree::{Column, SortEntry},
    },
};

use self::sorting::SortingStack;

use super::{expr::convert_expr, scope::Scope};

pub fn convert_result_columns(
    result_columns: Vec<ResultColumnStatement>,
    scope: &mut Scope,
) -> Result<(Vec<Column>, Vec<SortEntry>), String> {
    let mut columns = Vec::<Column>::new();
    let mut sorting_stack = SortingStack::new();
    for column_statement in result_columns {
        match column_statement {
            ResultColumnStatement::Spec(spec) => {
                handle_spec(spec, &mut columns, &mut sorting_stack, scope)?;
            }
            ResultColumnStatement::Glob(glob) => {
                handle_glob(glob, &mut columns, &mut sorting_stack, scope)?;
            }
        }
    }
    Ok((columns, sorting_stack.into()))
}

fn handle_spec(
    spec: ColumnSpec,
    columns: &mut Vec<Column>,
    sorting_stack: &mut SortingStack,
    scope: &mut Scope,
) -> Result<(), String> {
    let expr = convert_expr(spec.expr, scope)?;
    let alias = spec.alias;
    if let Some(sort_spec) = spec.column_control.sort {
        let sorting_expr = alias
            .as_ref()
            .map(|a| SqlExpr::atom(scope.options.dialect.quote_identifier(a)))
            .unwrap_or_else(|| expr.clone());
        sorting_stack.push(sorting_expr, sort_spec);
    }
    columns.push(Column { expr, alias });
    // TODO convert GroupSpec into GROUP BY
    Ok(())
}

fn handle_glob(
    glob: ColumnGlob,
    columns: &mut Vec<Column>,
    sorting_stack: &mut SortingStack,
    scope: &mut Scope,
) -> Result<(), String> {
    scope.with_path_prefix(glob.head.clone(), |scope| -> Result<(), String> {
        for spec in glob.specs.iter() {
            if let Some(ref sort_spec) = spec.column_control.sort {
                let sql_expr = convert_expr(spec.expr.clone(), scope)?;
                sorting_stack.push(sql_expr, sort_spec.to_owned());
            }
            // TODO convert GroupSpec into GROUP BY
        }
        Ok(())
    })?;

    let (table, table_alias) = if glob.head.len() == 0 {
        let base_table = scope.get_base_table();
        (scope.get_base_table(), base_table.name.clone())
    } else {
        let clarified_path = clarify_path(glob.head, scope)?;
        if let Some(tail) = clarified_path.tail {
            let err_msg = match tail {
                ClarifiedPathTail::Column(col) => msg::column_glob_after_non_fk_column(&col),
                ClarifiedPathTail::ChainToMany(_) => msg::column_glob_on_path_to_many(),
            };
            return Err(err_msg);
        }
        let Some(chain_to_one) = clarified_path.head else {
                return Err(msg::empty_path());
            };
        let table = scope
            .schema
            .tables
            .get(&chain_to_one.get_ending_table_id())
            .unwrap();
        let table_alias = scope.join_chain_to_one(&chain_to_one);
        (table, table_alias)
    };

    let mut hidden_columns: HashSet<usize> = HashSet::new();
    let mut column_aliases: HashMap<usize, String> = HashMap::new();

    for spec in glob.specs {
        if let Expr::Path(ref path) = spec.expr {
            if let Ok(first_path_part) = path.iter().exactly_one() {
                if let PathPart::Column(column_name) = first_path_part {
                    let column_id = scope
                        .options
                        .resolve_identifier(&table.column_lookup, &column_name)
                        .copied()
                        .ok_or_else(|| msg::col_not_in_table(&column_name, &table.name))?;
                    if spec.column_control.is_hidden {
                        hidden_columns.insert(column_id);
                    }
                    if let Some(alias) = spec.alias {
                        column_aliases.insert(column_id, alias);
                    }
                }
            }
        }
    }

    for column in table.columns.values().sorted_by_key(|c| c.id) {
        let expr = scope.table_column_expr(&table_alias, &column.name);
        let alias = column_aliases.get(&column.id).cloned();
        if !hidden_columns.contains(&column.id) {
            columns.push(Column { expr, alias });
        }
    }
    Ok(())
}

mod sorting {
    use querydown_parser::ast::SortSpec;

    use crate::sql::tree::{SortEntry, SqlExpr};

    pub struct UnplacedSortEntry {
        entry: SortEntry,
        ordinal: Option<u32>,
    }

    pub struct SortingStack {
        entries: Vec<UnplacedSortEntry>,
    }

    impl SortingStack {
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        pub fn push(&mut self, expr: SqlExpr, sort_spec: SortSpec) {
            let entry = UnplacedSortEntry {
                entry: SortEntry {
                    expr,
                    direction: sort_spec.direction,
                    nulls_sort: sort_spec.nulls_sort,
                },
                ordinal: sort_spec.ordinal,
            };
            self.entries.push(entry);
        }
    }

    impl From<SortingStack> for Vec<SortEntry> {
        fn from(stack: SortingStack) -> Self {
            let mut entries = stack.entries;
            let max_ordinal = entries.iter().filter_map(|e| e.ordinal).max().unwrap_or(0);
            entries.sort_by_key(|entry| entry.ordinal.unwrap_or(max_ordinal.saturating_add(1)));
            entries.into_iter().map(|entry| entry.entry).collect()
        }
    }
}
