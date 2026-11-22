pub mod chain;
pub mod links;
pub mod primitive_schema;

use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};

use querydown_parser::ast;

use crate::{errors::msg, Error, Options};

use self::{
    links::{
        ForeignKey, ForwardLinkToOne, Link, LinkToOne, MultiLink, Reference, ReverseLinkToMany,
        ReverseLinkToOne,
    },
    primitive_schema::{PrimitiveReference, PrimitiveSchema, PrimitiveTable},
};

pub type TableName = String;
pub type ColumnName = String;
pub type TableId = usize;
pub type ColumnId = usize;

#[derive(Debug)]
pub struct Schema {
    pub tables: HashMap<TableId, Table>,
    /// The tables by name: those of the current schema, and those of the other schemas within it.
    pub namespace: Namespace,
}

/// The tables of a schema, by name, along with the schemas nested within it. The root namespace
/// holds the tables of the current schema (written `#table`), and every other schema is nested
/// within it by its path (written `#/schema/table`).
#[derive(Debug, Default)]
pub struct Namespace {
    pub tables: HashMap<TableName, TableId>,
    pub schemas: HashMap<String, Namespace>,
}

impl Namespace {
    /// The namespace of the schema at `path` (relative to this one), creating it if need be.
    fn schema_mut(&mut self, path: &[String]) -> &mut Namespace {
        path.iter().fold(self, |namespace, name| {
            namespace.schemas.entry(name.clone()).or_default()
        })
    }

    /// The namespace of the schema at `path` (relative to this one), matching names exactly.
    fn schema(&self, path: &[String]) -> Option<&Namespace> {
        path.iter()
            .try_fold(self, |namespace, name| namespace.schemas.get(name))
    }
}

impl Schema {
    /// Finds the table that `name` refers to, matching each of its names (the schema's and the
    /// table's) with the identifier resolution of `options`.
    pub fn find_table(&self, options: &Options, name: &ast::TableName) -> Option<&Table> {
        let namespace = name
            .schema
            .iter()
            .try_fold(&self.namespace, |namespace, schema| {
                options.resolve_identifier(&namespace.schemas, schema)
            })?;
        options
            .resolve_identifier(&namespace.tables, &name.name)
            .map(|id| &self.tables[id])
    }

    /// Like [`Schema::find_table`], but when there's no such table, the error is `not_found` with a
    /// hint naming any tables of the same name in other schemas, which may be what was meant.
    pub fn find_table_or(
        &self,
        options: &Options,
        name: &ast::TableName,
        not_found: impl FnOnce() -> Error,
    ) -> Result<&Table, Error> {
        if let Some(table) = self.find_table(options, name) {
            return Ok(table);
        }
        let mut namesakes: Vec<String> = self
            .tables
            .values()
            .filter(|table| table.schema != name.schema)
            .filter(|table| {
                let lookup = HashMap::from([(table.name.clone(), ())]);
                options.resolve_identifier(&lookup, &name.name).is_some()
            })
            .map(|table| format!("`#{}`", table.written_name()))
            .collect();
        namesakes.sort();
        let error = not_found();
        if namesakes.is_empty() {
            return Err(error);
        }
        Err(error.with_hint(format!(
            "A table of that name is in another schema, which you can name before it, as in {}.",
            namesakes.join(" or ")
        )))
    }

    pub fn get_ideal_alias_for_link_to_one(&self, link: &LinkToOne) -> &str {
        // The `unwrap` calls within this fn are safe because we know all links within the schema
        // are valid.
        match link {
            LinkToOne::ForwardLinkToOne(forward_link) => {
                let target_table_id = forward_link.target.table_id;
                let base_table_id = forward_link.base.table_id;
                let base_table = self.tables.get(&base_table_id).unwrap();
                let links_which_point_to_the_same_target_table = base_table
                    .forward_links_to_one
                    .values()
                    .filter(|&l| l.target.table_id == target_table_id);
                // TODO_PERF: we don't need to consume the whole iterator here just to see if the
                // count is greater than 1. We can stop when we get a count of 2.
                let is_duplicate = links_which_point_to_the_same_target_table.count() > 1;
                if is_duplicate {
                    &base_table
                        .columns
                        .get(&forward_link.base.column_id)
                        .unwrap()
                        .name
                } else {
                    &self.tables.get(&target_table_id).unwrap().name
                }
            }
            LinkToOne::ReverseLinkToOne(reverse_link) => {
                let table_id = reverse_link.get_end().table_id;
                let table = self.tables.get(&table_id).unwrap();
                &table.name
            }
        }
    }

    pub fn get_referenced_column_name(&self, reference: &Reference) -> String {
        let table = self.tables.get(&reference.table_id).unwrap();
        let column = table.columns.get(&reference.column_id).unwrap();
        column.name.clone()
    }
}

#[derive(Debug)]
pub struct Table {
    pub id: TableId,
    pub name: TableName,
    /// The names of the schema the table is in, outermost first. Empty for a table in the current
    /// schema, and for a table that isn't in the database (e.g. the result of a pipeline stage).
    pub schema: Vec<String>,
    pub columns: HashMap<ColumnId, Column>,
    pub column_lookup: HashMap<ColumnName, ColumnId>,
    /// Keys are starting column ids in this table
    pub forward_links_to_one: HashMap<ColumnId, ForwardLinkToOne>,
    /// Keys are ending table ids in the other table
    pub reverse_links_to_one: HashMap<TableId, Vec<ReverseLinkToOne>>,
    /// Keys are ending table ids in the other table
    pub reverse_links_to_many: HashMap<TableId, Vec<ReverseLinkToMany>>,
}

impl Table {
    /// The table's name, including its schema: what the SQL refers to it by, and how it's written
    /// in Querydown (after a `#`).
    pub fn qualified_name(&self) -> ast::TableName {
        ast::TableName::in_schema(self.schema.clone(), self.name.clone())
    }

    /// The table's name as written in Querydown (without a `#`), for messages: `issues`, or
    /// `/sales/orders` for a table in another schema.
    pub fn written_name(&self) -> String {
        self.qualified_name().to_string()
    }

    pub fn get_links(&self) -> impl Iterator<Item = MultiLink> + '_ {
        let forward_links_to_one = self
            .forward_links_to_one
            .values()
            .copied()
            .map(MultiLink::ForwardLinkToOne);
        let reverse_links_to_many = self
            .reverse_links_to_many
            .values()
            .flatten()
            .copied()
            .map(MultiLink::ReverseLinkToMany);
        let reverse_links_to_one = self
            .reverse_links_to_one
            .values()
            .flatten()
            .copied()
            .map(MultiLink::ReverseLinkToOne);
        forward_links_to_one
            .chain(reverse_links_to_many)
            .chain(reverse_links_to_one)
    }
}

/// A coarse classification of a column's data type, used to decide how the match operator (`:`)
/// behaves. This is deliberately shallow: it only needs to distinguish the cases that change
/// behavior (currently text), and anything we can't confidently classify becomes
/// [`ValueType::Unknown`], which falls back to exact equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Text,
    Number,
    Date,
    /// A timestamp value. `has_tz` distinguishes a zoned timestamp (`timestamptz`, the type of
    /// `@now`) from a naive one (`timestamp`/`datetime`). This matters because some databases
    /// (notably DuckDB built without the ICU extension) can't perform arithmetic or comparison
    /// between a zoned and a naive timestamp without timezone data. See
    /// [`crate::compiler::temporal`].
    Time {
        has_tz: bool,
    },
    /// A length of time (SQL's `interval`), e.g. a duration literal like `1w`, or the difference
    /// between two timestamps.
    Duration,
    Boolean,
    Array(Box<ValueType>),
    Unknown,
}

impl ValueType {
    /// Parse a type from the free-form string used in the schema. A trailing `[]` denotes an array
    /// of the inner type. Unrecognized strings become [`ValueType::Unknown`] so that new database
    /// types don't break compilation.
    pub fn parse(raw: &str) -> ValueType {
        let trimmed = raw.trim();
        if let Some(inner) = trimmed.strip_suffix("[]") {
            return ValueType::Array(Box::new(ValueType::parse(inner)));
        }
        match trimmed.to_lowercase().as_str() {
            "text" | "varchar" | "char" | "character varying" | "bpchar" | "string" => {
                ValueType::Text
            }
            "integer" | "int" | "int4" | "int8" | "bigint" | "smallint" | "numeric" | "decimal"
            | "real" | "double precision" | "double" | "float" | "float4" | "float8" => {
                ValueType::Number
            }
            "date" => ValueType::Date,
            // Postgres introspection emits internal `typname`s (`timestamp`, `timestamptz`); DuckDB
            // introspection emits SQL-standard `data_type`s (`TIMESTAMP`, `TIMESTAMP WITH TIME
            // ZONE`). We recognize both spellings so a zoned column is classified as zoned.
            "timestamptz" | "timestamp with time zone" => ValueType::Time { has_tz: true },
            "timestamp"
            | "timestamp_ns"
            | "timestamp_ms"
            | "timestamp_s"
            | "datetime"
            | "timestamp without time zone" => ValueType::Time { has_tz: false },
            "interval" => ValueType::Duration,
            "boolean" | "bool" => ValueType::Boolean,
            _ => ValueType::Unknown,
        }
    }

    pub fn from_opt(raw: &Option<String>) -> ValueType {
        raw.as_deref()
            .map(ValueType::parse)
            .unwrap_or(ValueType::Unknown)
    }

    /// Whether this is a temporal type (a date or a timestamp): a point in time, as opposed to a
    /// [`ValueType::Duration`], which is a length of it.
    pub fn is_temporal(&self) -> bool {
        matches!(self, ValueType::Date | ValueType::Time { .. })
    }

    /// Whether this is a temporal type that carries a time zone. Only a zoned timestamp qualifies;
    /// a `date` or a naive timestamp does not.
    pub fn is_zoned(&self) -> bool {
        matches!(self, ValueType::Time { has_tz: true })
    }
}

#[derive(Debug)]
pub struct Column {
    pub id: ColumnId,
    pub name: ColumnName,
    pub r#type: ValueType,
    /// The column's data type exactly as the schema gives it (e.g. `"timestamptz"`), for display.
    pub type_name: Option<String>,
}

fn make_table(id: TableId, primitive_table: PrimitiveTable) -> Table {
    let mut columns = HashMap::<ColumnId, Column>::new();
    let mut max_column_id: ColumnId = 0;
    for primitive_column in primitive_table.columns {
        max_column_id += 1;
        let column = Column {
            id: max_column_id,
            name: primitive_column.name,
            r#type: ValueType::from_opt(&primitive_column.r#type),
            type_name: primitive_column.r#type,
        };
        columns.insert(max_column_id, column);
    }
    let column_lookup = columns
        .iter()
        .map(|(id, column)| (column.name.clone(), *id))
        .collect();
    Table {
        id,
        name: primitive_table.name,
        schema: primitive_table.schema.0,
        columns,
        column_lookup,
        forward_links_to_one: HashMap::new(),
        reverse_links_to_one: HashMap::new(),
        reverse_links_to_many: HashMap::new(),
    }
}

impl TryFrom<PrimitiveSchema> for Schema {
    type Error = Error;

    fn try_from(primitive_schema: PrimitiveSchema) -> Result<Schema, Error> {
        let mut max_table_id: TableId = 0;
        let mut tables = HashMap::<TableId, Table>::new();
        for primitive_table in primitive_schema.tables {
            max_table_id += 1;
            let table = make_table(max_table_id, primitive_table);
            tables.insert(max_table_id, table);
        }

        let mut namespace = Namespace::default();
        // In the order of the ids, so that of two tables with the same name, the last one listed
        // wins, whatever the order of the map.
        let mut ids: Vec<&TableId> = tables.keys().collect();
        ids.sort();
        for id in ids {
            let table = &tables[id];
            namespace
                .schema_mut(&table.schema)
                .tables
                .insert(table.name.clone(), *id);
        }

        let foreign_keys: Vec<ForeignKey> = {
            let get_table_by_name = |reference: &PrimitiveReference| -> Result<&Table, Error> {
                let table_id = namespace
                    .schema(&reference.schema.0)
                    .and_then(|schema| schema.tables.get(&reference.table))
                    .ok_or_else(|| {
                        let name = ast::TableName::in_schema(
                            reference.schema.0.clone(),
                            reference.table.clone(),
                        );
                        Error::schema(format!(
                            "The schema has a link involving the table `{name}`, which is not \
                             among the schema's tables."
                        ))
                    })?;
                let table = tables
                    .get(table_id)
                    .ok_or_else(|| Error::schema(format!("Table not found by id: {}", table_id)))?;
                Ok(table)
            };
            let get_column_id_by_name = |table: &Table, name: &String| -> Result<ColumnId, Error> {
                let column_id = table.column_lookup.get(name).ok_or_else(|| {
                    Error::schema(format!(
                        "The schema has a link involving the column `{name}`, which is not \
                                 among the columns of the table `{}`.",
                        table.written_name()
                    ))
                })?;
                Ok(*column_id)
            };
            let mut foreign_keys: Vec<ForeignKey> = vec![];
            for primitive_link in primitive_schema.links {
                let base_table = get_table_by_name(&primitive_link.from)?;
                let base_column_id =
                    get_column_id_by_name(base_table, &primitive_link.from.column)?;
                let target_table = get_table_by_name(&primitive_link.to)?;
                let target_column_id =
                    get_column_id_by_name(target_table, &primitive_link.to.column)?;
                foreign_keys.push(ForeignKey {
                    base: Reference {
                        table_id: base_table.id,
                        column_id: base_column_id,
                    },
                    target: Reference {
                        table_id: target_table.id,
                        column_id: target_column_id,
                    },
                    unique: primitive_link.unique,
                });
            }
            foreign_keys
        };

        for foreign_key in foreign_keys {
            let base = foreign_key.base;
            let target = foreign_key.target;

            let base_table = tables.get_mut(&base.table_id).unwrap();
            match base_table.forward_links_to_one.entry(base.column_id) {
                Occupied(_) => {
                    return Err(msg::multiple_fk_from_col());
                }
                Vacant(e) => {
                    e.insert(ForwardLinkToOne::from(foreign_key));
                }
            }

            let target_table = tables.get_mut(&target.table_id).unwrap();
            if foreign_key.unique {
                target_table
                    .reverse_links_to_one
                    .entry(base.table_id)
                    .or_default()
                    .push(ReverseLinkToOne::from(foreign_key))
            } else {
                target_table
                    .reverse_links_to_many
                    .entry(base.table_id)
                    .or_default()
                    .push(ReverseLinkToMany::from(foreign_key));
            }
        }

        Ok(Schema { tables, namespace })
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::get_test_resource;

    use super::*;

    #[test]
    fn test_schema_from_primitive_schema() {
        let primitive_schema: PrimitiveSchema =
            serde_json::from_str(&get_test_resource("issue_schema.json")).unwrap();
        let schema = Schema::try_from(primitive_schema);
        assert!(schema.is_ok())
    }
}
