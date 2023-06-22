use querydown_parser::parse;

use crate::{
    schema::{primitive_schema::PrimitiveSchema, Schema},
    sql::tree::{Column, Select, SqlExpr},
    Options,
};

use super::{
    expr::{convert_condition_set, convert_expr},
    rendering::Render,
    scope::Scope,
    sorting::SortingStack,
};

pub struct Compiler {
    options: Options,
    schema: Schema,
}

impl Compiler {
    pub fn new(schema_json: &str, options: Options) -> Result<Self, String> {
        let primitive_schema = serde_json::from_str::<PrimitiveSchema>(schema_json)
            .map_err(|_| "Schema input is not valid JSON.")?;
        let schema = Schema::try_from(primitive_schema)?;
        Ok(Self { options, schema })
    }

    pub fn compile(&self, input: String) -> Result<String, String> {
        let mut query = parse(&input)?;
        let base_table_name = std::mem::take(&mut query.base_table);
        let mut scope = Scope::build(&self.options, &self.schema, &base_table_name)?;
        let mut select = Select::from(scope.get_base_table().name.clone());
        let mut transformations_iter = query.transformations.into_iter();
        let first_transformation = transformations_iter.next().unwrap_or_default();
        let second_transformation = transformations_iter.next();
        if second_transformation.is_some() {
            return Err("Pipelines not yet supported".to_string());
        }

        select.conditions = convert_condition_set(first_transformation.conditions, &mut scope)?;

        let mut sorting_stack = SortingStack::new();

        for column_spec in first_transformation.column_layout.column_specs {
            let expr = convert_expr(column_spec.expr, &mut scope)?;
            let alias = column_spec.alias;
            if let Some(sort_spec) = column_spec.column_control.sort {
                let sorting_expr = alias
                    .as_ref()
                    .map(|a| SqlExpr::atom(scope.options.dialect.quote_identifier(a)))
                    .unwrap_or_else(|| expr.clone());
                sorting_stack.push(sorting_expr, sort_spec);
            }
            select.columns.push(Column { expr, alias });
        }

        select.sorting = sorting_stack.into();

        (select.joins, select.ctes) = scope.decompose_join_tree();

        let mut result = select.render(&mut scope);
        result.push(';');
        Ok(result)
    }
}
