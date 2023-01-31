use crate::schema::schema::Schema;

#[derive(Debug)]
pub struct Context {
    pub engine: Engine,
    pub schema: Schema,
}

#[derive(Debug)]
pub enum Engine {
    Sqlite,
    Postgres,
    MySql,
}
