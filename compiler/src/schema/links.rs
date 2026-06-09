use std::ops::Not;

use querydown_parser::ast::ConditionSet;

use super::{ColumnId, TableId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForwardLinkToOne {
    pub base: Reference,
    pub target: Reference,
}

impl From<ForeignKey> for ForwardLinkToOne {
    fn from(foreign_key: ForeignKey) -> Self {
        Self {
            base: foreign_key.base,
            target: foreign_key.target,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReverseLinkToOne {
    pub base: Reference,
    pub target: Reference,
}

impl From<ForeignKey> for ReverseLinkToOne {
    fn from(foreign_key: ForeignKey) -> Self {
        Self {
            base: foreign_key.base,
            target: foreign_key.target,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReverseLinkToMany {
    pub base: Reference,
    pub target: Reference,
}

impl From<ForeignKey> for ReverseLinkToMany {
    fn from(foreign_key: ForeignKey) -> Self {
        Self {
            base: foreign_key.base,
            target: foreign_key.target,
        }
    }
}

pub trait Link {
    fn get_start(&self) -> Reference;
    fn get_end(&self) -> Reference;
}

impl Link for ForwardLinkToOne {
    fn get_start(&self) -> Reference {
        self.base
    }

    fn get_end(&self) -> Reference {
        self.target
    }
}

impl Link for ReverseLinkToOne {
    fn get_start(&self) -> Reference {
        self.target
    }

    fn get_end(&self) -> Reference {
        self.base
    }
}

impl Link for ReverseLinkToMany {
    fn get_start(&self) -> Reference {
        self.target
    }

    fn get_end(&self) -> Reference {
        self.base
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reference {
    pub table_id: TableId,
    pub column_id: ColumnId,
}

#[derive(Debug, Clone, Copy)]
pub struct ForeignKey {
    pub base: Reference,
    pub target: Reference,
    pub unique: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkToOne {
    ForwardLinkToOne(ForwardLinkToOne),
    ReverseLinkToOne(ReverseLinkToOne),
}

impl Link for LinkToOne {
    fn get_start(&self) -> Reference {
        match self {
            LinkToOne::ForwardLinkToOne(link) => link.get_start(),
            LinkToOne::ReverseLinkToOne(link) => link.get_start(),
        }
    }

    fn get_end(&self) -> Reference {
        match self {
            LinkToOne::ForwardLinkToOne(link) => link.get_end(),
            LinkToOne::ReverseLinkToOne(link) => link.get_end(),
        }
    }
}

impl TryFrom<FilteredLink> for LinkToOne {
    type Error = FilteredLink;

    fn try_from(filtered_link: FilteredLink) -> Result<Self, Self::Error> {
        if filtered_link.condition_set.is_empty().not() {
            return Err(filtered_link);
        }
        match filtered_link.link {
            MultiLink::ForwardLinkToOne(link) => Ok(LinkToOne::ForwardLinkToOne(link)),
            MultiLink::ReverseLinkToOne(link) => Ok(LinkToOne::ReverseLinkToOne(link)),
            _ => Err(filtered_link),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MultiLink {
    ForwardLinkToOne(ForwardLinkToOne),
    ReverseLinkToOne(ReverseLinkToOne),
    ReverseLinkToMany(ReverseLinkToMany),
}

impl Link for MultiLink {
    fn get_start(&self) -> Reference {
        match self {
            MultiLink::ForwardLinkToOne(link) => link.get_start(),
            MultiLink::ReverseLinkToOne(link) => link.get_start(),
            MultiLink::ReverseLinkToMany(link) => link.get_start(),
        }
    }

    fn get_end(&self) -> Reference {
        match self {
            MultiLink::ForwardLinkToOne(link) => link.get_end(),
            MultiLink::ReverseLinkToOne(link) => link.get_end(),
            MultiLink::ReverseLinkToMany(link) => link.get_end(),
        }
    }
}

#[derive(Debug)]
pub struct FilteredLink {
    pub link: MultiLink,
    pub condition_set: ConditionSet,
}

impl From<MultiLink> for FilteredLink {
    fn from(link: MultiLink) -> Self {
        Self {
            link,
            condition_set: ConditionSet::default(),
        }
    }
}

impl Link for FilteredLink {
    fn get_start(&self) -> Reference {
        self.link.get_start()
    }

    fn get_end(&self) -> Reference {
        self.link.get_end()
    }
}
