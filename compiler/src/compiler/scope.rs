use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use querydown_parser::ast::{
    ComputedColumn, ConstantDef, CustomComparisonDef, Expr, ExprKind, FunctionDef, Loc, PathPart,
    WindowFn,
};

use crate::{
    errors::msg,
    schema::{
        chain::Chain,
        links::{FilteredLink, Link, LinkToOne},
        Schema, Table,
    },
    sql::expr::build,
    sql::tree::{Cte, CtePurpose, Join, JoinType, SqlExpr},
    Error, Options,
};

use super::{
    constants::*,
    expr::convert_expr,
    functions::{
        get_standard_aggregate_functions, get_standard_scalar_functions, AggregateFuncMap,
        AggregateFunction, ScalarFuncMap, ScalarFunction,
    },
    join_tree::JoinTree,
    paths::{build_cte_select, clarify_path, AggregateExprTemplate, ValueViaCte},
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

/// A variable bound to the expression that is inlined wherever the variable is referenced.
///
/// The expression means what it meant *where it was written*, which for a local binding is not
/// where it's inlined: an argument `author` passed to a function is a column of the table the
/// function was called on, even if the function's body uses its parameter within a
/// `#comments{ … }` filter, and a variable within the argument is one that was visible at the call,
/// not a parameter of the function that happens to share its name. So a binding records the
/// lexical [environment](Env) and the [context](BindingContext) of its expression.
#[derive(Debug)]
pub struct Binding {
    id: usize,
    pub expr: Expr,
    /// The variables that `expr` can see. `None` for a constant, which sees only the constants.
    env: Option<Env>,
    /// Where `expr` was written. `None` for a constant, which isn't written within the query: any
    /// columns it refers to are those of wherever it's used.
    context: Option<BindingContext>,
}

/// Where an expression was written: which scope was being compiled, and what it was compiling.
#[derive(Debug, Clone)]
pub struct BindingContext {
    scope_id: usize,
    /// The scope's path prefix at the time, e.g. `author` within `author{ … }`, along with where
    /// the innermost scoped condition set's path was written (see [`Scope::scoped_path_loc`]).
    path_prefix: Vec<PathPart>,
    scoped_path_loc: Loc,
    /// The definitions that were being expanded at the time.
    stack: Vec<Expansion>,
}

/// A lexical environment: the variables visible at some point in the source, innermost first.
/// Environments are persistent (extending one leaves the original intact), which lets a binding
/// hold on to the environment its expression was written in.
#[derive(Debug, Clone, Default)]
pub struct Env(Option<Rc<EnvNode>>);

#[derive(Debug)]
struct EnvNode {
    name: String,
    binding: Rc<Binding>,
    parent: Env,
}

impl Env {
    fn with(&self, name: String, binding: Binding) -> Env {
        Env(Some(Rc::new(EnvNode {
            name,
            binding: Rc::new(binding),
            parent: self.clone(),
        })))
    }

    fn lookup(&self, name: &str) -> Option<Rc<Binding>> {
        let mut env = self;
        while let Some(node) = &env.0 {
            if node.name == name {
                return Some(Rc::clone(&node.binding));
            }
            env = &node.parent;
        }
        None
    }
}

/// Expansion bookkeeping shared by a scope and every scope spawned from it, since a definition can
/// be expanded across scopes (e.g. a constant referenced within `++#comments{ ... }`).
#[derive(Default)]
struct ExpansionState {
    /// Definitions currently being expanded, outermost first.
    stack: Vec<Expansion>,
    /// IDs of the constants currently being expanded, with which a constant that is defined in
    /// terms of itself is caught.
    expanding_constants: Vec<usize>,
    next_binding_id: usize,
    next_scope_id: usize,
}

/// Marks a definition or constant as being expanded, until dropped. See
/// [`Scope::enter_expansion`] and [`Scope::enter_constant`].
#[must_use]
pub struct ExpansionGuard {
    state: Rc<RefCell<ExpansionState>>,
    is_constant: bool,
}

impl Drop for ExpansionGuard {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if self.is_constant {
            state.expanding_constants.pop();
        } else {
            state.stack.pop();
        }
    }
}

/// A context in which SQL doesn't permit an aggregate of the rows being queried, like `%count` or
/// `created_at%max`. (An aggregate of *related* records, like `#comments.created_at%max`, is
/// computed in a query of its own, and is fine anywhere.) See [`Scope::with_aggregate_ban`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateBan {
    /// The conditions of a query, which become its `WHERE` clause.
    Condition,
    /// The filter of a path to related records (`#comments{…}`).
    RelatedRecordsFilter,
    /// The functions applied to each value that an aggregate function aggregates (the `|length` in
    /// `#comments.body|length%avg`), which are evaluated before the values are aggregated.
    AggregatedValue,
}

/// The record that the rows of a CTE's `SELECT` are related to: the one that a path to related
/// records (`#comments`) started from. An expression written in that record's context can be
/// compiled within the CTE by joining its table back in (see [`Scope::convert_in_context`]).
#[derive(Debug, Clone)]
pub struct OuterRecord<'a> {
    /// The context of the record.
    key: ContextKey,
    table: &'a Table,
    /// The column of `table` that the CTE's base table refers to.
    column: String,
    /// The alias, within the CTE, of the CTE's base table, and its column that refers to `table`.
    referrer_alias: String,
    referrer_column: String,
}

impl<'a> OuterRecord<'a> {
    pub fn new(
        parent: &Scope<'a, '_>,
        head: &[LinkToOne],
        table: &'a Table,
        column: String,
        referrer_alias: String,
        referrer_column: String,
    ) -> Self {
        OuterRecord {
            key: parent.context_key(head.to_vec()),
            table,
            column,
            referrer_alias,
            referrer_column,
        }
    }
}

/// Identifies the record that an expression is about: the one reached from the base table of the
/// scope with ID `scope_id` by following `links`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ContextKey {
    scope_id: usize,
    links: Vec<LinkToOne>,
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
    /// Identifies this scope among all those compiling the query (see [`BindingContext`]).
    id: usize,
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
    /// The variables visible to the expression being compiled: the user-defined constants, plus
    /// (while a function or custom comparison is being applied) its parameters and local
    /// assignments. Each maps to an AST expression which is inlined wherever the variable is
    /// referenced.
    env: Env,
    /// The user-defined constants, which are the variables visible within a definition.
    globals: Env,
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
    /// Where the first window function that this scope compiled (excluding substitutions) was
    /// written, if it has compiled one. Used to reject window functions in a grouped stage.
    compiled_window: Option<Loc>,
    /// When set, compiling an aggregate of the rows being queried is an error (see
    /// [`AggregateBan`]). Not inherited by spawned scopes, which compile queries of their own.
    aggregate_ban: Option<AggregateBan>,
    /// Where the path of the innermost scoped condition set being compiled (the `issue` in
    /// `issue{ … }`) was written.
    pub scoped_path_loc: Loc,
    /// Whether this scope compiles into the same `SELECT` as its parent (e.g. the filter of one
    /// link of a path to related records), rather than into a query of its own.
    shares_select_with_parent: bool,
    /// For the scope of a CTE that reaches related records: the record they're related to.
    outer_record: Option<OuterRecord<'a>>,
    /// When this scope's base table stands in for a record of an enclosing query that has been
    /// joined into this one (see [`Scope::convert_in_context`]): that record's context.
    stands_for: Option<ContextKey>,
    /// Whether SQL has been generated that refers to this scope's base table by its alias.
    base_alias_used: Cell<bool>,
    /// The record of an enclosing query that has been joined into this scope's `SELECT`, if one
    /// has been (see [`Scope::convert_in_context`]).
    stand_in: Option<StandIn>,
}

/// The table of a record of an enclosing query, joined into the `SELECT` of the CTE whose rows are
/// related to that record, so that expressions about the record can be compiled there.
struct StandIn {
    /// The alias under which the record's table is joined.
    alias: String,
    /// What has been joined to the record's table in turn.
    join_tree: JoinTree,
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
            id: 0,
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
            env: Env::default(),
            globals: Env::default(),
            expansion_state: Rc::default(),
            user_functions: HashMap::new(),
            custom_comparisons: HashMap::new(),
            window_ban: None,
            window_substitutions: Vec::new(),
            compiled_window: None,
            aggregate_ban: None,
            scoped_path_loc: Loc::NONE,
            shares_select_with_parent: false,
            outer_record: None,
            stands_for: None,
            base_alias_used: Cell::new(false),
            stand_in: None,
        }
    }

    /// Records the query's constant definitions. Like computed columns, the definitions' expressions
    /// are resolved lazily (inlined when referenced), so this does not validate them and definition
    /// order does not matter.
    pub fn register_constants(&mut self, constants: Vec<ConstantDef>) {
        for constant in constants {
            let binding = Binding {
                id: self.next_binding_id(),
                expr: constant.expr,
                env: None,
                context: None,
            };
            self.globals = self.globals.with(constant.name, binding);
        }
        self.env = self.globals.clone();
    }

    fn next_binding_id(&self) -> usize {
        let mut state = self.expansion_state.borrow_mut();
        state.next_binding_id += 1;
        state.next_binding_id
    }

    /// Looks up a variable by name (without the `@` sigil) among those visible to the expression
    /// being compiled.
    pub fn lookup_variable(&self, name: &str) -> Option<Rc<Binding>> {
        self.env.lookup(name)
    }

    /// The environment of the expression being compiled.
    pub fn env(&self) -> Env {
        self.env.clone()
    }

    /// The environment of a definition (a named function, a computed column, or a custom
    /// comparison), which sees the constants but none of the variables of wherever it's used.
    pub fn globals(&self) -> Env {
        self.globals.clone()
    }

    /// Evaluates `f` with `env` as the environment, restoring the previous one afterward.
    pub fn with_env<T>(&mut self, env: Env, f: impl FnOnce(&mut Self) -> T) -> T {
        let saved = std::mem::replace(&mut self.env, env);
        let result = f(self);
        self.env = saved;
        result
    }

    /// Where the expression being compiled was written, for binding it to a variable.
    pub fn context(&self) -> BindingContext {
        BindingContext {
            scope_id: self.id,
            path_prefix: self.path_prefix.clone(),
            scoped_path_loc: self.scoped_path_loc,
            stack: self.expansion_state.borrow().stack.clone(),
        }
    }

    /// Extends `env` with a local variable bound to `expr`, which was written within `context`,
    /// where it could see the variables of `expr_env`.
    pub fn bind(
        &self,
        env: &Env,
        name: String,
        expr: Expr,
        expr_env: Env,
        context: BindingContext,
    ) -> Env {
        let binding = Binding {
            id: self.next_binding_id(),
            expr,
            env: Some(expr_env),
            context: Some(context),
        };
        env.with(name, binding)
    }

    /// The expression that `expr` stands for: itself, unless it's a user-defined variable, in which
    /// case it's whatever the variable is (ultimately) bound to. This is how a literal with a
    /// special meaning in a comparison (`@null`, a duration, an empty string) is recognized when it
    /// arrives through a constant or a parameter.
    ///
    /// Only the *form* of the result is meaningful: compiling it here would lose the context that
    /// its binding recorded.
    pub fn see_through_variables(&self, expr: &Expr) -> Expr {
        let mut expr = expr.clone();
        let mut env = self.env.clone();
        // Constants can be defined in terms of each other in a cycle, which is reported when
        // they're compiled. Here, just stop.
        for _ in 0..64 {
            let ExprKind::Variable(name) = &expr.kind else {
                break;
            };
            if is_builtin_constant(name) {
                break;
            }
            let Some(binding) = env.lookup(name) else {
                break;
            };
            env = binding.env.clone().unwrap_or_else(|| self.globals.clone());
            expr = binding.expr.clone();
        }
        expr
    }

    /// Records that the constant bound by `binding` is being expanded, until the returned guard is
    /// dropped. Fails if it is already being expanded, which means that it's defined in terms of
    /// itself.
    fn enter_constant(&self, binding: &Binding) -> Option<ExpansionGuard> {
        let mut state = self.expansion_state.borrow_mut();
        if state.expanding_constants.contains(&binding.id) {
            return None;
        }
        state.expanding_constants.push(binding.id);
        Some(ExpansionGuard {
            state: Rc::clone(&self.expansion_state),
            is_constant: true,
        })
    }

    /// Evaluates `f` — which compiles or examines the expression of `binding` — in the context
    /// that the expression was written in, as far as that context is this scope's to restore: its
    /// environment, the definitions being expanded, and (when the expression was written within
    /// this very scope) the path prefix. Returns `None` for a constant defined in terms of itself.
    ///
    /// The second argument to `f` says whether the expression's table context is in effect too.
    /// When it isn't — the expression was written in an enclosing query — see
    /// [`Scope::convert_in_context`].
    fn with_binding<T>(
        &mut self,
        binding: &Binding,
        f: impl FnOnce(&mut Self, bool) -> T,
    ) -> Option<T> {
        let Some(context) = &binding.context else {
            // A constant is compiled wherever it's used, seeing only the other constants.
            let _guard = self.enter_constant(binding)?;
            let globals = self.globals.clone();
            return Some(self.with_env(globals, |scope| f(scope, true)));
        };
        let env = binding.env.clone().unwrap_or_else(|| self.globals.clone());
        let saved_stack = std::mem::replace(
            &mut self.expansion_state.borrow_mut().stack,
            context.stack.clone(),
        );
        let is_home = self.id == context.scope_id;
        let saved_prefix = if is_home {
            Some((
                std::mem::replace(&mut self.path_prefix, context.path_prefix.clone()),
                std::mem::replace(&mut self.scoped_path_loc, context.scoped_path_loc),
            ))
        } else {
            None
        };
        let result = self.with_env(env, |scope| f(scope, is_home));
        if let Some((prefix, loc)) = saved_prefix {
            self.path_prefix = prefix;
            self.scoped_path_loc = loc;
        }
        self.expansion_state.borrow_mut().stack = saved_stack;
        Some(result)
    }

    /// Compiles the expression that `binding` binds to a variable named `name`, which is being
    /// referenced within this scope.
    pub fn convert_binding(&mut self, name: &str, binding: &Binding) -> Result<SqlExpr, Error> {
        let expr = binding.expr.clone();
        self.with_binding(binding, |scope, is_home| {
            if is_home {
                return convert_expr(expr, scope);
            }
            // The unwrap is safe because only a local binding is ever away from home.
            let context = binding.context.as_ref().unwrap();
            scope.convert_in_context(expr, context)
        })
        .unwrap_or_else(|| Err(msg::constant_refers_to_itself(name)))
    }

    /// Examines the expression of `binding` with `f` (e.g. to infer its type), in the context the
    /// expression was written in. Unlike [`Scope::convert_binding`], this is for looking only:
    /// nothing that `f` compiles is kept. Returns `None` for a constant defined in terms of itself.
    pub fn inspect_binding<T>(
        &mut self,
        binding: &Binding,
        f: impl FnOnce(&Expr, &mut Scope) -> T,
    ) -> Option<T> {
        self.with_binding(binding, |scope, is_home| {
            if is_home {
                return f(&binding.expr, scope);
            }
            // The unwrap is safe because only a local binding is ever away from home.
            let context = binding.context.as_ref().unwrap();
            let Some(home) = scope.ancestor(context.scope_id) else {
                return f(&binding.expr, scope);
            };
            let home_table = home.base_table;
            let mut probe = scope.spawn(home_table);
            probe.path_prefix = context.path_prefix.clone();
            probe.scoped_path_loc = context.scoped_path_loc;
            f(&binding.expr, &mut probe)
        })
    }

    /// The enclosing scope (or this one) with the given ID.
    fn ancestor(&self, id: usize) -> Option<&Scope<'a, 'b>> {
        let mut scope = Some(self);
        while let Some(s) = scope {
            if s.id == id {
                return Some(s);
            }
            scope = s.parent;
        }
        None
    }

    /// Identifies the record reached from this scope's base table by following `links`. A scope
    /// whose base table stands in for a record of an enclosing query identifies it as that query
    /// would, so that the two agree.
    fn context_key(&self, links: Vec<LinkToOne>) -> ContextKey {
        match &self.stands_for {
            Some(key) => ContextKey {
                scope_id: key.scope_id,
                links: key.links.iter().copied().chain(links).collect(),
            },
            None => ContextKey {
                scope_id: self.id,
                links,
            },
        }
    }

    /// Identifies the record that an expression written within `context` is about, or `None` if
    /// its path prefix doesn't lead to a single record.
    fn key_of_context(&self, context: &BindingContext) -> Option<ContextKey> {
        let home = self.ancestor(context.scope_id)?;
        if context.path_prefix.is_empty() {
            return Some(home.context_key(vec![]));
        }
        let clarified = clarify_path(context.path_prefix.clone(), home).ok()?;
        match (clarified.head, clarified.tail) {
            (Some(chain), None) => Some(home.context_key(chain.get_links().to_vec())),
            _ => None,
        }
    }

    /// The record of an enclosing query that the rows of this scope's `SELECT` are related to, if
    /// they are: that of the CTE whose `SELECT` this scope compiles into.
    fn outer_record(&self) -> Option<&OuterRecord<'a>> {
        let mut scope = self;
        loop {
            if scope.outer_record.is_some() {
                return scope.outer_record.as_ref();
            }
            if !scope.shares_select_with_parent {
                return None;
            }
            scope = scope.parent?;
        }
    }

    /// Compiles `expr`, which was written within `context`: in the query of an enclosing scope,
    /// rather than the one this scope is compiling. That happens when an argument is passed to a
    /// function or a custom comparison whose body uses it within a path to related records, e.g.
    /// `title` in:
    ///
    /// ```text
    /// #issues.comment:@x = ++#comments{body:@x}
    /// #issues comment:`title`
    /// ```
    ///
    /// Here `title` is a column of the issue, but `@x` is referenced within the CTE that finds the
    /// issue's comments. Each row of that CTE is related to one issue (the CTE groups them by it),
    /// so the issue can be joined into the CTE, and the expression compiled against the join.
    ///
    /// That only works when the expression's record is the very one that the CTE's rows are related
    /// to. Otherwise (e.g. two levels of related records away) there is nothing to join, and an
    /// expression that refers to its record is an error. One that doesn't — a literal, say — needs
    /// no join, and compiles anywhere.
    fn convert_in_context(
        &mut self,
        expr: Expr,
        context: &BindingContext,
    ) -> Result<SqlExpr, Error> {
        let Some(home_table) = self.ancestor(context.scope_id).map(|home| home.base_table) else {
            // The expression's scope is no longer being compiled, which shouldn't happen. Its
            // record can't be reached, in any case.
            return Err(msg::outer_record_unreachable());
        };
        let key = self.key_of_context(context);
        // A scope standing in for the expression's record compiles it directly.
        if key.is_some() && key == self.stands_for {
            let saved_prefix = std::mem::take(&mut self.path_prefix);
            let saved_loc = std::mem::replace(&mut self.scoped_path_loc, Loc::NONE);
            let result = convert_expr(expr, self);
            self.path_prefix = saved_prefix;
            self.scoped_path_loc = saved_loc;
            return result;
        }
        let outer_record = self
            .outer_record()
            .filter(|outer| Some(&outer.key) == key.as_ref())
            .cloned();
        let Some(outer) = outer_record else {
            // The record can't be joined in, so the expression mustn't refer to it. To find out
            // whether it does, it's compiled within a throwaway scope based on its own table.
            let alias = self.get_alias(&home_table.name);
            let (result, refers_to_record) = {
                let mut probe = self.spawn_with_aliases(home_table, &alias, self.cloned_aliases());
                probe.path_prefix = context.path_prefix.clone();
                probe.scoped_path_loc = context.scoped_path_loc;
                probe.aggregate_ban = self.aggregate_ban;
                let result = convert_expr(expr, &mut probe);
                let refers_to_record = probe.base_alias_used.get() || !probe.join_tree.is_empty();
                (result, refers_to_record)
            };
            self.aliases.remove(&alias);
            let sql = result?;
            return match refers_to_record {
                true => Err(msg::outer_record_unreachable()),
                false => Ok(sql),
            };
        };
        // The expression is compiled within a scope of its own, based on its record's table as
        // joined into this scope's `SELECT`. The join is made only once it's needed, and is then
        // shared by every expression about the record.
        let (alias, join_tree, is_joined) = match self.stand_in.take() {
            Some(stand_in) => (stand_in.alias, stand_in.join_tree, true),
            None => {
                let alias = self.get_alias(&outer.table.name);
                let join_tree = JoinTree::new(alias.clone());
                (alias, join_tree, false)
            }
        };
        let (result, is_used, join_tree, aliases) = {
            let mut scope = self.spawn_with_aliases(outer.table, &alias, self.cloned_aliases());
            scope.aggregate_ban = self.aggregate_ban;
            scope.stands_for = Some(outer.key.clone());
            scope.join_tree = join_tree;
            let result = convert_expr(expr, &mut scope);
            let is_used = scope.base_alias_used.get() || !scope.join_tree.is_empty();
            let join_tree = std::mem::replace(&mut scope.join_tree, JoinTree::new(alias.clone()));
            (result, is_used, join_tree, scope.cloned_aliases())
        };
        if is_joined || is_used {
            self.reserve_aliases(aliases);
            self.stand_in = Some(StandIn { alias, join_tree });
        } else {
            self.aliases.remove(&alias);
        }
        result
    }

    /// Records that `expansion` is being expanded, until the returned guard is dropped. Fails with a
    /// descriptive error if it is already being expanded, which would mean the definition refers to
    /// itself (directly or indirectly) and would otherwise expand forever.
    pub fn enter_expansion(&self, expansion: Expansion) -> Result<ExpansionGuard, Error> {
        let mut state = self.expansion_state.borrow_mut();
        if let Some(index) = state.stack.iter().position(|e| *e == expansion) {
            return Err(Error::compile(
                expansion.cycle_message(&state.stack[index..]),
            ));
        }
        state.stack.push(expansion);
        Ok(ExpansionGuard {
            state: Rc::clone(&self.expansion_state),
            is_constant: false,
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

    /// Records the query's computed column definitions, validating that each refers to a real table.
    /// The definitions' expressions are resolved lazily (when referenced), so this does not validate
    /// the expressions themselves, and definition order does not matter.
    pub fn register_computed_columns(
        &mut self,
        computed_columns: Vec<ComputedColumn>,
    ) -> Result<(), Error> {
        for computed_column in computed_columns {
            let table = get_table_by_name(self.options, self.schema, &computed_column.table)
                .ok_or_else(|| {
                    msg::computed_column_unknown_table(&computed_column.table)
                        .at(computed_column.table_loc)
                })?;
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
    ) -> Result<(), Error> {
        for def in custom_comparisons {
            let table =
                get_table_by_name(self.options, self.schema, &def.table).ok_or_else(|| {
                    msg::custom_comparison_unknown_table(&def.table).at(def.table_loc)
                })?;
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

    /// The alias under which the base table appears in the SQL (see [`Scope::spawn_with_aliases`]),
    /// for generating SQL that refers to it.
    pub fn get_base_alias(&self) -> &str {
        self.base_alias_used.set(true);
        &self.base_alias
    }

    pub fn decompose_join_tree(&mut self) -> (Vec<Join>, Vec<Cte>) {
        let join_tree =
            std::mem::replace(&mut self.join_tree, JoinTree::new(self.base_alias.clone()));
        let (mut joins, mut ctes) = join_tree.decompose(self);
        // A record of an enclosing query that has been joined in (see `convert_in_context`).
        if let Some(stand_in) = self.stand_in.take() {
            // The unwrap is safe because a stand-in is only ever made for the outer record.
            let outer = self.outer_record().cloned().unwrap();
            joins.push(Join {
                table: outer.table.name.clone(),
                alias: stand_in.alias.clone(),
                conditions: build::cmp::eq(
                    self.table_column_expr(&outer.referrer_alias, &outer.referrer_column),
                    self.table_column_expr(&stand_in.alias, &outer.column),
                ),
                join_type: JoinType::LeftOuter,
            });
            let (stand_in_joins, stand_in_ctes) = stand_in.join_tree.decompose(self);
            joins.extend(stand_in_joins);
            ctes.extend(stand_in_ctes);
        }
        (joins, ctes)
    }

    /// Spawns a scope for compiling a query of its own (a CTE), based on `base_table`.
    pub fn spawn(&'b self, base_table: &'a Table) -> Self {
        let mut scope = self.spawn_with_aliases(base_table, &base_table.name, HashSet::new());
        scope.shares_select_with_parent = false;
        scope
    }

    /// Records the record that the rows of this scope's query are related to. See
    /// [`OuterRecord`].
    pub fn set_outer_record(&mut self, outer_record: OuterRecord<'a>) {
        self.outer_record = Some(outer_record);
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
        let id = {
            let mut state = self.expansion_state.borrow_mut();
            state.next_scope_id += 1;
            state.next_scope_id
        };
        Scope {
            id,
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
            env: self.env.clone(),
            globals: self.globals.clone(),
            expansion_state: Rc::clone(&self.expansion_state),
            user_functions: HashMap::new(),
            custom_comparisons: HashMap::new(),
            window_ban: self.window_ban,
            window_substitutions: Vec::new(),
            compiled_window: None,
            aggregate_ban: None,
            scoped_path_loc: Loc::NONE,
            shares_select_with_parent: true,
            outer_record: None,
            stands_for: None,
            base_alias_used: Cell::new(false),
            stand_in: None,
        }
    }

    /// Bans aggregates of the rows being queried while evaluating `f` (see [`AggregateBan`]),
    /// restoring the previous ban afterward.
    pub fn with_aggregate_ban<T>(
        &mut self,
        ban: AggregateBan,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.aggregate_ban.replace(ban);
        let result = f(self);
        self.aggregate_ban = previous;
        result
    }

    pub fn aggregate_ban(&self) -> Option<AggregateBan> {
        self.aggregate_ban
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

    /// Records that this scope has compiled a window function, written at `loc`.
    pub fn note_compiled_window(&mut self, loc: Loc) {
        self.compiled_window.get_or_insert(loc);
    }

    /// Where the first window function that this scope compiled was written, if it has compiled one.
    pub fn compiled_window(&self) -> Option<Loc> {
        self.compiled_window
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
    ) -> Result<SqlExpr, Error> {
        let starting_reference = chain.get_first_link().get_start();
        let starting_table_id = starting_reference.table_id;
        let starting_column_id = starting_reference.column_id;
        let starting_table = self.schema.tables.get(&starting_table_id).unwrap();
        let starting_column = starting_table.columns.get(&starting_column_id).unwrap();
        let head_links: &[LinkToOne] = head.as_ref().map_or(&[], |chain| chain.get_links());
        let ValueViaCte {
            select,
            value_alias,
            coalesce_to_zero,
        } = build_cte_select(
            head_links,
            chain,
            aggregate_expr_template_opt,
            self,
            purpose,
        )?;
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

/// Whether `name` (without the `@` sigil) is one of the built-in constants, like `@now`.
pub fn is_builtin_constant(name: &str) -> bool {
    [VAR_INFINITY, VAR_NOW, VAR_TRUE, VAR_FALSE, VAR_NULL].contains(&name)
}

fn get_table_by_name<'a>(options: &Options, schema: &'a Schema, name: &str) -> Option<&'a Table> {
    options
        .resolve_identifier(&schema.table_lookup, name)
        .map(|id| schema.tables.get(id).unwrap())
}
