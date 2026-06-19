mod comparisons;
mod constants;
mod expr;
mod functions;
mod join_tree;
mod paths;
mod rendering;
mod result_columns;
mod scope;

use querydown_parser::ast::AnnotationValue;
use querydown_parser::parse;
use serde::Serialize;

use crate::{
    schema::{primitive_schema::PrimitiveSchema, Schema},
    sql::tree::Select,
    Options,
};

use self::{
    expr::convert_condition_set,
    rendering::Render,
    result_columns::{convert_result_columns, convert_sort_exprs, ConvertedResultColumns},
    scope::Scope,
};

/// The output of compiling Querydown code: the generated SQL plus column-level annotation.
///
/// `column_annotations` is positionally aligned with the columns of the result set — one entry per
/// output column, in order, with `null` for any column that has no annotation.
#[derive(Debug, Serialize)]
pub struct CompileResult {
    pub sql: String,
    #[serde(rename = "columnAnnotations")]
    pub column_annotations: Vec<Option<AnnotationValue>>,
}

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

    pub fn compile(&self, input: String) -> Result<CompileResult, String> {
        let query = parse(&input)?;
        let mut scope = Scope::build(&self.options, &self.schema, &query.base_table)?;
        scope.register_constants(query.constants);
        scope.register_computed_columns(query.computed_columns)?;
        let mut select = Select::from(scope.get_base_table().name.clone());

        let mut transformations_iter = query.transformations.into_iter();
        let first_transformation = transformations_iter.next().unwrap_or_default();
        let second_transformation = transformations_iter.next();
        if second_transformation.is_some() {
            return Err("Pipelines not yet supported".to_string());
        }

        select.conditions = convert_condition_set(first_transformation.conditions, &mut scope)?;

        let standalone_sorting = convert_sort_exprs(first_transformation.sorting, &mut scope)?;

        let result_columns = first_transformation.result_columns;
        let ConvertedResultColumns {
            columns,
            sorting: column_sorting,
            grouping,
            column_annotations,
        } = convert_result_columns(result_columns, &mut scope)?;
        select.columns = columns;
        select.grouping = grouping;
        // Standalone `\\` sorts take precedence over column `\s` sorts, hence they come first.
        select.sorting = standalone_sorting
            .into_iter()
            .chain(column_sorting)
            .collect();

        (select.joins, select.ctes) = scope.decompose_join_tree();

        Ok(CompileResult {
            sql: format!("{};", select.render(&mut scope)),
            column_annotations,
        })
    }
}
