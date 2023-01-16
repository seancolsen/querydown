pub enum Engine {
    Sqlite,
    Postgres,
    MySql,
}

pub struct Context {
    pub engine: Engine,
}
