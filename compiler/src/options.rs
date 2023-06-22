use std::collections::HashMap;

use crate::{sql::Dialect, utils::FlexMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierResolution {
    Strict,
    Flexible,
}

impl Default for IdentifierResolution {
    fn default() -> Self {
        IdentifierResolution::Flexible
    }
}

pub struct Options {
    pub dialect: Box<dyn Dialect>,
    pub identifier_resolution: IdentifierResolution,
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
