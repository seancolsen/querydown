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
    // The bounds of a range are strings too (its `..` isn't a path separator), and a word before a
    // window condition is a search term (its `%%` doesn't pipe the word to an aggregate).
    let rendered = render(Conditions, r"title:a..m backend %%(project\p)%row_number:1");
    assert!(rendered.starts_with("title:column ::comparison-operator a:string"));
    assert!(
        rendered.contains(" m:string backend:search-term "),
        "{rendered}"
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
fn test_tables_in_other_schemas() {
    // A schema's names are part of the table's token, wherever the table is named.
    check(
        Query,
        "#/sales/orders ++#/crm/`old notes`/contacts $#/a/b/c/2 $>>/crm/x.y",
        "#/sales/orders:base-table ++:has-quantity #/crm/`old notes`/contacts:table \
         $:result-column #/a/b/c:table /:operator 2:number $:result-column >>:path-separator \
         /crm/x:table .:path-separator y:column",
    );
    check(
        Definitions,
        "#/sales/orders.total = 1",
        "#/sales/orders:table .:path-separator total:column =:definition 1:number",
    );
    // Without a schema, a `/` is still a division.
    check(
        Display,
        "$#issues/2",
        "$:result-column #issues:table /:operator 2:number",
    );
    // So is a `/` that doesn't continue a schema path, even in source that doesn't parse.
    check(
        Conditions,
        "++#/sales/ x",
        "++:has-quantity #/sales:table /:operator x:column",
    );
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
fn test_aggregate_arguments() {
    check(
        Display,
        "$#issues.x%percentile(0.5) $y%list(\\\\z \\d 1)",
        "$:result-column #issues:table .:path-separator x:column %:pipe \
         percentile:aggregate-function (:bracket 0.5:number ):bracket $:result-column y:column \
         %:pipe list:aggregate-function (:bracket \\\\:sort z:column \\d:flag 1:number ):bracket",
    );
}

#[test]
fn test_function_arguments() {
    check(
        Display,
        "$tags|map(@t @i => @t + @i) $@@some(tags @t => @t:x)",
        "$:result-column tags:column |:pipe map:function (:bracket @t:variable @i:variable \
         =>:definition @t:variable +:operator @i:variable ):bracket $:result-column \
         @@some:function (:bracket tags:column @t:variable =>:definition @t:variable \
         ::comparison-operator x:string ):bracket",
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
fn test_query_annotation() {
    check(
        Query,
        "#issues \\\\created_at \\\\\\{foo:100 'a b':[x @null]}",
        "#issues:base-table \\\\:sort created_at:column \\\\\\:annotation {:annotation-bracket \
         foo:annotation-key ::punctuation 100:annotation-value 'a b':annotation-key \
         ::punctuation [:annotation-bracket x:annotation-value @null:annotation-value \
         ]:annotation-bracket }:annotation-bracket",
    );
    // An unfinished one is lexed as an annotation too, rather than as a sort expression followed by
    // a stray `\`.
    check(
        Query,
        "#issues \\\\\\",
        "#issues:base-table \\\\\\:annotation",
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
        "a:-1.5 b:3d..1y6m c:@2024-01-31..<@now d:@null e:@limit f:\"it's\" g:'\\d+\\n'",
        "a:column ::comparison-operator -:number 1.5:number b:column ::comparison-operator \
         3d:duration ..:operator 1y6m:duration c:column ::comparison-operator \
         @2024-01-31:date ..:operator <:operator @now:constant d:column \
         ::comparison-operator @null:constant e:column ::comparison-operator @limit:variable \
         f:column ::comparison-operator \"it's\":string g:column ::comparison-operator \
         '\\d+\\n':string",
    );
    check(
        Conditions,
        "// line\na:1 /* block /* nested */ */",
        "// line:comment a:column ::comparison-operator 1:number \
         /* block /* nested */ */:comment",
    );
}

#[test]
fn test_flagged_strings() {
    check(
        Conditions,
        r#"a:^"x" b:^^it's "y"^ c:^/v\d+/ d:^e'\n\'\u{e9}' e:^f"{{x}}""#,
        r#"a:column ::comparison-operator ^:string-flags "x":string b:column ::comparison-operator ^:string-flags ^it's "y"^:string c:column ::comparison-operator ^:string-flags /v\d+/:string d:column ::comparison-operator ^e:string-flags ':string \n:string-escape \':string-escape \u{e9}:string-escape ':string e:column ::comparison-operator ^f:string-flags ":string {{:string-escape x:string }}:string-escape ":string"#,
    );
    // An interpolation is code, in which a bare word is a column even on the right of a comparison,
    // and in which the quote character doesn't end the string.
    check(
        Conditions,
        r#"title:^f|Re: {subject|upper} ({#comments})|"#,
        r#"title:column ::comparison-operator ^f:string-flags |Re: :string {:string-interpolation subject:column |:pipe upper:function }:string-interpolation  (:string {:string-interpolation #comments:table }:string-interpolation )|:string"#,
    );
    // Nested strings and condition sets within an interpolation.
    check(
        Display,
        r#"$^f"a{^f"b{c}"}d{ {x y} }""#,
        r#"$:result-column ^f:string-flags "a:string {:string-interpolation ^f:string-flags "b:string {:string-interpolation c:column }:string-interpolation ":string }:string-interpolation d:string {:string-interpolation {:bracket x:search-term y:search-term }:bracket }:string-interpolation ":string"#,
    );
}

#[test]
fn test_invalid_flagged_strings() {
    // A lone `}` in a formatted string, and a `^` that begins no string.
    check(
        Conditions,
        r#"a:^f"}" b:^x"#,
        r#"a:column ::comparison-operator ^f:string-flags ":string }:invalid ":string b:column ::comparison-operator ^:invalid x:column"#,
    );
    // Unterminated, and within an unterminated interpolation.
    check(
        Conditions,
        r#"a:^e"x\"#,
        r#"a:column ::comparison-operator ^e:string-flags "x:string \:string-escape"#,
    );
    check(
        Conditions,
        r#"a:^f"x{(y]"#,
        r#"a:column ::comparison-operator ^f:string-flags "x:string {:string-interpolation (:bracket y:column ]:bracket"#,
    );
}

#[test]
fn test_flagged_text_runs() {
    fn runs(source: &str) -> Vec<(&str, bool)> {
        flagged_text_runs(source)
            .into_iter()
            .map(|run| (&source[run.span], run.open))
            .collect()
    }
    assert_eq!(runs(r#"a:^"x" b"#), vec![(r#"^"x""#, false)]);
    assert_eq!(
        runs(r#"^f"a{b}c{^'d'}"#),
        vec![
            (r#"^f"a"#, true),
            ("}c", true),
            ("^'d'", false),
            ("}", true)
        ]
    );
    assert_eq!(
        runs(r#"^f"a{b}c""#),
        vec![(r#"^f"a"#, true), (r#"}c""#, false)]
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
fn test_union() {
    // Each query of a union has a base table, and `+++` is one token rather than `++` plus `+`.
    check(
        Query,
        "#issues $id $\"a\"+++ #blocks $blocker +++#blocks $blocking ~~~ $*",
        "#issues:base-table $:result-column id:column $:result-column \"a\":string +++:union \
         #blocks:base-table $:result-column blocker:column +++:union #blocks:base-table \
         $:result-column blocking:column ~~~:pipeline $:result-column *:glob",
    );
    // Invalid source still lexes `+++` as a union, not a quantity.
    check(
        Query,
        "#issues $id +++ ",
        "#issues:base-table $:result-column id:column +++:union",
    );
}

#[test]
fn test_regex_flags() {
    check(
        Conditions,
        r"title:\cm~'^a.b$' body:~x",
        "title:column ::comparison-operator \\cm:flag ~:comparison-operator '^a.b$':string \
         body:column :~:comparison-operator x:string",
    );
    // Within an expansion, bare words are still string literals.
    check(
        Conditions,
        r"title:\s~[a b]",
        "title:column ::comparison-operator \\s:flag ~:comparison-operator [:bracket a:string \
         b:string ]:bracket",
    );
}

#[test]
fn test_invalid_regex_flags() {
    // Unknown flags are still flags (the parser reports them), as is a backslash with none. A
    // missing `~` is tolerated, and a doubled one is taken whole.
    check(
        Conditions,
        r"a:\xi~1 b:\~2 c:\c d:\c~~3",
        "a:column ::comparison-operator \\xi:flag ~:comparison-operator 1:number b:column \
         ::comparison-operator \\:flag ~:comparison-operator 2:number c:column \
         ::comparison-operator \\c:flag d:string ::comparison-operator \\c:flag \
         ~~:comparison-operator 3:number",
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
    // A multi-byte character right after a number, where a duration's unit could be.
    check(
        Query,
        "#issues $ ? #comments:>0~'✅' ~~'❌'",
        "#issues:base-table $:result-column ?:case #comments:table :>:comparison-operator \
         0:number ~:case '✅':string ~~:case '❌':string",
    );
}

/// Tokenizing never panics and always upholds its guarantees, even midway through typing.
#[test]
fn test_every_prefix_is_well_formed() {
    let query = "@@f = @a => @a + 1\n#t.c = x|f\n#issues /* c */ title:~[\"a\\n\" b] body:\\cms~x \
                 ++#comments(author){x} due:@2024-01-01..<@now created:3d..1y6m \
                 \\\\created_at \\d $author.* $%count -> total \\s2d @{w:100 t:[a b] ok:@true} \
                 $? s:=1 ~ 'é' ~~ -1.5 $%%(a\\p d\\s)%sum(x) $q%percentile(0.5 \\s) ~~~ $*(id\\h) $`naïve`.( $y ) \
                 $^fe|a\\n{{{x|f(^'é')}}}| \
                 ~~~ ++#/s/`t u`/v $#/w/x/2 >>/y/z";
    for grammar in [Query, Definitions, Conditions, Sorting, Display] {
        for (end, _) in query.char_indices() {
            let source = &query[..end];
            assert_well_formed(source, &tokenize(source, grammar));
        }
    }
}

/// Tokenizing never panics with a multi-byte character anywhere in the source, e.g. where the lexer
/// looks a few bytes ahead. (A 4-byte character catches a lookahead of any length.)
#[test]
fn test_non_ascii_anywhere_is_well_formed() {
    let query =
        "#issues $id $ ? #comments:>0~'é' 90min ~~'❌' due:@2024-01-01 1y6m \\d x|f $@a ++#c";
    for (at, _) in query.char_indices().chain([(query.len(), ' ')]) {
        let source = format!("{}😀{}", &query[..at], &query[at..]);
        for grammar in [Query, Definitions, Conditions, Sorting, Display] {
            assert_well_formed(&source, &tokenize(&source, grammar));
        }
    }
}
