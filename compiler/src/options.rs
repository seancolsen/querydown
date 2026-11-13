use std::collections::HashMap;

use crate::{sql::Dialect, utils::FlexMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdentifierResolution {
    Strict,
    #[default]
    Flexible,
}

pub struct Options {
    pub dialect: Box<dyn Dialect>,
    pub identifier_resolution: IdentifierResolution,
    /// The maximum number of rows the query returns, rendered as a `LIMIT` clause on the outermost
    /// query. `None` imposes no limit.
    ///
    /// Pagination is controlled by the application rather than by the query's author, so it is an
    /// option to the compiler rather than part of the Querydown language.
    pub limit: Option<u64>,
    /// The number of rows to skip before returning any, rendered as an `OFFSET` clause on the
    /// outermost query. `None` skips no rows.
    pub offset: Option<u64>,
}

impl Options {
    pub fn resolve_identifier<'b, T>(
        &self,
        map: &'b HashMap<String, T>,
        identifier: &str,
    ) -> Option<&'b T> {
        match self.identifier_resolution {
            IdentifierResolution::Strict => map.get(identifier),
            IdentifierResolution::Flexible => map.flex_get(identifier),
        }
    }
}
