use std::collections::HashMap;

use crate::{dialects::dialect::Dialect, utils::flex_map::FlexMap};

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
        use crate::IdentifierResolution::*;
        match self.identifier_resolution {
            Strict => map.get(identifier),
            Flexible => map.flex_get(identifier),
        }
    }
}
