//! Tests for autocompletion. Each source marks the cursor with `‸`.

use querydown_parser::highlight::Grammar;

use super::*;
use crate::tests::corpus_loader::build_options;
use crate::tests::get_test_resource;
use crate::IdentifierResolution;

const CURSOR: &str = "‸";

fn compiler() -> Compiler {
    compiler_for(&get_test_resource("issue_schema.json"))
}

fn compiler_for(schema_json: &str) -> Compiler {
    let options = build_options(IdentifierResolution::Flexible, "postgres");
    Compiler::new(schema_json, options).unwrap()
}

/// Completes `marked` (a source with the cursor marked) as a whole query.
fn complete(marked: &str) -> Completions {
    complete_with(marked, Grammar::Query, None, None)
}

/// Completes `marked` with the given grammar, base table and definitions.
fn complete_with(
    marked: &str,
    grammar: Grammar,
    base_table: Option<&str>,
    definitions: Option<&str>,
) -> Completions {
    let cursor = marked
        .find(CURSOR)
        .expect("the source should mark the cursor");
    let source = marked.replacen(CURSOR, "", 1);
    compiler().complete(&CompletionRequest {
        source: &source,
        cursor,
        grammar,
        base_table,
        definitions,
    })
}

/// Completes `marked` as a section of a query based on `#issues`.
fn complete_section(marked: &str, grammar: Grammar, definitions: Option<&str>) -> Completions {
    complete_with(marked, grammar, Some("issues"), definitions)
}

fn labels(completions: &Completions) -> Vec<&str> {
    completions.items.iter().map(|i| i.label.as_str()).collect()
}

/// The labels of the items of `kind`.
fn labels_of(completions: &Completions, kind: CompletionKind) -> Vec<&str> {
    completions
        .items
        .iter()
        .filter(|i| i.kind == kind)
        .map(|i| i.label.as_str())
        .collect()
}

fn find<'a>(completions: &'a Completions, label: &str) -> &'a CompletionItem {
    completions
        .items
        .iter()
        .find(|i| i.label == label)
        .unwrap_or_else(|| panic!("no item `{label}` in {:?}", labels(completions)))
}

const USERS: [&str; 6] = ["id", "username", "email", "team", "birth_date", "last_seen"];
const ISSUES: [&str; 9] = [
    "id",
    "title",
    "description",
    "created_at",
    "author",
    "status",
    "project",
    "duplicate_of",
    "due_date",
];
const COMMENTS: [&str; 5] = ["id", "issue", "user", "body", "created_at"];
const PROJECTS: [&str; 5] = ["id", "name", "product", "is_active", "is_pro_bono"];

/// Asserts that completing `marked` offers exactly the `columns`, in order.
#[track_caller]
fn assert_columns(marked: &str, columns: &[&str]) {
    let completions = complete(marked);
    assert_eq!(labels(&completions), columns, "completing {marked:?}");
}

// ------------------------------------------------------------------------------------------------
// Tables
// ------------------------------------------------------------------------------------------------

#[test]
fn base_table_offers_every_table() {
    let completions = complete("#‸");
    assert_eq!(completions.from, 1);
    assert_eq!(completions.to, 1);
    let tables = labels_of(&completions, CompletionKind::Table);
    assert_eq!(tables.len(), 11);
    assert!(tables.contains(&"issues") && tables.contains(&"issue_labels"));
    // Sorted by name.
    assert_eq!(tables[0], "assignments");
}

#[test]
fn base_table_completion_replaces_the_word_typed_so_far() {
    let completions = complete("#iss‸");
    assert_eq!((completions.from, completions.to), (1, 4));
    assert!(labels(&completions).contains(&"issues"));
}

#[test]
fn base_table_offers_user_defined_tables() {
    let completions = complete("#pm = #( #issues $project \\g $%count -> n )\n#‸");
    let pm = find(&completions, "pm");
    assert_eq!(pm.detail.as_deref(), Some("user-defined table"));
    assert!(labels(&completions).contains(&"issues"));
}

#[test]
fn related_tables_rank_directly_linked_tables_first() {
    let completions = complete("#issues $#‸");
    assert_eq!(find(&completions, "comments").boost, 1);
    assert_eq!(find(&completions, "users").boost, 1);
    assert_eq!(find(&completions, "clients").boost, 0);
}

#[test]
fn tables_in_a_path_follow_the_path() {
    // From `users` (via `author`), `teams` is directly linked but `blocks` isn't.
    let completions = complete("#issues $author.#‸");
    assert_eq!(find(&completions, "teams").boost, 1);
    assert_eq!(find(&completions, "blocks").boost, 0);
}

#[test]
fn definition_table() {
    let completions = complete("#us‸.age = birth_date|age|years\n#users");
    assert!(labels(&completions).contains(&"users"));
}

// ------------------------------------------------------------------------------------------------
// Columns
// ------------------------------------------------------------------------------------------------

#[test]
fn result_column_offers_base_table_columns() {
    let completions = complete("#issues $‸");
    assert_eq!(labels(&completions), ISSUES);
    let author = find(&completions, "author");
    assert_eq!(author.kind, CompletionKind::Column);
    assert_eq!(author.detail.as_deref(), Some("integer → users"));
    assert_eq!(find(&completions, "title").detail.as_deref(), Some("text"));
}

#[test]
fn columns_in_each_part_of_the_query() {
    // A condition (which would otherwise be a search term), sorting, and display.
    assert_columns("#issues ti‸", &ISSUES);
    assert_columns("#issues \\\\‸", &ISSUES);
    assert_columns("#issues status:open \\\\due_date $id $‸", &ISSUES);
    // Within a condition set, a comparison's left side, and arithmetic.
    assert_columns("#issues [status:open ‸]", &ISSUES);
    assert_columns("#issues ‸:open", &ISSUES);
    assert_columns("#issues $id + ‸", &ISSUES);
}

#[test]
fn cursor_within_a_word() {
    let completions = complete("#issues $ti‸tle $id");
    assert_eq!((completions.from, completions.to), (9, 11));
    assert_eq!(labels(&completions), ISSUES);
}

#[test]
fn columns_along_a_path() {
    assert_columns("#issues $author.‸", &USERS);
    assert_columns("#issues author.‸ $id", &USERS);
    assert_columns("#issues $author.team.‸", &["id", "name"]);
    assert_columns("#issues $project.product.client.na‸", &["id", "name"]);
    // A path through a column that isn't a link leads nowhere.
    assert_columns("#issues $title.‸", &[]);
}

#[test]
fn columns_of_related_tables() {
    assert_columns("#issues $#comments.‸", &COMMENTS);
    assert_columns("#issues $#comments.user.‸", &USERS);
    assert_columns("#issues ++#comments{‸}", &COMMENTS);
    assert_columns("#issues --#comments{body:x ‸}", &COMMENTS);
}

#[test]
fn scoped_conditions() {
    assert_columns("#comments issue{‸}", &ISSUES);
    assert_columns("#comments issue[title:a ‸]", &ISSUES);
    assert_columns("#comments issue{project{‸}}", &PROJECTS);
}

#[test]
fn linking_column_offers_foreign_keys() {
    let completions = complete("#issues --#blocks(‸)");
    assert_eq!(labels(&completions), ["blocker", "blocking"]);
    let blocker = find(&completions, "blocker");
    assert_eq!(blocker.detail.as_deref(), Some("→ issues"));
    assert_eq!(blocker.boost, 1);
}

#[test]
fn aggregate_sorting_is_relative_to_the_first_related_table() {
    assert_columns("#issues $#labels.name%list(\\\\‸)", &["id", "name"]);
    assert_columns(
        "#issues $#issue_labels.label.name%list_all(\\\\‸)",
        &["id", "issue", "label"],
    );
    // Without a related table, it's the current table.
    assert_columns("#issues $status \\g $title%list(\\\\‸)", &ISSUES);
}

#[test]
fn column_globs() {
    assert_columns("#issues $*(‸)", &ISSUES);
    assert_columns("#issues $author.*(‸ \\h)", &USERS);
}

#[test]
fn result_column_nesting() {
    assert_columns("#comments $issue.( $‸ )", &ISSUES);
    assert_columns("#comments $issue.( $id $project.( $‸ ) )", &PROJECTS);
}

#[test]
fn scoped_sorting() {
    assert_columns("#issues \\\\project.( \\\\‸ )", &PROJECTS);
}

#[test]
fn window_definitions_use_the_current_table() {
    assert_columns("#issues $%%(‸)%row_number", &ISSUES);
    assert_columns("#issues $%%(project\\p ‸)%row_number", &ISSUES);
}

#[test]
fn right_side_of_a_comparison() {
    // A bare word there is a string, which can't be completed…
    assert_columns("#issues status:op‸", &[]);
    assert_columns("#issues status:[open re‸]", &[]);
    // …but a path refers to columns.
    assert_columns("#issues description:author.‸", &USERS);
}

#[test]
fn unclosed_brackets_are_closed() {
    // These don't parse as they are, but do once their brackets are closed.
    assert_columns("#comments issue{ti‸", &ISSUES);
    assert_columns("#issues ++#comments{user:x ‸", &COMMENTS);
    assert_columns("#issues $#labels.name%list(\\\\‸", &["id", "name"]);
}

#[test]
fn invalid_source_falls_back_to_the_base_table() {
    // An error before the cursor that closing brackets can't fix.
    assert_columns("#issues title::: $‸", &ISSUES);
    assert_columns("#issues title::: $author.‸", &USERS);
}

#[test]
fn computed_columns() {
    let source = "#users.age = birth_date|age|years|floor\n#issues $author.‸";
    let completions = complete(source);
    let age = find(&completions, "age");
    assert_eq!(age.kind, CompletionKind::ComputedColumn);
    assert_eq!(
        labels_of(&completions, CompletionKind::Column),
        USERS,
        "the users table's own columns too"
    );
    // Only on the table that hosts it.
    assert_columns("#users.age = 1\n#issues $‸", &ISSUES);
}

#[test]
fn definition_bodies_are_relative_to_their_table() {
    assert_columns("#users.age = ‸", &USERS);
    assert_columns(
        "#users.age = birth_date|age\n#users.old = ‸\n#users",
        &[
            "id",
            "username",
            "email",
            "team",
            "birth_date",
            "last_seen",
            "age",
        ],
    );
    assert_columns("#issues.comment:@x = ++#comments{‸}\n#issues", &COMMENTS);
}

#[test]
fn custom_comparisons_are_offered_as_conditions() {
    let definitions = "#issues.comment:@x = ++#comments{body:@x}\n";
    let completions = complete(&format!("{definitions}#issues ‸"));
    assert_eq!(
        find(&completions, "comment").kind,
        CompletionKind::CustomComparison
    );
    let completions = complete(&format!("{definitions}#issues co‸:x"));
    assert!(labels(&completions).contains(&"comment"));
    // A custom comparison can't be a result column.
    assert_columns(&format!("{definitions}#issues $‸"), &ISSUES);
}

#[test]
fn special_custom_comparisons_are_not_offered() {
    let source = "#users.__querydown_linked_record_comparison:@x = username:@x\n#users ‸";
    assert_columns(source, &USERS);
}

#[test]
fn pipeline_stages_offer_the_previous_stages_columns() {
    let source = "#issues $project \\g $%count -> issue_count\n~~~\n‸";
    assert_columns(source, &["project", "issue_count"]);
    let source = "#issues $id $title\n~~~\n$id $title\n~~~\n$‸";
    assert_columns(source, &["id", "title"]);
    // Without result columns, a stage outputs all of its table's columns.
    assert_columns("#teams\n~~~\n$‸", &["id", "name"]);
}

#[test]
fn user_defined_tables_offer_their_columns() {
    let source = "#pm = #( #issues $project \\g $%count -> n )\n#pm n:>3 $‸";
    assert_columns(source, &["project", "n"]);
}

#[test]
fn subqueries_have_their_own_base_table() {
    assert_columns("@latest = #( #comments $‸ )\n#issues", &COMMENTS);
    assert_columns("#t = #( #teams $‸ )\n#t", &["id", "name"]);
}

#[test]
fn function_bodies_have_no_table() {
    assert_columns("@@f = @x => @x + ‸\n#issues", &[]);
    // But an anonymous function is applied in place.
    assert_columns("#issues $due_date|(@d => @d + ‸)", &ISSUES);
}

#[test]
fn names_that_need_quoting_are_inserted_with_backticks() {
    let schema = r#"{
        "tables": [{ "name": "Work Items", "columns": [
            { "name": "Due Date", "type": "date" },
            { "name": "title" }
        ]}],
        "links": []
    }"#;
    let compiler = compiler_for(schema);
    let source = "#`Work Items` $";
    let completions = compiler.complete(&CompletionRequest::query(source, source.len()));
    let due = find(&completions, "Due Date");
    assert_eq!(due.insert.as_deref(), Some("`Due Date`"));
    assert_eq!(due.detail.as_deref(), Some("date"));
    assert_eq!(find(&completions, "title").insert, None);
    assert_eq!(find(&completions, "title").detail, None);
    let completions = compiler.complete(&CompletionRequest::query("#", 1));
    let table = find(&completions, "Work Items");
    assert_eq!(table.insert.as_deref(), Some("`Work Items`"));
}

#[test]
fn flexible_identifiers_are_followed() {
    assert_columns("#Issues $Author.‸", &USERS);
}

// ------------------------------------------------------------------------------------------------
// Functions
// ------------------------------------------------------------------------------------------------

#[test]
fn scalar_functions_after_a_pipe() {
    let completions = complete("#issues $title|‸");
    let upper = find(&completions, "uppercase");
    assert_eq!(upper.kind, CompletionKind::Function);
    assert_eq!(upper.detail.as_deref(), Some("x|uppercase"));
    assert!(upper.info.is_some());
    assert_eq!(labels(&completions).len(), 38);
    // Also within a chain, and when followed by arguments.
    assert!(labels(&complete("#issues $due_date|age|d‸")).contains(&"days"));
    assert!(labels(&complete("#issues $title|con‸(\"x\")")).contains(&"concat"));
}

#[test]
fn direct_calls() {
    let completions = complete("#issues $@@‸");
    assert!(labels(&completions).contains(&"max"));
    let completions = complete("#issues $@@ma‸(due_date 0)");
    assert!(labels(&completions).contains(&"max"));
}

#[test]
fn user_defined_functions() {
    let source = "@@fiscal_year = @date @offset => (@date - 1m)|year\n#issues $created_at|‸";
    let completions = complete(source);
    let fy = find(&completions, "fiscal_year");
    assert_eq!(fy.kind, CompletionKind::Function);
    assert_eq!(fy.detail.as_deref(), Some("@date @offset => …"));
}

#[test]
fn aggregate_functions() {
    let completions = complete("#issues $#comments.created_at%‸");
    assert_eq!(
        find(&completions, "max").kind,
        CompletionKind::AggregateFunction
    );
    assert_eq!(labels(&completions).len(), 12);
    assert!(labels(&complete("#issues $status \\g $%c‸")).contains(&"count"));
}

#[test]
fn window_functions() {
    let completions = complete("#issues $%%(project\\p created_at\\s)%‸");
    assert_eq!(
        find(&completions, "row_number").kind,
        CompletionKind::WindowFunction
    );
    assert_eq!(labels(&completions).len(), 16);
}

// ------------------------------------------------------------------------------------------------
// Constants and variables
// ------------------------------------------------------------------------------------------------

#[test]
fn constants_and_user_defined_constants() {
    let completions = complete("@user_id = 1234\n#issues author:@‸");
    assert_eq!(
        labels_of(&completions, CompletionKind::Constant),
        ["now", "infinity", "true", "false", "null"]
    );
    assert_eq!(
        labels_of(&completions, CompletionKind::Variable),
        ["user_id"]
    );
    assert_eq!(complete("#issues created_at:<@n‸").from, 21);
}

#[test]
fn function_parameters_and_assignments() {
    let source = "@@f = @birth_date =>\n  @year = @birth_date|year\n  @‸\n#users";
    let completions = complete(source);
    assert_eq!(
        labels_of(&completions, CompletionKind::Variable),
        ["birth_date", "year"]
    );
    let completions = complete("#issues $due_date|(@days => @‸)");
    assert_eq!(labels_of(&completions, CompletionKind::Variable), ["days"]);
    let completions = complete("#issues.comment:@x = ++#comments{body:@‸}\n#issues");
    assert_eq!(labels_of(&completions, CompletionKind::Variable), ["x"]);
}

#[test]
fn parameters_are_not_in_scope_outside_their_function() {
    let completions = complete("@@f = @x => @x\n#issues author:@‸");
    assert_eq!(
        labels_of(&completions, CompletionKind::Variable),
        Vec::<&str>::new()
    );
}

// ------------------------------------------------------------------------------------------------
// Where nothing is offered
// ------------------------------------------------------------------------------------------------

#[test]
fn nothing_within_strings_comments_numbers_flags_or_new_names() {
    for marked in [
        "#issues title:\"fo‸",
        "#issues title:\"fo‸\"",
        "#issues $`ti‸",
        "#issues // $‸",
        "#issues /* $‸ */",
        "#issues id:1‸",
        "#issues created_at:<3‸",
        "#issues $title \\s‸",
        "#issues $title -> ‸",
        "@con‸ = 1\n#issues",
        "@@fun‸ = @x => @x\n#issues",
        "#users.ag‸ = 1\n#users",
        "#issues $id|(@para‸ => 1)",
    ] {
        assert_eq!(labels(&complete(marked)), Vec::<&str>::new(), "{marked:?}");
    }
}

#[test]
fn after_a_closed_string_or_comment() {
    assert_columns("#issues title:\"x\" ‸", &ISSUES);
    assert_columns("#issues /* note */ ‸", &ISSUES);
}

// ------------------------------------------------------------------------------------------------
// Sections
// ------------------------------------------------------------------------------------------------

#[test]
fn sections_use_the_given_base_table_and_definitions() {
    let completions = complete_section("ti‸", Grammar::Conditions, None);
    assert_eq!(labels(&completions), ISSUES);
    let completions = complete_section("\\\\author.‸", Grammar::Sorting, None);
    assert_eq!(labels(&completions), USERS);
    let definitions = Some("#issues.overdue = due_date:<@now\n@@double = @x => @x * 2");
    let completions = complete_section("$id|‸", Grammar::Display, definitions);
    assert!(labels(&completions).contains(&"double"));
    let completions = complete_section("$‸", Grammar::Display, definitions);
    assert_eq!(
        find(&completions, "overdue").kind,
        CompletionKind::ComputedColumn
    );
    let completions = complete_section("#users.age = ‸", Grammar::Definitions, None);
    assert_eq!(labels(&completions), USERS);
}

#[test]
fn invalid_sections_fall_back_to_the_given_base_table() {
    let completions = complete_section("title::: ‸", Grammar::Conditions, None);
    assert_eq!(labels(&completions), ISSUES);
}

#[test]
fn sections_based_on_a_user_defined_table() {
    let definitions = Some("#pm = #( #issues $project \\g $%count -> n )");
    let completions = complete_with("$‸", Grammar::Display, Some("pm"), definitions);
    assert_eq!(labels(&completions), ["project", "n"]);
}

// ------------------------------------------------------------------------------------------------
// Annotations
// ------------------------------------------------------------------------------------------------

fn annotation(marked: &str) -> (AnnotationPosition, usize) {
    let completions = complete(marked);
    assert!(completions.items.is_empty());
    let position = completions
        .annotation
        .unwrap_or_else(|| panic!("{marked:?} should be within an annotation"));
    (position, completions.from)
}

fn key(name: &str) -> AnnotationPathSegment {
    AnnotationPathSegment::Key(name.to_string())
}

#[test]
fn annotation_keys() {
    let (position, from) = annotation("#issues $title @{‸");
    assert_eq!(position.path, []);
    assert_eq!(position.slot, AnnotationSlot::Key);
    assert_eq!(from, 17);
    let (position, _) = annotation("#issues $title @{width:100 'text color':red fo‸}");
    assert_eq!(position.slot, AnnotationSlot::Key);
    assert_eq!(position.keys, ["width", "text color"]);
}

#[test]
fn annotation_values() {
    let (position, _) = annotation("#issues $title @{width:‸");
    assert_eq!(position.path, [key("width")]);
    assert_eq!(position.slot, AnnotationSlot::Value);
    let (position, from) = annotation("#issues $title @{sortable: @tr‸}");
    assert_eq!(position.path, [key("sortable")]);
    assert_eq!(from, 27, "includes the `@`");
}

#[test]
fn nested_annotations() {
    let marked = "#issues $#comments @{rules:[\n  {gte:10 bg:'#fbc9ff'}\n  {‸";
    let (position, _) = annotation(marked);
    assert_eq!(
        position.path,
        [key("rules"), AnnotationPathSegment::Index(1)]
    );
    assert_eq!(position.slot, AnnotationSlot::Key);
    let (position, _) = annotation("#issues $id @{a:{b:[x ‸");
    assert_eq!(
        position.path,
        [key("a"), key("b"), AnnotationPathSegment::Index(1)]
    );
    assert_eq!(position.slot, AnnotationSlot::Value);
}

#[test]
fn outside_annotations() {
    // After the annotation is closed, or between a key and its `:`, or within a string.
    assert_columns("#issues $title @{width:100} $‸", &ISSUES);
    let completions = complete("#issues $title @{width ‸");
    assert_eq!((completions.annotation, completions.items.len()), (None, 0));
    let completions = complete("#issues $title @{format:'YY‸");
    assert_eq!((completions.annotation, completions.items.len()), (None, 0));
}

#[test]
fn offsets_are_bytes() {
    // `é` is two bytes.
    let completions = complete("#issues title:\"é\" $author.us‸");
    assert_eq!((completions.from, completions.to), (27, 29));
    assert_eq!(labels(&completions), USERS);
}
