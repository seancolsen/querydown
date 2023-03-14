use std::ops::BitAnd;

use crate::syntax_tree::ConditionSet;

use super::schema::{ColumnId, TableId};
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

#[derive(Debug, Clone, Copy)]
pub enum SimpleLink {
    ForwardLinkToOne(ForwardLinkToOne),
    ReverseLinkToOne(ReverseLinkToOne),
    ReverseLinkToMany(ReverseLinkToMany),
}

impl Link for SimpleLink {
    fn get_direction(&self) -> LinkDirection {
        match self {
            SimpleLink::ForwardLinkToOne(link) => link.get_direction(),
            SimpleLink::ReverseLinkToOne(link) => link.get_direction(),
            SimpleLink::ReverseLinkToMany(link) => link.get_direction(),
        }
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        match self {
            SimpleLink::ForwardLinkToOne(link) => link.get_join_quantity(),
            SimpleLink::ReverseLinkToOne(link) => link.get_join_quantity(),
            SimpleLink::ReverseLinkToMany(link) => link.get_join_quantity(),
        }
    }

    fn get_start(&self) -> Reference {
        match self {
            SimpleLink::ForwardLinkToOne(link) => link.get_start(),
            SimpleLink::ReverseLinkToOne(link) => link.get_start(),
            SimpleLink::ReverseLinkToMany(link) => link.get_start(),
        }
    }

    fn get_end(&self) -> Reference {
        match self {
            SimpleLink::ForwardLinkToOne(link) => link.get_end(),
            SimpleLink::ReverseLinkToOne(link) => link.get_end(),
            SimpleLink::ReverseLinkToMany(link) => link.get_end(),
        }
    }

    fn get_base(&self) -> Reference {
        match self {
            SimpleLink::ForwardLinkToOne(link) => link.get_base(),
            SimpleLink::ReverseLinkToOne(link) => link.get_base(),
            SimpleLink::ReverseLinkToMany(link) => link.get_base(),
        }
    }

    fn get_target(&self) -> Reference {
        match self {
            SimpleLink::ForwardLinkToOne(link) => link.get_target(),
            SimpleLink::ReverseLinkToOne(link) => link.get_target(),
            SimpleLink::ReverseLinkToMany(link) => link.get_target(),
        }
    }
}

#[derive(Debug)]
pub struct FilteredReverseLinkToMany {
    pub link: ReverseLinkToMany,
    pub condition_set: ConditionSet,
}

impl Link for FilteredReverseLinkToMany {
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

#[derive(Debug)]
pub enum GenericLink {
    ForwardLinkToOne(ForwardLinkToOne),
    ReverseLinkToOne(ReverseLinkToOne),
    FilteredReverseLinkToMany(FilteredReverseLinkToMany),
}

impl GenericLink {
    pub fn to_many(link: ReverseLinkToMany) -> Self {
        Self::FilteredReverseLinkToMany(FilteredReverseLinkToMany {
            link,
            condition_set: ConditionSet::default(),
        })
    }

    pub fn filtered_to_many(link: ReverseLinkToMany, condition_set: ConditionSet) -> Self {
        Self::FilteredReverseLinkToMany(FilteredReverseLinkToMany {
            link,
            condition_set,
        })
    }

    pub fn set_condition_set(&mut self, condition_set: ConditionSet) {
        match self {
            GenericLink::ForwardLinkToOne(_) => {}
            GenericLink::ReverseLinkToOne(_) => {}
            GenericLink::FilteredReverseLinkToMany(link) => {
                link.condition_set = condition_set;
            }
        }
    }
}

impl Link for GenericLink {
    fn get_direction(&self) -> LinkDirection {
        match self {
            GenericLink::ForwardLinkToOne(link) => link.get_direction(),
            GenericLink::ReverseLinkToOne(link) => link.get_direction(),
            GenericLink::FilteredReverseLinkToMany(link) => link.get_direction(),
        }
    }

    fn get_join_quantity(&self) -> JoinQuantity {
        match self {
            GenericLink::ForwardLinkToOne(link) => link.get_join_quantity(),
            GenericLink::ReverseLinkToOne(link) => link.get_join_quantity(),
            GenericLink::FilteredReverseLinkToMany(link) => link.get_join_quantity(),
        }
    }

    fn get_start(&self) -> Reference {
        match self {
            GenericLink::ForwardLinkToOne(link) => link.get_start(),
            GenericLink::ReverseLinkToOne(link) => link.get_start(),
            GenericLink::FilteredReverseLinkToMany(link) => link.get_start(),
        }
    }

    fn get_end(&self) -> Reference {
        match self {
            GenericLink::ForwardLinkToOne(link) => link.get_end(),
            GenericLink::ReverseLinkToOne(link) => link.get_end(),
            GenericLink::FilteredReverseLinkToMany(link) => link.get_end(),
        }
    }

    fn get_base(&self) -> Reference {
        match self {
            GenericLink::ForwardLinkToOne(link) => link.get_base(),
            GenericLink::ReverseLinkToOne(link) => link.get_base(),
            GenericLink::FilteredReverseLinkToMany(link) => link.get_base(),
        }
    }

    fn get_target(&self) -> Reference {
        match self {
            GenericLink::ForwardLinkToOne(link) => link.get_target(),
            GenericLink::ReverseLinkToOne(link) => link.get_target(),
            GenericLink::FilteredReverseLinkToMany(link) => link.get_target(),
        }
    }
}

impl From<SimpleLink> for GenericLink {
    fn from(link: SimpleLink) -> Self {
        match link {
            SimpleLink::ForwardLinkToOne(link) => Self::ForwardLinkToOne(link),
            SimpleLink::ReverseLinkToOne(link) => Self::ReverseLinkToOne(link),
            SimpleLink::ReverseLinkToMany(link) => Self::to_many(link),
        }
    }
}
