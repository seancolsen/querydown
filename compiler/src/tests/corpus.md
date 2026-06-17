# E2E test corpus

```toml options
schema = "issues"
identifier_resolution = "flexible"
```

- ⛔ = skip
- 🔦 = solo

## Simple

> Show all issue ids

```qd
#issues $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues";
```

## Simple library schema

```toml options
schema = "library"
```

> Show all patrons

```qd
#Patrons
```

```sql
SELECT "Patrons".* FROM "Patrons";
```

## Large examples

### ⛔ Main README

```qd
#issues
created_at:>@6M|ago
--#assignments
++#labels{name:..["Regression" "Bug"]}
10..20:#comments{!user.team.name:"Backend"}
$*
$author.username
$#comments.created_at%min \sd
```

```
TODO
```

## Flexible identifiers

```toml options
schema = "library"
```

### Simplest flexible identifier

> All checkouts

```qd
#checkouts
```

```sql
SELECT
  "Checkouts".*
FROM "Checkouts";
```

### snake_case

> Checkouts from over one month ago and not yet returned

```qd
#checkouts check_in_time:@null check_out_time:<@1M|ago
```

```sql
SELECT
  "Checkouts".*
FROM "Checkouts"
WHERE
  "Checkouts"."Check In Time" IS NULL AND
  "Checkouts"."Checkout Time" < NOW() - make_interval(months => 1);
```

### camelCase

> Checkouts from over one month ago and not yet returned

```qd
#checkouts checkInTime:@null checkOutTime:<@1M|ago
```

```sql
SELECT
  "Checkouts".*
FROM "Checkouts"
WHERE
  "Checkouts"."Check In Time" IS NULL AND
  "Checkouts"."Checkout Time" < NOW() - make_interval(months => 1);
```

### Complex flexible identifiers

```qd
#items
++#checkouts{check_in_time:@null patron.first_name:"Foo"}
book.page_count:>200
```

```sql
WITH
  "cte0" AS (
    SELECT
      "Checkouts"."Item" AS "pk"
    FROM "Checkouts"
    LEFT JOIN "Patrons" ON
      "Checkouts"."Patron" = "Patrons"."id"
    WHERE
      "Checkouts"."Check In Time" IS NULL AND
      "Patrons"."First Name" = 'Foo'
    GROUP BY "Checkouts"."Item"
  )
SELECT
  "Items".*
FROM "Items"
LEFT JOIN "cte0" ON
  "Items"."id" = "cte0"."pk"
LEFT JOIN "Books" ON
  "Items"."Book" = "Books"."id"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "Books"."Page Count" > 200;
```

## Values

### Date

> Issues created since 2023-01-01

```qd
#issues created_at:>=@2023-01-01
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" >= DATE '2023-01-01';
```

### Duration

```qd
#issues created_at:>@6Y|ago
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" > NOW() - make_interval(years => 6);
```

### Duration, lowercase

```qd
#issues created_at:>@6y|ago
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" > NOW() - make_interval(years => 6);
```

## Comparisons

### Negated comparison

> Issues whose status is not "open"

```qd
#issues !status:"open"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  NOT "issues"."status" = 'open';
```

### Negated expression as a result column

> Whether each project is inactive, as a boolean column

```qd
#projects $!is_active->inactive
```

```sql
SELECT
  NOT "projects"."is_active" AS "inactive"
FROM "projects";
```

### Negated condition set

> Issues that are not both open and high priority

```qd
#issues !{status:"open" status:"high"}
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  NOT (
    "issues"."status" = 'open' AND
    "issues"."status" = 'high'
  );
```

### Regex

> Issues with titles containing "foo"

```qd
#issues title:~"foo"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."title" ~* 'foo';
```

### Expansion

```qd
#issues title:~..["color" "colour"]
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  ("issues"."title" ~* 'color' OR "issues"."title" ~* 'colour');
```

### Dual expansion

```qd
#issues {title description}..:~..["color" "colour"]
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  ("issues"."title" ~* 'color' OR "issues"."title" ~* 'colour') AND
  ("issues"."description" ~* 'color' OR "issues"."description" ~* 'colour');
```

### Regex (DuckDB)

```toml options
dialect = "duckdb"
```

> Issues with titles containing "foo", targeting DuckDB

```qd
#issues title:~"foo"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  regexp_matches("issues"."title", 'foo', 'i');
```

### Duration (DuckDB)

```toml options
dialect = "duckdb"
```

> DuckDB has no `make_interval`, so a single-part duration uses a `to_*` function.

```qd
#issues created_at:>@6Y|ago
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" > NOW() - to_years(6);
```

### Multi-part duration (DuckDB)

```toml options
dialect = "duckdb"
```

> A multi-part duration sums `to_*` functions, parenthesized so it stays atomic when subtracted.

```qd
#issues created_at:>@1Y2D|ago
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" > NOW() - (to_years(1) + to_days(2));
```

### String escaping (DuckDB)

```toml options
dialect = "duckdb"
```

> DuckDB escapes a single-quote by doubling it, rather than with a backslash.

```qd
#issues title:"can't"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."title" = 'can''t';
```

### Infinity

> The `@infinity` constant casts the `'infinity'` string literal, since a bare keyword is invalid.

```qd
#issues id:<@infinity
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."id" < CAST('infinity' AS double precision);
```

### Simple range

```qd
#issues id:50..100
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."id" >= 50 AND
  "issues"."id" <= 100;
```

### Range with exclusive ends

```qd
#issues created_at:@2000-01-01<..<@2010-01-01
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" > DATE '2000-01-01' AND
  "issues"."created_at" < DATE '2010-01-01';
```

### Range containing pipes

```qd
#issues created_at:(@2Y|ago)..(@1Y|ago)
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" >= NOW() - make_interval(years => 2) AND
  "issues"."created_at" <= NOW() - make_interval(years => 1);
```

### Range vs expansion

```qd
#comments [created_at issue.created_at]..:@2000-01-01..<@2000-02-01
```

```sql
SELECT
  "comments".*
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  (
    "comments"."created_at" >= DATE '2000-01-01' AND "comments"."created_at" < DATE '2000-02-01'
    OR
    "issues"."created_at" >= DATE '2000-01-01' AND "issues"."created_at" < DATE '2000-02-01'
  );
```


## Condition sets

### "Has some" with "OR"

This test is part of a bug fix. Previously, we were using `JOIN` instead of `LEFT JOIN` when joining "has some" related tables because that produced simpler SQL. But that didn't work when the condition was nested inside an `OR` condition set. We use `LEFT JOIN` plus a `WHERE` condition because it seems less prone to bugs.

> Issues that have labels or comments

```qd
#issues [++#labels ++#comments]
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    GROUP BY "issue_labels"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues".*
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  ("cte0"."pk" IS NOT NULL OR "cte1"."pk" IS NOT NULL);
```

### OR shorthand with comma

The comma is shorthand for an "OR" condition set, equivalent to wrapping the conditions in `[ ]`.

> Issues that are open or created after 2023-03-04

```qd
#issues status:"open",created_at:>@2023-03-04
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  ("issues"."status" = 'open' OR "issues"."created_at" > DATE '2023-03-04');
```

## Paths to one

### Joined column in related table

> Issues under project named "foo".

```qd
#issues project.name:"foo" $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  "projects"."name" = 'foo';
```

### Comparing an FK column to NULL

```qd
#issues author:@null
```

```sql
SELECT
 "issues".*
FROM "issues"
WHERE
  "issues"."author" IS NULL;
```

### ⛔ Referenced column in related table should not be joined

This test case ensures that we don't have an unnecessary join on `projects` when the `projects.id` value can already be found within `issues.project`.

**TODO** This is not yet implemented. We need to make some changes within `build_linked_path` to optimize for this case. The SQL we're producing still works even though this test case is not satisfied. We're just producing SQL that has a superfluous join.

> Issues under project with id 1.

```qd
#issues project.id:1 $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues"
WHERE
  "issues"."project" = 1;
```

## Paths to many

### Path to many with column at end

> Issues, showing the date of their most recent comment.

```qd
#issues $id $#comments.created_at%max->most_recent_comment
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1" AS "most_recent_comment"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Path through one, many

> Issues, showing the total number of comments that the issue's author has made across all issues

```qd
#issues $id->id $author.#comments->total_comments_by_author
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    GROUP BY "comments"."user"
  )
SELECT
  "issues"."id" AS "id",
  "cte0"."v1" AS "total_comments_by_author"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Path through many, many

> Users, showing the date of the most recent comment made across all the tickets the user has created.

```qd
#users $id->id $#issues.#comments.created_at%max->v
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "issues"
    JOIN "comments" ON
      "issues"."id" = "comments"."issue"
    GROUP BY "issues"."author"
  )
SELECT
  "users"."id" AS "id",
  "cte0"."v1" AS "v"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Path through many, one, many

> Projects, showing the date of the most recent comment made by users who have ever created tickets associated with the project.

```qd
#projects $id->id $#issues.author.#comments.created_at%max->v
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "issues"
    JOIN "users" ON
      "issues"."author" = "users"."id"
    JOIN "comments" ON
      "users"."id" = "comments"."user"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."id" AS "id",
  "cte0"."v1" AS "v"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### List aggregate (array_agg)

> Issues with an array of their label names.

```qd
#issues $id $#issue_labels.label.name%list
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg("labels"."name") AS "v1"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### List aggregate with ORDER BY

> Issues with an array of their label names sorted alphabetically.

```qd
#issues $id $#issue_labels.label.name%list(\\name)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg("labels"."name" ORDER BY "labels"."name" ASC NULLS LAST) AS "v1"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### List aggregate with descending ORDER BY

> Issues with an array of their label names sorted reverse-alphabetically.

```qd
#issues $id $#issue_labels.label.name%list(\\name \d)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg("labels"."name" ORDER BY "labels"."name" DESC NULLS LAST) AS "v1"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Multiple CTEs

> Issues that have comments and assignments

```qd
#issues ++#comments ++#assignments
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "assignments"."issue" AS "pk"
    FROM "assignments"
    GROUP BY "assignments"."issue"
  )
SELECT
  "issues".*
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NOT NULL;
```

## "Has" conditions

### Basic has some

> Issues that have comments

```qd
#issues ++#comments
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues".*
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Basic has none

> Users who have not authored any issues

```qd
#users --#issues
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk"
    FROM "issues"
    GROUP BY "issues"."author"
  )
SELECT
  "users".*
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### Double has none

> Users who have not created any tickets which have comments

```qd
#users --#issues.#comments
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk"
    FROM "issues"
    JOIN "comments" ON
      "issues"."id" = "comments"."issue"
    GROUP BY "issues"."author"
  )
SELECT
  "users".*
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### Double has some

> Users who have created at least one ticket which has at least one comment

```qd
#users ++#issues.#comments
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk"
    FROM "issues"
    JOIN "comments" ON
      "issues"."id" = "comments"."issue"
    GROUP BY "issues"."author"
  )
SELECT
  "users".*
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### ⛔ Has through inferred intermediate

FIXME there is a bug here


```qd
#issues ++#labels
```

```sql
TODO
```


## Filtered paths

### Simple filtered path in has none

> Users who have not created any issues in the past year

```qd
#users --#issues{created_at:>@1Y|ago}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk"
    FROM "issues"
    WHERE
      "issues"."created_at" > NOW() - make_interval(years => 1)
    GROUP BY "issues"."author"
  )
SELECT
  "users".*
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### Simple filtered path for value

> Users, showing the number of issues created in the past year

```qd
#users $#issues{created_at:>@1Y|ago}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      "issues"."created_at" > NOW() - make_interval(years => 1)
    GROUP BY "issues"."author"
  )
SELECT
  "cte0"."v1"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Filtered path through inferred intermediate

> Issues that are not labeled bug

```qd
#issues --#labels{name:"bug"} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    WHERE
      "labels"."name" = 'bug'
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### ⛔A filter that aligns with the join

> Issues, showing the total number of comments made on the issue by the issue's author

```qd
#issues $#comments{user:issue.author}
```

```sql
TODO
```

### ⛔Nested filter

> Clients that don't have any issues without comments

```qd
#clients --#issues{--#comments}
```

```sql
TODO
```

## Column control flags

### Basic sort

> Issues, showing the most recent ones first

```qd
#issues $id $title $created_at \sd
```

```sql
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."created_at"
FROM "issues"
ORDER BY
  "issues"."created_at" DESC NULLS LAST;
```

## Sorting outside of result columns

### Basic standalone sort

> Issue titles, sorted with the most recently-created issues first, without showing the creation date

```qd
#issues
\\created_at \d
$title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."created_at" DESC NULLS LAST;
```

### Multiple standalone sorts

> Precedence follows the order in which the sorting expressions are listed. The `n` flag sorts NULL values first.

```qd
#issues
\\status
\\created_at \dn
$title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."status" ASC NULLS LAST,
  "issues"."created_at" DESC NULLS FIRST;
```

### Standalone sort with a computed expression

> Issue titles, sorted by the length of their description, longest first

```qd
#issues
\\description|length \d
$title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  char_length("issues"."description") DESC NULLS LAST;
```

### Mixing standalone and column sorts

> Standalone `\\` sorts take precedence over column `\s` sorts, so they come first in the ORDER BY.

```qd
#issues
\\created_at \d
$title \s
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."created_at" DESC NULLS LAST,
  "issues"."title" ASC NULLS LAST;
```

## Column globs

### Basic column glob

> Issues, showing all columns

```qd
#issues $*
```

```sql
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues";
```

### Complex column glob

> Issues, showing all columns

```qd
#issues
$*(
  id->identifier
  title \sd
  duplicateOf \h
  "this has no effect"
  description|length \sd1
)
$author.*(username \sd1)
```

```sql
SELECT
  "issues"."id" AS "identifier",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."due_date",
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
ORDER BY
  char_length("issues"."description") DESC NULLS LAST,
  "users"."username" DESC NULLS LAST,
  "issues"."title" DESC NULLS LAST;
```

## Column annotations

A test case may include an optional ` ```json ` block after the SQL block. When
present, the harness compares its `columnAnnotations` against the compiler output.

### Full example

```qd
#issues
$title @{width:100}
$created_at @{formatter:timeElapsed textColor:light}
$due_date @{format:'YYYY-MM-DD' datePicker:@true}
$#comments @{formattingConditions:[
  {gte:10 bg:'#fbc9ff'}
  {gte:5 bg:'#d5d2ff'}
]}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title",
  "issues"."created_at",
  "issues"."due_date",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

```json
{
  "columnAnnotations": [
    { "width": 100 },
    { "formatter": "timeElapsed", "textColor": "light" },
    { "format": "YYYY-MM-DD", "datePicker": true },
    {
      "formattingConditions": [
        { "gte": 10, "bg": "#fbc9ff" },
        { "gte": 5, "bg": "#d5d2ff" }
      ]
    }
  ]
}
```

### Null slot for a column without annotation

> A column without annotation gets a `null` slot, keeping the array aligned with the columns.

```qd
#issues $title @{width:100} $id
```

```sql
SELECT
  "issues"."title",
  "issues"."id"
FROM "issues";
```

```json
{
  "columnAnnotations": [
    { "width": 100 },
    null
  ]
}
```

### Annotation on a globbed column

> Annotation attached to a column inside a glob is associated with that one expanded column.

```qd
#issues $*(title @{width:100})
```

```sql
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues";
```

```json
{
  "columnAnnotations": [
    null,
    { "width": 100 },
    null,
    null,
    null,
    null,
    null,
    null,
    null
  ]
}
```
