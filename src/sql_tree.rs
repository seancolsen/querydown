#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub base_table: String,
    pub columns: Vec<String>,
    pub ctes: Vec<Cte>,
    pub condition_set: ConditionSet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cte {
    pub name: String,
    pub select: Select,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<ConditionSetEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionSetEntry {
    Expression(Expression),
    ConditionSet(ConditionSet),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Conjunction {
    #[default]
    And,
    Or,
}

type Expression = String;
