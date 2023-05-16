use std::ops::{BitAnd, Not};

use crate::syntax_tree::ConditionSet;

use super::schema::{ColumnId, Schema, TableId};
use JoinQuantity::*;

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
    fn get_base(&self) -> Reference;
    fn get_target(&self) -> Reference;
    fn get_direction(&self) -> LinkDirection;
    fn get_join_quantity(&self) -> JoinQuantity;
}

impl Link for ForwardLinkToOne {
    fn get_direction(&self) -> LinkDirection {
        LinkDirection::Forward
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        One
    }

    fn get_start(&self) -> Reference {
        self.base
    }

    fn get_end(&self) -> Reference {
        self.target
    }

    fn get_base(&self) -> Reference {
        self.base
    }

    fn get_target(&self) -> Reference {
        self.target
    }
}

impl Link for ReverseLinkToOne {
    fn get_direction(&self) -> LinkDirection {
        LinkDirection::Reverse
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        One
    }

    fn get_start(&self) -> Reference {
        self.target
    }

    fn get_end(&self) -> Reference {
        self.base
    }

    fn get_base(&self) -> Reference {
        self.base
    }

    fn get_target(&self) -> Reference {
        self.target
    }
}

impl Link for ReverseLinkToMany {
    fn get_direction(&self) -> LinkDirection {
        LinkDirection::Reverse
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        Many
    }

    fn get_start(&self) -> Reference {
        self.target
    }

    fn get_end(&self) -> Reference {
        self.base
    }

    fn get_base(&self) -> Reference {
        self.base
    }

    fn get_target(&self) -> Reference {
        self.target
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum JoinQuantity {
    One,
    Many,
}

impl BitAnd for JoinQuantity {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (One, One) => One,
            (One, Many) => Many,
            (Many, One) => Many,
            (Many, Many) => Many,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LinkDirection {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reference {
    pub table_id: TableId,
    pub column_id: ColumnId,
}

impl Reference {
    pub fn new(table_id: TableId, column_id: ColumnId) -> Self {
        Self {
            table_id,
            column_id,
        }
    }
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
    fn get_direction(&self) -> LinkDirection {
        match self {
            LinkToOne::ForwardLinkToOne(link) => link.get_direction(),
            LinkToOne::ReverseLinkToOne(link) => link.get_direction(),
        }
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        match self {
            LinkToOne::ForwardLinkToOne(link) => link.get_join_quantity(),
            LinkToOne::ReverseLinkToOne(link) => link.get_join_quantity(),
        }
    }

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

    fn get_base(&self) -> Reference {
        match self {
            LinkToOne::ForwardLinkToOne(link) => link.get_base(),
            LinkToOne::ReverseLinkToOne(link) => link.get_base(),
        }
    }

    fn get_target(&self) -> Reference {
        match self {
            LinkToOne::ForwardLinkToOne(link) => link.get_target(),
            LinkToOne::ReverseLinkToOne(link) => link.get_target(),
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
    fn get_direction(&self) -> LinkDirection {
        match self {
            MultiLink::ForwardLinkToOne(link) => link.get_direction(),
            MultiLink::ReverseLinkToOne(link) => link.get_direction(),
            MultiLink::ReverseLinkToMany(link) => link.get_direction(),
        }
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        match self {
            MultiLink::ForwardLinkToOne(link) => link.get_join_quantity(),
            MultiLink::ReverseLinkToOne(link) => link.get_join_quantity(),
            MultiLink::ReverseLinkToMany(link) => link.get_join_quantity(),
        }
    }

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

    fn get_base(&self) -> Reference {
        match self {
            MultiLink::ForwardLinkToOne(link) => link.get_base(),
            MultiLink::ReverseLinkToOne(link) => link.get_base(),
            MultiLink::ReverseLinkToMany(link) => link.get_base(),
        }
    }

    fn get_target(&self) -> Reference {
        match self {
            MultiLink::ForwardLinkToOne(link) => link.get_target(),
            MultiLink::ReverseLinkToOne(link) => link.get_target(),
            MultiLink::ReverseLinkToMany(link) => link.get_target(),
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
    fn get_direction(&self) -> LinkDirection {
        self.link.get_direction()
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        self.link.get_join_quantity()
    }

    fn get_start(&self) -> Reference {
        self.link.get_start()
    }

    fn get_end(&self) -> Reference {
        self.link.get_end()
    }

    fn get_base(&self) -> Reference {
        self.link.get_base()
    }

    fn get_target(&self) -> Reference {
        self.link.get_target()
    }
}
