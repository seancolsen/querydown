use chumsky::Parser;

use crate::{
    compiling::sorting::SortingStack,
    converters::{convert_condition_set, convert_join_tree},
    parsing::query::query,
    rendering::{Render, RenderingContext},
    schema::{primitive_schema::PrimitiveSchema, schema::Schema},
    sql_tree::{Column, Select, Simplify},
    Options,
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
        let mut query = query()
            .parse(input)
            // TODO_ERR improve error handling
            .map_err(|_| "Invalid querydown code".to_string())?;
        let base_table_name = std::mem::take(&mut query.base_table);
        let mut cx = RenderingContext::build(&self.options, &self.schema, &base_table_name)?;
        let mut select = Select::from(cx.get_base_table().name.clone());
        let mut transformations_iter = query.transformations.into_iter();
        let first_transformation = transformations_iter.next().unwrap_or_default();
        let second_transformation = transformations_iter.next();
        if second_transformation.is_some() {
            return Err("Pipelines not yet supported".to_string());
        }

        select.condition_set = convert_condition_set(&first_transformation.condition_set, &mut cx);

        let mut sorting_stack = SortingStack::new();

        for column_spec in first_transformation.column_layout.column_specs {
            let expression = column_spec.expression.render(&mut cx);
            let alias = column_spec.alias;
            if let Some(sort_spec) = column_spec.column_control.sort {
                let expr = alias
                    .as_ref()
                    .map(|a| cx.options.dialect.quote_identifier(a))
                    .unwrap_or_else(|| expression.clone());
                sorting_stack.push(expr, sort_spec);
            }
            select.columns.push(Column { expression, alias });
        }

        select.sorting = sorting_stack.into();

        (select.joins, select.ctes) = convert_join_tree(cx.take_join_tree(), &cx);

        select.simplify();
        let mut result = select.render(&mut cx);
        result.push(';');
        Ok(result)
    }
}
