use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use querydown_parser::ast::{
    ComputedColumn, ConstantDef, CustomComparisonDef, Expr, FunctionDef, PathPart, WindowFn,
};

use crate::{
    errors::msg,
    schema::{
        chain::Chain,
        links::{FilteredLink, Link, LinkToOne},
        Schema, Table,
    },
    sql::expr::build,
    sql::tree::{Cte, CtePurpose, Join, SqlExpr},
    Options,
};

use super::{
    constants::*,
    functions::{
        get_standard_aggregate_functions, get_standard_scalar_functions, AggregateFuncMap,
        AggregateFunction, ScalarFuncMap, ScalarFunction,
    },
    join_tree::JoinTree,
    paths::{build_cte_select, AggregateExprTemplate, ValueViaCte},
};

/// A user definition whose body is being expanded (inlined). Expanding a definition from within its
/// own expansion — directly or via other definitions — would recurse forever, so the scope tracks
/// which definitions are being expanded in order to report such cycles as errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expansion {
    Function(String),
    ComputedColumn { table: String, column: String },
    CustomComparison { table: String, name: String },
}

impl Expansion {
    fn code(&self) -> String {
        match self {
            Expansion::Function(name) => format!("`{name}`"),
            Expansion::ComputedColumn { table, column } => format!("`{table}.{column}`"),
            Expansion::CustomComparison { table, name } => format!("`{table}.{name}`"),
        }
    }

    /// The error message for a cycle, given the chain of expansions that leads back to `self`
    /// (starting with `self`).
    fn cycle_message(&self, chain: &[Expansion]) -> String {
        let mut message = match self {
            Expansion::Function(name) => {
                format!("The function `{name}` calls itself, but functions can't be recursive.")
            }
            Expansion::ComputedColumn { table, column } => format!(
                "The computed column `{column}` on the table `{table}` is defined in terms of \
                 itself."
            ),
            Expansion::CustomComparison { table, name }
                if name == DEFAULT_TEXT_SEARCH_COMPARISON_NAME =>
            {
                format!(
                    "The default text search for the table `{table}` is defined in terms of itself. \
                     Note that within the definition of `{name}`, a bare word (or any value \
                     without a column to compare it to) is itself a default text search."
                )
            }
            Expansion::CustomComparison { table, name } => format!(
                "The custom comparison `{name}` on the table `{table}` is defined in terms of \
                 itself."
            ),
        };
        if chain.len() > 1 {
            let path: Vec<String> = chain
                .iter()
                .chain(std::iter::once(self))
                .map(Expansion::code)
                .collect();
            message.push_str(&format!(" (The cycle is {}.)", path.join(" → ")));
        }
        message
    }
}

/// How a variable binding was introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// A constant defined before the query, e.g. `@user_id = 1234`.
    Constant,
    /// A function parameter, a function's local assignment, or a custom comparison's parameter.
    Local,
}

#[derive(Debug, Clone)]
struct Binding {
    id: usize,
    kind: BindingKind,
    expr: Expr,
}

/// The result of looking up a variable with [`Scope::lookup_variable`].
pub enum VariableLookup {
    /// The variable's bound expression, and the ID with which to [hide](Scope::hide_binding) the
    /// binding while that expression is being expanded.
    Found(Expr, usize),
    /// The variable is bound, but only by bindings that are currently being expanded — i.e. it is
    /// being referenced from within its own definition.
    OnlyHidden(BindingKind),
    NotFound,
}

/// Expansion bookkeeping shared by a scope and every scope spawned from it, since a definition can
/// be expanded across scopes (e.g. a constant referenced within `++#comments{ ... }`).
#[derive(Default)]
struct ExpansionState {
    /// Definitions currently being expanded, outermost first.
    stack: Vec<Expansion>,
    /// IDs of variable bindings currently being expanded. While a binding's expression is being
    /// inlined, references to its name resolve to whatever binding it shadows instead (if any). This
    /// keeps substitution hygienic — e.g. a function parameter `@x` bound to an argument that is
    /// itself `@x` (referring to an outer `@x`) — and prevents infinite expansion.
    hidden_bindings: Vec<usize>,
    next_binding_id: usize,
}

/// Marks a definition or variable binding as being expanded, until dropped. See
/// [`Scope::enter_expansion`] and [`Scope::hide_binding`].
#[must_use]
pub struct ExpansionGuard {
    state: Rc<RefCell<ExpansionState>>,
    is_binding: bool,
}

impl Drop for ExpansionGuard {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if self.is_binding {
            state.hidden_bindings.pop();
        } else {
            state.stack.pop();
        }
    }
}

/// A context in which SQL doesn't permit a window function, so that compiling one there would
/// produce SQL the database rejects. See [`Scope::with_window_ban`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowBan {
    /// A `WHERE` clause. Window functions written directly among a stage's conditions are computed
    /// in a wrapping query instead (see [`Scope::substitute_window`]), so this only catches windows
    /// that are reached some other way, e.g. through a computed column.
    Condition,
    /// Anything compiled within the CTE for a path to related records (`#comments{…}`), whose
    /// values end up in a `WHERE` clause or an aggregate.
    RelatedRecords,
    /// The argument or sorting of an aggregate function.
    Aggregate,
    /// The arguments, partitioning, or ordering of another window function.
    Nested,
}

pub struct Scope<'a, 'b> {
    parent: Option<&'b Scope<'a, 'b>>,
    pub options: &'a Options,
    pub schema: &'a Schema,
    base_table: &'a Table,
    /// The alias under which the base table appears in the SQL. Usually the table's own name, but a
    /// scope evaluating a filter on a table that a path revisits (e.g. the second `comments` in
    /// `#comments.issue.#comments{…}`) addresses that table by a suffixed alias.
    base_alias: String,
    join_tree: JoinTree,
    pub path_prefix: Vec<PathPart>,
    aliases: HashSet<String>,
    cte_naming_index: usize,
    scalar_functions: ScalarFuncMap,
    aggregate_functions: AggregateFuncMap,
    /// Computed columns, defined before the query and keyed by `(canonical table name, column name
    /// as written in the definition)`. The column name is looked up using identifier resolution.
    computed_columns: HashMap<String, HashMap<String, Expr>>,
    /// Variable bindings keyed by name (without the `@` sigil). This holds user-defined constants
    /// (registered before the query) as well as function parameters and local assignments (bound
    /// temporarily while a user-defined function is being applied). Each binding maps to an AST
    /// expression which is inlined wherever the variable is referenced. A name may have several
    /// bindings, the last of which shadows the others.
    variables: HashMap<String, Vec<Binding>>,
    expansion_state: Rc<RefCell<ExpansionState>>,
    /// User-defined scalar functions, keyed by name, registered before the query.
    user_functions: HashMap<String, FunctionDef>,
    /// Custom comparisons, defined before the query and keyed by `(canonical table name, comparison
    /// name as written in the definition)`. The name is looked up using identifier resolution.
    custom_comparisons: HashMap<String, HashMap<String, CustomComparisonDef>>,
    /// When set, compiling a window function is an error (see [`WindowBan`]). Inherited by spawned
    /// scopes, since anything they compile is folded into the same part of the query.
    window_ban: Option<WindowBan>,
    /// Window functions whose values were computed in a wrapping query, paired with the SQL that
    /// references each value. A window function that matches one of these compiles to that
    /// reference instead. Not inherited by spawned scopes, which compile in other contexts.
    window_substitutions: Vec<(WindowFn, SqlExpr)>,
    /// Whether this scope has compiled a window function (excluding substitutions). Used to reject
    /// window functions in a grouped stage.
    has_compiled_window: bool,
}

impl<'a, 'b> Scope<'a, 'b> {
    /// Builds a top-level scope whose base table is supplied directly (rather than looked up by
    /// name in the schema). This is used for the stages of a query pipeline, whose base "table" is
    /// a synthetic table representing the output of the previous stage.
    pub fn build_with_base_table(
        options: &'a Options,
        schema: &'a Schema,
        base_table: &'a Table,
    ) -> Self {
        Self {
            parent: None,
            options,
            schema,
            base_table,
            base_alias: base_table.name.to_owned(),
            join_tree: JoinTree::new(base_table.name.to_owned()),
            path_prefix: vec![],
            aliases: HashSet::from([base_table.name.to_owned()]),
            cte_naming_index: 0,
            scalar_functions: get_standard_scalar_functions(),
            aggregate_functions: get_standard_aggregate_functions(),
            computed_columns: HashMap::new(),
            variables: HashMap::new(),
            expansion_state: Rc::default(),
            user_functions: HashMap::new(),
            custom_comparisons: HashMap::new(),
            window_ban: None,
            window_substitutions: Vec::new(),
            has_compiled_window: false,
        }
    }

    /// Records the query's constant definitions. Like computed columns, the definitions' expressions
    /// are resolved lazily (inlined when referenced), so this does not validate them and definition
    /// order does not matter.
    pub fn register_constants(&mut self, constants: Vec<ConstantDef>) {
        for constant in constants {
            self.push_binding(constant.name, BindingKind::Constant, constant.expr);
        }
    }

    fn push_binding(&mut self, name: String, kind: BindingKind, expr: Expr) {
        let id = {
            let mut state = self.expansion_state.borrow_mut();
            state.next_binding_id += 1;
            state.next_binding_id
        };
        self.variables
            .entry(name)
            .or_default()
            .push(Binding { id, kind, expr });
    }

    /// Looks up a variable binding by name (without the `@` sigil), falling back to parent scopes.
    /// Bindings that are currently being expanded are skipped (see [`Scope::hide_binding`]).
    pub fn lookup_variable(&self, name: &str) -> VariableLookup {
        let mut hidden_kind: Option<BindingKind> = None;
        let mut scope = Some(self);
        while let Some(s) = scope {
            if let Some(bindings) = s.variables.get(name) {
                let state = self.expansion_state.borrow();
                for binding in bindings.iter().rev() {
                    if state.hidden_bindings.contains(&binding.id) {
                        hidden_kind.get_or_insert(binding.kind);
                    } else {
                        return VariableLookup::Found(binding.expr.clone(), binding.id);
                    }
                }
            }
            scope = s.parent;
        }
        match hidden_kind {
            Some(kind) => VariableLookup::OnlyHidden(kind),
            None => VariableLookup::NotFound,
        }
    }

    /// Hides the variable binding with the given ID (from [`Scope::lookup_variable`]) until the
    /// returned guard is dropped. Call this while expanding the binding's expression, so that
    /// references within it can't resolve to the binding itself.
    pub fn hide_binding(&self, id: usize) -> ExpansionGuard {
        self.expansion_state.borrow_mut().hidden_bindings.push(id);
        ExpansionGuard {
            state: Rc::clone(&self.expansion_state),
            is_binding: true,
        }
    }

    /// Records that `expansion` is being expanded, until the returned guard is dropped. Fails with a
    /// descriptive error if it is already being expanded, which would mean the definition refers to
    /// itself (directly or indirectly) and would otherwise expand forever.
    pub fn enter_expansion(&self, expansion: Expansion) -> Result<ExpansionGuard, String> {
        let mut state = self.expansion_state.borrow_mut();
        if let Some(index) = state.stack.iter().position(|e| *e == expansion) {
            return Err(expansion.cycle_message(&state.stack[index..]));
        }
        state.stack.push(expansion);
        Ok(ExpansionGuard {
            state: Rc::clone(&self.expansion_state),
            is_binding: false,
        })
    }

    /// Records the query's user-defined function definitions. Their bodies are resolved lazily (when
    /// the function is applied), so this does not validate them and definition order does not matter.
    pub fn register_functions(&mut self, functions: Vec<FunctionDef>) {
        for function in functions {
            self.user_functions.insert(function.name.clone(), function);
        }
    }

    /// Looks up a user-defined scalar function by name, falling back to parent scopes.
    pub fn get_user_function(&self, name: &str) -> Option<&FunctionDef> {
        self.user_functions.get(name).or_else(|| {
            self.parent
                .and_then(|parent| parent.get_user_function(name))
        })
    }

    /// Evaluates `f` with additional variable bindings temporarily in scope, removing them
    /// afterward. Used to bind a user-defined function's parameters and local assignments while its
    /// body is being compiled.
    pub fn with_variable_bindings<T>(
        &mut self,
        bindings: Vec<(String, Expr)>,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let names: Vec<String> = bindings.iter().map(|(name, _)| name.clone()).collect();
        for (name, expr) in bindings {
            self.push_binding(name, BindingKind::Local, expr);
        }
        let result = f(self);
        for name in names.into_iter().rev() {
            if let Some(stack) = self.variables.get_mut(&name) {
                stack.pop();
                if stack.is_empty() {
                    self.variables.remove(&name);
                }
            }
        }
        result
    }

    /// Records the query's computed column definitions, validating that each refers to a real table.
    /// The definitions' expressions are resolved lazily (when referenced), so this does not validate
    /// the expressions themselves, and definition order does not matter.
    pub fn register_computed_columns(
        &mut self,
        computed_columns: Vec<ComputedColumn>,
    ) -> Result<(), String> {
        for computed_column in computed_columns {
            let table = get_table_by_name(self.options, self.schema, &computed_column.table)
                .ok_or_else(|| msg::computed_column_unknown_table(&computed_column.table))?;
            self.computed_columns
                .entry(table.name.clone())
                .or_default()
                .insert(computed_column.name, computed_column.expr);
        }
        Ok(())
    }

    /// Looks up a computed column's expression by table and column name, using identifier
    /// resolution for the column name. Falls back to parent scopes.
    pub fn get_computed_column(&self, table_name: &str, column_name: &str) -> Option<&Expr> {
        self.computed_columns
            .get(table_name)
            .and_then(|columns| self.options.resolve_identifier(columns, column_name))
            .or_else(|| {
                self.parent
                    .and_then(|parent| parent.get_computed_column(table_name, column_name))
            })
    }

    /// Records the query's custom comparison definitions, validating that each refers to a real
    /// table. The bodies are resolved lazily (when the comparison is used), so this does not
    /// validate them and definition order does not matter.
    pub fn register_custom_comparisons(
        &mut self,
        custom_comparisons: Vec<CustomComparisonDef>,
    ) -> Result<(), String> {
        for def in custom_comparisons {
            let table = get_table_by_name(self.options, self.schema, &def.table)
                .ok_or_else(|| msg::custom_comparison_unknown_table(&def.table))?;
            self.custom_comparisons
                .entry(table.name.clone())
                .or_default()
                .insert(def.name.clone(), def);
        }
        Ok(())
    }

    /// Whether any custom comparisons are registered (here or in a parent scope). Used to avoid the
    /// cost of custom-comparison resolution on queries that define none.
    pub fn has_custom_comparisons(&self) -> bool {
        !self.custom_comparisons.is_empty()
            || self
                .parent
                .map(|parent| parent.has_custom_comparisons())
                .unwrap_or(false)
    }

    /// Looks up a custom comparison by table and name, using identifier resolution for the name.
    /// Falls back to parent scopes.
    pub fn get_custom_comparison(
        &self,
        table_name: &str,
        name: &str,
    ) -> Option<&CustomComparisonDef> {
        self.custom_comparisons
            .get(table_name)
            .and_then(|comparisons| self.options.resolve_identifier(comparisons, name))
            .or_else(|| {
                self.parent
                    .and_then(|parent| parent.get_custom_comparison(table_name, name))
            })
    }

    pub fn get_base_table(&self) -> &Table {
        self.base_table
    }

    /// The alias under which the base table appears in the SQL (see [`Scope::spawn_with_aliases`]).
    pub fn get_base_alias(&self) -> &str {
        &self.base_alias
    }

    pub fn decompose_join_tree(&mut self) -> (Vec<Join>, Vec<Cte>) {
        let join_tree =
            std::mem::replace(&mut self.join_tree, JoinTree::new(self.base_alias.clone()));
        join_tree.decompose(self)
    }

    pub fn spawn(&'b self, base_table: &'a Table) -> Self {
        self.spawn_with_aliases(base_table, &base_table.name, HashSet::new())
    }

    /// Like [`Scope::spawn`], but seeds the new scope's set of reserved table aliases and names the
    /// alias by which the base table is addressed. This is used when the spawned scope renders SQL
    /// that is folded back into the parent's query (e.g. an aggregate's `ORDER BY`), so that any
    /// joins it introduces avoid colliding with aliases the parent has already assigned, and so
    /// that its columns refer to the right instance of a table the parent joined more than once.
    pub fn spawn_with_aliases(
        &'b self,
        base_table: &'a Table,
        base_alias: &str,
        aliases: HashSet<String>,
    ) -> Self {
        let mut aliases = aliases;
        aliases.insert(base_alias.to_owned());
        Scope {
            parent: Some(self),
            options: self.options,
            schema: self.schema,
            base_table,
            base_alias: base_alias.to_owned(),
            join_tree: JoinTree::new(base_alias.to_owned()),
            path_prefix: vec![],
            aliases,
            cte_naming_index: 0,
            scalar_functions: HashMap::new(),
            aggregate_functions: HashMap::new(),
            computed_columns: HashMap::new(),
            variables: HashMap::new(),
            expansion_state: Rc::clone(&self.expansion_state),
            user_functions: HashMap::new(),
            custom_comparisons: HashMap::new(),
            window_ban: self.window_ban,
            window_substitutions: Vec::new(),
            has_compiled_window: false,
        }
    }

    /// Bans window functions while evaluating `f` (see [`WindowBan`]), restoring the previous ban
    /// afterward. The innermost ban wins, since it best describes where the window function is.
    pub fn with_window_ban<T>(&mut self, ban: WindowBan, f: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.window_ban.replace(ban);
        let result = f(self);
        self.window_ban = previous;
        result
    }

    /// Bans window functions for the rest of this scope's life (see [`WindowBan`]).
    pub fn ban_windows(&mut self, ban: WindowBan) {
        self.window_ban = Some(ban);
    }

    pub fn window_ban(&self) -> Option<WindowBan> {
        self.window_ban
    }

    /// Makes `window` compile to `reference`, the column through which a wrapping query supplies
    /// its value.
    pub fn add_window_substitution(&mut self, window: WindowFn, reference: SqlExpr) {
        self.window_substitutions.push((window, reference));
    }

    /// The SQL referencing `window`'s value, if a wrapping query supplies it.
    pub fn substitute_window(&self, window: &WindowFn) -> Option<SqlExpr> {
        self.window_substitutions
            .iter()
            .find(|(w, _)| w == window)
            .map(|(_, reference)| reference.clone())
    }

    pub fn note_compiled_window(&mut self) {
        self.has_compiled_window = true;
    }

    pub fn has_compiled_window(&self) -> bool {
        self.has_compiled_window
    }

    /// Returns a copy of the table aliases currently reserved in this scope. Used to seed a spawned
    /// scope via [`Scope::spawn_with_aliases`].
    pub fn cloned_aliases(&self) -> HashSet<String> {
        self.aliases.clone()
    }

    /// Reserves `aliases` in this scope so that it won't assign any of them again. Used after a
    /// scope spawned via [`Scope::spawn_with_aliases`] has folded its joins into this scope's query,
    /// so that the aliases those joins use stay taken.
    pub fn reserve_aliases(&mut self, aliases: HashSet<String>) {
        self.aliases.extend(aliases);
    }

    /// `parts` prepended with the scope's current `path_prefix`, i.e. the path as seen from the base
    /// table. A path written inside a scoped condition set (`author{…}`) is relative to the scoped
    /// table, so it must be prefixed this way before it is clarified; clarifying the bare parts would
    /// resolve them from the base table instead.
    pub fn prefixed(&self, parts: &[PathPart]) -> Vec<PathPart> {
        self.path_prefix.iter().chain(parts).cloned().collect()
    }

    pub fn with_path_prefix<T>(
        &mut self,
        path_prefix: Vec<PathPart>,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        // We don't do any nested path prefixing, so the logic here simply resets the path prefix
        // to be empty afterwards. We can make it more complex if we need to support nested path
        // prefixing at some point.
        self.path_prefix = path_prefix;
        let return_value = f(self);
        self.path_prefix = vec![];
        return_value
    }

    pub fn table_column_expr(&self, table_name: &str, column_name: &str) -> SqlExpr {
        SqlExpr::atom(self.options.dialect.table_column(table_name, column_name))
    }

    /// Returns a table alias that is unique within the context of the query.
    fn integrate_chain(&mut self, chain: Option<&Chain<LinkToOne>>, cte: Option<Cte>) -> String {
        // TODO figure out how to reduce code duplication between the logic here and
        // Scope::get_alias. There are some borrowing issues with using the get_alias method here.
        // Need to find a way to structure this code so that both use-cases can share it.
        let mut aliases = std::mem::take(&mut self.aliases);
        let mut try_alias = |alias: &str| -> bool {
            if !aliases.contains(alias) {
                aliases.insert(alias.to_string());
                true
            } else {
                false
            }
        };
        let get_alias = |link: &LinkToOne| -> String {
            let ideal_alias = self.schema.get_ideal_alias_for_link_to_one(link);
            if try_alias(ideal_alias) {
                return ideal_alias.to_string();
            }
            let mut suffix_index: usize = 1;
            loop {
                let new_alias = format!("{}_{}", ideal_alias, suffix_index);
                if try_alias(&new_alias) {
                    return new_alias;
                }
                suffix_index += 1;
            }
        };
        let alias = self.join_tree.integrate_chain(chain, get_alias, cte);
        self.aliases = aliases;
        alias
    }

    pub fn join_chain_to_one(&mut self, chain: &Chain<LinkToOne>) -> String {
        self.integrate_chain(Some(chain), None)
    }

    pub fn get_alias(&mut self, ideal_alias: &str) -> String {
        let mut suffix_index: usize = 0;
        loop {
            let alias = if suffix_index == 0 {
                ideal_alias.to_owned()
            } else {
                format!("{}_{}", ideal_alias, suffix_index)
            };
            if !self.aliases.contains(&alias) {
                self.aliases.insert(alias.clone());
                return alias;
            }
            suffix_index += 1;
        }
    }

    pub fn join_chain_to_many(
        &mut self,
        head: &Option<Chain<LinkToOne>>,
        chain: Chain<FilteredLink>,
        aggregate_expr_template_opt: Option<AggregateExprTemplate>,
        purpose: CtePurpose,
    ) -> Result<SqlExpr, String> {
        let starting_reference = chain.get_first_link().get_start();
        let starting_table_id = starting_reference.table_id;
        let starting_column_id = starting_reference.column_id;
        let starting_table = self.schema.tables.get(&starting_table_id).unwrap();
        let starting_column = starting_table.columns.get(&starting_column_id).unwrap();
        let ValueViaCte {
            select,
            value_alias,
            coalesce_to_zero,
        } = build_cte_select(chain, aggregate_expr_template_opt, self, purpose)?;
        let cte_alias = self.get_cte_alias();
        let cte = Cte {
            select,
            alias: cte_alias.clone(),
            join_column_name: starting_column.name.clone(),
        };
        self.integrate_chain(head.as_ref(), Some(cte));
        let reference = self.table_column_expr(&cte_alias, &value_alias);
        // The CTE is joined via a LEFT JOIN, so base rows with no related records yield NULL. For
        // counts we coalesce that NULL to zero.
        if coalesce_to_zero {
            Ok(build::cond::coalesce(vec![reference, build::value::zero()]))
        } else {
            Ok(reference)
        }
    }

    fn get_cte_alias(&mut self) -> String {
        loop {
            let alias = format!("{}{}", CTE_ALIAS_PREFIX, self.cte_naming_index);
            self.cte_naming_index += 1;
            if !self.aliases.contains(&alias) {
                self.aliases.insert(alias.clone());
                return alias;
            }
        }
    }

    pub fn get_table_by_name(&self, name: &str) -> Option<&Table> {
        get_table_by_name(self.options, self.schema, name)
    }

    pub fn get_scalar_function(&self, name: &str) -> Option<&ScalarFunction> {
        self.scalar_functions.get(name).or_else(|| {
            self.parent
                .and_then(|parent| parent.get_scalar_function(name))
        })
    }

    pub fn get_aggregate_function(&self, name: &str) -> Option<&AggregateFunction> {
        self.aggregate_functions.get(name).or_else(|| {
            self.parent
                .and_then(|parent| parent.get_aggregate_function(name))
        })
    }
}

fn get_table_by_name<'a>(options: &Options, schema: &'a Schema, name: &str) -> Option<&'a Table> {
    options
        .resolve_identifier(&schema.table_lookup, name)
        .map(|id| schema.tables.get(id).unwrap())
}
