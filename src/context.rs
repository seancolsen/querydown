use crate::schema::Schema;

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub engine: Engine,
    pub schema: Schema,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Engine {
    Sqlite,
    Postgres,
    MySql,
}
