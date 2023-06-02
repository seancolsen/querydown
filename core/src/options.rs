use crate::dialects::dialect::Dialect;

pub enum IdentifierResolution {
    Strict,
    Flexible,
}

pub struct Options {
    pub dialect: Box<dyn Dialect>,
    pub identifier_resolution: IdentifierResolution,
}
