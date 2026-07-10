use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::utils::*;

use super::condition_set::condition_set;
use super::path::path;

/// Parses a "scoped comparison": a path immediately (no space) followed by a condition set, e.g.
/// `issue{title:dashboard}`. This produces an [`Expr::ScopedConditionSet`] carrying the path and the
/// condition set verbatim; the scoping itself is resolved in the compiler, which evaluates every
/// entry as though the path had been written in front of it.
///
/// Resolving the scope in the compiler (rather than desugaring the path onto each entry here) is what
/// lets the scope contain _anything_ you could write as a top-level condition on the related table —
/// including a bare [default text search](Expr::String) term, which has no leading column reference
/// to prefix and no flat-syntax equivalent. So `issue{dashboard}` searches the issue's text columns.
///
/// The **absence of a space** is what distinguishes this from `issue {title:dashboard}`, in which
/// `issue` is a default text-search term and `{title:dashboard}` a separate condition set. That
/// distinction falls out of the grammar: `path` does not consume trailing whitespace, and the
/// condition set must follow it directly. (A spaced word never reaches this parser at all, because a
/// bare word followed by a space is consumed as a search term earlier — see `bare_word`.)
pub fn scoped_comparison<'src>(expr: impl Psr<'src, Expr> + 'src) -> impl Psr<'src, Expr> {
    path(expr.clone())
        .then(condition_set(expr))
        .map(|(path, condition_set)| {
            Expr::ScopedConditionSet(ScopedConditionSet {
                path,
                condition_set,
            })
        })
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::parse_conditions;

    /// Parses the single condition-set entry produced by a scoped comparison, asserting that the
    /// whole input is one scoped comparison, and returns its [`ScopedConditionSet`].
    fn parse_scoped(input: &str) -> ScopedConditionSet {
        let conditions = parse_conditions(input).unwrap();
        assert_eq!(conditions.entries.len(), 1, "expected one entry: {input}");
        match conditions.entries.into_iter().next().unwrap() {
            Expr::ScopedConditionSet(scoped) => scoped,
            other => panic!("expected a scoped condition set, got {other:?} for: {input}"),
        }
    }

    #[test]
    fn captures_path_and_condition_set_verbatim() {
        // The path and condition set are captured as written — no desugaring — with `{ }` yielding an
        // AND set and `[ ]` an OR set.
        let scoped = parse_scoped("issue{title:dashboard}");
        assert_eq!(scoped.path, vec![PathPart::Column("issue".to_string())]);
        assert_eq!(scoped.condition_set.conjunction, Conjunction::And);
        assert_eq!(
            scoped.condition_set.entries,
            vec![Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("title".to_string())])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::String("dashboard".to_string())),
            }))]
        );

        assert_eq!(
            parse_scoped("issue[a:1 b:2]").condition_set.conjunction,
            Conjunction::Or
        );
    }

    #[test]
    fn a_multi_part_head_is_captured() {
        let scoped = parse_scoped("issue.project{name:x}");
        assert_eq!(
            scoped.path,
            vec![
                PathPart::Column("issue".to_string()),
                PathPart::Column("project".to_string()),
            ]
        );
    }

    #[test]
    fn a_bare_word_entry_is_kept_as_a_search_term() {
        // A bare word inside the scope is a default text-search term (an `Expr::String`), exactly as
        // it would be at the top level. This is the case that cannot be desugared and must be scoped
        // in the compiler.
        let scoped = parse_scoped("issue{dashboard}");
        assert_eq!(
            scoped.condition_set.entries,
            vec![Expr::String("dashboard".to_string())]
        );
    }

    #[test]
    fn scopes_nest() {
        // A scoped comparison is an ordinary expression, so it can itself be an entry of a scope.
        let outer = parse_scoped("issue{project{name:x}}");
        assert_eq!(outer.path, vec![PathPart::Column("issue".to_string())]);
        assert!(matches!(
            outer.condition_set.entries.as_slice(),
            [Expr::ScopedConditionSet(_)]
        ));
    }

    #[test]
    fn requires_no_space_before_the_brace() {
        // With a space, `issue` is a default text-search term and `{id:7}` a separate condition set —
        // two entries — rather than a single scoped comparison.
        let spaced = parse_conditions("issue {id:7}").unwrap();
        assert_eq!(spaced.entries.len(), 2);
        assert_eq!(spaced.entries[0], Expr::String("issue".to_string()));
        assert!(matches!(spaced.entries[1], Expr::ConditionSet(_)));
    }

    #[test]
    fn works_inside_an_outer_condition_set() {
        // A scoped comparison nests inside `[ ]`/`{ }` like any other boolean expression. The
        // top-level conditions are an implicit AND set whose sole entry here is the explicit `[ ]`.
        let conditions = parse_conditions("[issue{title:x} body:y]").unwrap();
        let [Expr::ConditionSet(or_set)] = conditions.entries.as_slice() else {
            panic!(
                "expected a single OR condition set, got {:?}",
                conditions.entries
            );
        };
        assert_eq!(or_set.conjunction, Conjunction::Or);
        assert!(matches!(
            or_set.entries.as_slice(),
            [Expr::ScopedConditionSet(_), Expr::Comparison(_)]
        ));
    }
}
