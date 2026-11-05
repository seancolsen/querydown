import type { Grammar } from "../../src/index.js";

/** A snippet of Querydown to render, chosen to exercise some area of the highlighting. */
export interface Sample {
  name: string;
  title: string;
  grammar: Grammar;
  /** Whether the source is meant to be syntactically valid. The visual tests check this. */
  valid: boolean;
  /** Which highlighting to use: the default, a customized palette, or generic highlight styles. */
  highlighting?: "default" | "custom" | "standard";
  source: string;
}

/** A backtick, which can't be written directly within a template literal. */
const q = "`";

export const samples: Sample[] = [
  {
    name: "query",
    title: "A whole query",
    grammar: "query",
    valid: true,
    source: String.raw`// Recently active issues, newest first
@since = @now - 30d
#issues
  status:[open reopened] !wontfix
  title:~"crash|freeze" author.team.name:Backend
  ++#comments{created_at:>@since}
  \\created_at \d
  $id $title -> subject
  $author.username
  $#comments%count -> comments \sd`,
  },
  {
    name: "conditions",
    title: "Conditions",
    grammar: "conditions",
    valid: true,
    source: String.raw`accessibility "localhost:3000" foo,!bar
status:="do" title:=="Do" id:123 created_at:<6m
description:~'front[ -]?end\n' title:foo|concat(bar)
{title description}:~[foo bar] status:[open reopened]
created_at|year:2010..<2019 due_date:@2024-01-01..@2024-12-31
issue{title:dashboard} --#blocks(blocker) #issues:>0
${q}is_blocked${q} !${q}is_active${q} [status:"open" created_at:>1y6m]
/* block /* nested */ comment */ estimate:-1.5 deleted_at:@null`,
  },
  {
    name: "display",
    title: "Result columns",
    grammar: "display",
    valid: true,
    source: String.raw`$* $author.* $*(description \h) $id->Identifier
$project \g $created_at|year_month \g $%count -> issue_count \sd1
$title \s2 $@@max(due_date|age|days 0)
$#labels.name%list(\\name) $#comments.user.username%top(\\%count \d)
$%%(project\p created_at\sd)%row_number -> rn
$? #assignments:>0 ~ "assigned" ~~ "unassigned"
$(points * 2 + 1) / 3 $created_at|(@d => @d|year - 1)
$author.( $username $email ) $>>project.name
$title @{width:200 align:left tags:[a b] wrap:@true}`,
  },
  {
    name: "definitions",
    title: "Definitions",
    grammar: "definitions",
    valid: true,
    source: String.raw`@user_id = 1234
@latest = #( #comments $created_at%max )
@@fiscal_year = @date => (@date - 1m)|year
@@generation = @birth_date =>
  @year = @birth_date|year
  ? @year:<1965 ~ "Boomer" @year:<1981 ~ "Gen X" ~~ "Younger"
#users.age = birth_date|age|years|floor
#issues.comment:@x = ++#comments{body:@x}
#project_months = #( #issues $project \g $created_at|year_month \g $%count )`,
  },
  {
    name: "sorting",
    title: "Sorting",
    grammar: "sorting",
    valid: true,
    source: String.raw`\\priority \d \\due_date \n
\\author.( \\team.name \\username \dn )
\\#comments%count \d`,
  },
  {
    name: "pipeline",
    title: "A pipeline",
    grammar: "query",
    valid: true,
    source: String.raw`#issues $project \g $%count -> n
~~~ n:>10 $*`,
  },
  {
    name: "invalid",
    title: "Invalid and incomplete source",
    grammar: "query",
    valid: false,
    source: String.raw`#issues title:performance backend
  status:[open closed] & labels:~[bug
  $id $title|upper "bad \q escape" -> $`,
  },
  {
    name: "custom",
    title: "Customized colors",
    grammar: "query",
    valid: true,
    highlighting: "custom",
    source: String.raw`#issues status:open $title $author.username`,
  },
  {
    name: "standard",
    title: "A generic highlight style",
    grammar: "query",
    valid: true,
    highlighting: "standard",
    source: String.raw`// Generic styles apply via standard tags
#issues status:[open reopened] created_at:>@now - 7d
  \\created_at \d $title|upper $#comments%count -> n`,
  },
];
