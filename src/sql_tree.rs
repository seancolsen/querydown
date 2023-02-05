use crate::engines::engine::Engine;

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

#[derive(Debug, Clone, PartialEq, Default)]
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

impl From<String> for Select {
    fn from(base_table: String) -> Self {
        Self {
            base_table,
            columns: vec![],
            ctes: vec![],
            condition_set: ConditionSet::default(),
        }
    }
}

impl Select {
    pub fn render<E: Engine>(&self, engine: &E) -> String {
        let mut rendered = String::new();
        rendered.push_str("SELECT *");
        rendered.push_str(" FROM ");
        rendered.push_str(engine.quote_identifier(&self.base_table).as_str());
        rendered.push_str(";");
        rendered
    }
}
