use crate::{compiling::sql_tree::SortEntry, syntax_tree::SortSpec};

pub struct UnplacedSortEntry {
    entry: SortEntry,
    ordinal: Option<u32>,
}

pub struct SortingStack {
    entries: Vec<UnplacedSortEntry>,
}

impl SortingStack {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, expression: String, sort_spec: SortSpec) {
        let entry = UnplacedSortEntry {
            entry: SortEntry {
                expression,
                direction: sort_spec.direction,
                nulls_sort: sort_spec.nulls_sort,
            },
            ordinal: sort_spec.ordinal,
        };
        self.entries.push(entry);
    }
}

impl From<SortingStack> for Vec<SortEntry> {
    fn from(stack: SortingStack) -> Self {
        let mut entries = stack.entries;
        let max_ordinal = entries.iter().filter_map(|e| e.ordinal).max().unwrap_or(0);
        entries.sort_by_key(|entry| entry.ordinal.unwrap_or(max_ordinal.saturating_add(1)));
        entries.into_iter().map(|entry| entry.entry).collect()
    }
}
