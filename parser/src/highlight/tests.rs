use super::*;

/// Renders the tokens of `source` as `text:kind` pairs separated by spaces, e.g.
/// `#issues:base-table title:column ...`.
fn render(grammar: Grammar, source: &str) -> String {
    let tokens = tokenize(source, grammar);
    assert_well_formed(source, &tokens);
    tokens
        .iter()
        .map(|t| format!("{}:{}", &source[t.span.clone()], t.kind))
        .collect::<Vec<_>>()
        .join(" ")
}

#[track_caller]
fn check(grammar: Grammar, source: &str, expected: &str) {
    let actual = render(grammar, source);
    assert!(
        actual == expected,
        "tokens mismatch for {source:?}\n--- expected:\n{expected}\n--- actual:\n{actual}\n"
    );
}

/// Asserts the guarantees documented on [`tokenize`]: tokens are in order, don't overlap, and cover
/// every non-whitespace character.
#[track_caller]
fn assert_well_formed(source: &str, tokens: &[Token]) {
    let mut covered = vec![false; source.len()];
    let mut previous_end = 0;
    for token in tokens {
        assert!(
            token.span.start >= previous_end,
            "tokens overlap or are out of order in {source:?}"
        );
        assert!(
            token.span.start < token.span.end,
            "empty token in {source:?}"
        );
        previous_end = token.span.end;
        covered[token.span.clone()].fill(true);
    }
    for (i, c) in source.char_indices() {
        assert!(
            c.is_whitespace() || covered[i],
            "{c:?} at {i} is not covered by a token in {source:?}"
        );
    }
}

use Grammar::*;

#[test]
fn test_bare_words_resolve_like_the_parser() {
    // A bare word is a column on the left of a comparison, a string on the right, and a search term
    // when it stands alone as a condition.
    check(
        Query,
        "#issues title:performance backend $id",
        "#issues:base-table title:column ::comparison-operator performance:string \
         backend:search-term $:result-column id:column",
    );
    // Words at any depth on the right-hand side are strings, but a backticked identifier or a
    // multi-part path is a column.
    check(
        Conditions,
        "title:~[color colour] title:foo|concat(bar) title:=`body` title:author.name",
        "title:column :~:comparison-operator [:bracket color:string colour:string ]:bracket \
         title:column ::comparison-operator foo:string |:pipe concat:function (:bracket \
         bar:string ):bracket title:column :=:comparison-operator `body`:column title:column \
         ::comparison-operator author:column .:path-separator name:column",
    );
    // Within a condition set, a lone word is a search term; the comma shorthand makes each of its
    // operands one, as does negation.
    check(
        Conditions,
        "[a b] {c} foo,bar !baz author{name:bob}",
        "[:bracket a:search-term b:search-term ]:bracket {:bracket c:search-term }:bracket \
         foo:search-term ,:operator bar:search-term !:operator baz:search-term author:column \
         {:bracket name:column ::comparison-operator bob:string }:bracket",
    );
    // A lone word that isn't a condition is a column, even though the comma shorthand's operand
    // parser is what reads it.
    check(
        Display,
        "$foo $foo|upper",
        "$:result-column foo:column $:result-column foo:column |:pipe upper:function",
    );
}

#[test]
fn test_tables() {
    check(
        Query,
        "#issues ++#comments(author){x} --#labels $>>project.name",
        "#issues:base-table ++:has-quantity #comments:table (:bracket author:column ):bracket \
         {:bracket x:search-term }:bracket --:has-quantity #labels:table $:result-column \
         >>:path-separator project:table .:path-separator name:column",
    );
    // A subquery has its own base table, and a user-defined table can be a base table.
    check(
        Query,
        "@n = #( #issues %count )\n#open = #( #issues $id )\n#open",
        "@n:variable =:definition #:table (:bracket #issues:base-table %:pipe \
         count:aggregate-function ):bracket #open:table =:definition #:table (:bracket \
         #issues:base-table $:result-column id:column ):bracket #open:base-table",
    );
    // Only a whole query has a base table.
    check(Conditions, "++#issues", "++:has-quantity #issues:table");
}

#[test]
fn test_definitions() {
    check(
        Definitions,
        "@x = 1\n@@plus = @a @b => @a + @b\n#users.age = birth|age\n\
         #issues.c:@q = ++#comments{body:@q}",
        "@x:variable =:definition 1:number @@plus:function =:definition @a:variable @b:variable \
         =>:definition @a:variable +:operator @b:variable #users:table .:path-separator \
         age:column =:definition birth:column |:pipe age:function #issues:table \
         .:path-separator c:column ::comparison-operator @q:variable =:definition \
         ++:has-quantity #comments:table {:bracket body:column ::comparison-operator \
         @q:variable }:bracket",
    );
}

#[test]
fn test_functions() {
    check(
        Display,
        "$created_at|age|days $labels.name%list(\\sd) $%count $@@max(a 0) \
         $x|(@d => @d + 1) $%%(account\\p date\\s)%sum(amount)",
        "$:result-column created_at:column |:pipe age:function |:pipe days:function \
         $:result-column labels:column .:path-separator name:column %:pipe \
         list:aggregate-function (:bracket \\sd:flag ):bracket $:result-column %:pipe \
         count:aggregate-function $:result-column @@max:function (:bracket a:column 0:number \
         ):bracket $:result-column x:column |:pipe (:bracket @d:variable =>:definition \
         @d:variable +:operator 1:number ):bracket $:result-column %%:pipe (:bracket \
         account:column \\p:flag date:column \\s:flag ):bracket %:pipe sum:aggregate-function \
         (:bracket amount:column ):bracket",
    );
}

#[test]
fn test_result_columns() {
    check(
        Display,
        "$author.* $*(id\\h) $%count -> total \\s2d @{width:100 tags:[a b] ok:@true} \
         $author.( $name )",
        "$:result-column author:column .:path-separator *:glob $:result-column *:glob \
         (:bracket id:column \\h:flag ):bracket $:result-column %:pipe count:aggregate-function \
         ->:operator total:alias \\s2d:flag @:annotation {:annotation-bracket \
         width:annotation-key ::punctuation 100:annotation-value tags:annotation-key \
         ::punctuation [:annotation-bracket a:annotation-value b:annotation-value \
         ]:annotation-bracket ok:annotation-key ::punctuation @true:annotation-value \
         }:annotation-bracket \
         $:result-column author:column .:path-separator (:bracket $:result-column name:column \
         ):bracket",
    );
}

#[test]
fn test_sorting() {
    check(
        Sorting,
        "\\\\created_at \\d \\\\author.( \\\\name \\dn )",
        "\\\\:sort created_at:column \\d:flag \\\\:sort author:column .:path-separator \
         (:bracket \\\\:sort name:column \\dn:flag ):bracket",
    );
}

#[test]
fn test_literals() {
    check(
        Conditions,
        "a:-1.5 b:3d..1y6m c:@2024-01-31..<@now d:@null e:@limit f:'it\\'s' g:\"\\u00e9\\q\"",
        "a:column ::comparison-operator -:number 1.5:number b:column ::comparison-operator \
         3d:duration ..:operator 1y6m:duration c:column ::comparison-operator \
         @2024-01-31:date ..:operator <:operator @now:constant d:column \
         ::comparison-operator @null:constant e:column ::comparison-operator @limit:variable \
         f:column ::comparison-operator 'it:string \\':string-escape s':string g:column \
         ::comparison-operator \":string \\u00e9:string-escape \\q:invalid \":string",
    );
    check(
        Conditions,
        "// line\na:1 /* block /* nested */ */",
        "// line:comment a:column ::comparison-operator 1:number \
         /* block /* nested */ */:comment",
    );
}

#[test]
fn test_operators() {
    // `*` is multiplication or a glob, and `-` is subtraction, negation, or (doubled) a quantity.
    check(
        Display,
        "$x*2 $x - -1 $*",
        "$:result-column x:column *:operator 2:number $:result-column x:column -:operator \
         -:number 1:number $:result-column *:glob",
    );
    check(
        Query,
        "#issues $? status:=\"open\" ~ 1 ~~ 0 ~~~ $*",
        "#issues:base-table $:result-column ?:case status:column :=:comparison-operator \
         \"open\":string ~:case 1:number ~~:case 0:number ~~~:pipeline $:result-column *:glob",
    );
}

#[test]
fn test_invalid_source_degrades_to_the_lexer() {
    // The first comparison parsed before the error, so the parser's marks stand; the rest is lexed.
    check(
        Query,
        "#issues title:foo [a b c:",
        "#issues:base-table title:column ::comparison-operator foo:string [:bracket a:column \
         b:column c:column ::comparison-operator",
    );
    // The lexer's look-behind heuristics cover the most common cases.
    check(
        Conditions,
        "title:~[color colour] $x|upper%max -> y &",
        "title:column :~:comparison-operator [:bracket color:string colour:string ]:bracket \
         $:result-column x:column |:pipe upper:function %:pipe max:aggregate-function \
         ->:operator y:alias &:invalid",
    );
    check(
        Conditions,
        "a:\"unterminated",
        "a:column ::comparison-operator \"unterminated:string",
    );
    // Here the parser itself reads `a /` as the start of a division, since the comment never closes.
    check(
        Conditions,
        "a /* unterminated",
        "a:column /* unterminated:comment",
    );
}

#[test]
fn test_grammars_differ() {
    check(Query, "#issues foo", "#issues:base-table foo:search-term");
    check(Display, "$foo", "$:result-column foo:column");
    // A display column in the conditions section doesn't parse, so it falls back to the lexer — but
    // the search term before it was parsed, so it keeps the parser's reading.
    check(
        Conditions,
        "foo $foo",
        "foo:search-term $:result-column foo:column",
    );
}

#[test]
fn test_non_ascii() {
    check(
        Conditions,
        "title:\"café\" `naïve`:1",
        "title:column ::comparison-operator \"café\":string `naïve`:column \
         ::comparison-operator 1:number",
    );
}

/// Tokenizing never panics and always upholds its guarantees, even midway through typing.
#[test]
fn test_every_prefix_is_well_formed() {
    let query = "@@f = @a => @a + 1\n#t.c = x|f\n#issues /* c */ title:~[\"a\\n\" b] \
                 ++#comments(author){x} due:@2024-01-01..<@now created:3d..1y6m \
                 \\\\created_at \\d $author.* $%count -> total \\s2d @{w:100 t:[a b] ok:@true} \
                 $? s:=1 ~ 'é' ~~ -1.5 $%%(a\\p d\\s)%sum(x) ~~~ $*(id\\h) $`naïve`.( $y )";
    for grammar in [Query, Definitions, Conditions, Sorting, Display] {
        for (end, _) in query.char_indices() {
            let source = &query[..end];
            assert_well_formed(source, &tokenize(source, grammar));
        }
    }
}
