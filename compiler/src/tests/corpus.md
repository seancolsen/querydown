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
created_at:<6m
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
#checkouts check_in_time:@null check_out_time:<@now|minus(1m)
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
#checkouts checkInTime:@null checkOutTime:<@now|minus(1m)
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
++#checkouts{check_in_time:@null patron.first_name:="Foo"}
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
#issues created_at:>@now|minus(6y)
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" > NOW() - make_interval(years => 6);
```

### Duration, uppercase

> Duration units are case-insensitive.

```qd
#issues created_at:>@now|minus(6Y)
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" > NOW() - make_interval(years => 6);
```

## Comparisons

### Date within a past duration of now

> Issues created within the past 6 months. Comparing a date against a duration compiles to an
> absolute-value comparison against now, so it works whether the date is in the past or the future.

```qd
#issues created_at:<6m
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(months => 6));
```

### Date within a future duration of now

> Issues due within 6 months. Note that this uses the same `:<` operator as the past-date example,
> even though the dates lie in the future.

```qd
#issues due_date:<6m
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM make_interval(months => 6));
```

### Date beyond a duration of now

> Issues created more than 6 months from now (in either direction).

```qd
#issues created_at:>6m
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) > EXTRACT(epoch FROM make_interval(months => 6));
```

### Date within a duration of now (DuckDB)

```toml options
dialect = "duckdb"
```

```qd
#issues created_at:<6m
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM to_months(6));
```

### Negated comparison

> Issues whose status is not "open"

```qd
#issues !status:="open"
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
#issues !{status:="open" status:="high"}
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
#issues created_at:>@now|minus(6y)
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
#issues created_at:>@now|minus(1y2d)
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
#issues title:="can't"
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
#issues created_at:(@now|minus(2y))..(@now|minus(1y))
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


## The match operator

### Text match (contains)

The `:` operator does a case-insensitive "contains" match when the left-hand side is a text column.

> Issues whose title contains "performance"

```qd
#issues title:"performance"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('performance' COLLATE "C")) > 0, FALSE);
```

### Text match (contains, bare word on right-hand side)

A bare (unquoted) word on the right-hand side of a comparison is a string literal, so this is
equivalent to quoting `performance`. To refer to a column on the right-hand side instead, the
identifier can be quoted with backticks or written as a multi-part path.

> Issues whose title contains "performance", written without quotes

```qd
#issues title:performance
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('performance' COLLATE "C")) > 0, FALSE);
```

### Text match (contains, DuckDB)

```toml options
dialect = "duckdb"
```

> Issues whose title contains "performance", targeting DuckDB

```qd
#issues title:"performance"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  COALESCE(contains(lower(strip_accents("issues"."title")), lower(strip_accents('performance'))), FALSE);
```

### Text match against an empty string is exact equality

An empty string literal on the right-hand side of `:` is matched via strict equality rather than
"contains" (which would match every row). This makes `description:""` find rows with an empty
description, the same as the explicit `description:=""`.

> Issues with an empty description

```qd
#issues description:""
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."description" = '';
```

### Explicit equality on text

The `:=` operator forces exact equality, even for text columns.

> Issues whose title is exactly "performance"

```qd
#issues title:="performance"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."title" = 'performance';
```

### Match falls back to equality for non-text values

When the left-hand side is not text, `:` behaves as exact equality.

> Issues with id 50

```qd
#issues id:50
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."id" = 50;
```

### Match against null is a null check

A null comparison is an `IS NULL` check regardless of the column type, so text columns are not matched via "contains" here.

> Issues with no title

```qd
#issues title:@null
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."title" IS NULL;
```

## Default text search

### Basic default text search

A bare word that carries no comparison operator is a default text search term. It is matched (via
case-insensitive "contains") against every text-like column of the base table — here `title`,
`description`, and `status`. Multiple terms are combined with `AND`, so each term must be found in at
least one of those columns.

> Issues mentioning both "accessibility" and "feature"

```qd
#issues accessibility feature
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE)) AND
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('feature' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('feature' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('feature' COLLATE "C")) > 0, FALSE));
```

### Bare word searches even when it names a column

A bare word always searches; it does not defer to a column of the same name. Even though `title` is a
column, `#issues title` searches for the literal text "title". To reference the column itself as a
bare condition, quote it with backticks (`` `title` ``).

> Issues mentioning the word "title"

```qd
#issues title
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('title' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('title' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('title' COLLATE "C")) > 0, FALSE));
```

### Default text search with a quoted string

Quoting the search term lets it contain characters (like `:`) that a bare word may not.

> Issues mentioning "localhost:3000"

```qd
#issues "localhost:3000"
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('localhost:3000' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('localhost:3000' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('localhost:3000' COLLATE "C")) > 0, FALSE));
```

### Default text search with the comma "OR" shorthand

Each operand of the comma `,` shorthand is a boolean condition, so a bare operand is its own default
text search. This finds issues matching either term.

> Issues mentioning "foo" or "bar"

```qd
#issues foo,bar
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE));
```

### Default text search alongside other conditions

A default text search term can be freely mixed with ordinary comparisons.

> Open issues mentioning "accessibility"

```qd
#issues accessibility status:="open" $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE)) AND
  "issues"."status" = 'open';
```

### Default text search (DuckDB)

```toml options
dialect = "duckdb"
```

> Issues mentioning "accessibility", targeting DuckDB

```qd
#issues accessibility
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  (COALESCE(contains(lower(strip_accents("issues"."title")), lower(strip_accents('accessibility'))), FALSE) OR COALESCE(contains(lower(strip_accents("issues"."description")), lower(strip_accents('accessibility'))), FALSE) OR COALESCE(contains(lower(strip_accents("issues"."status")), lower(strip_accents('accessibility'))), FALSE));
```

### Configuring the default text search

A `__querydown_default_text_search` custom comparison on the base table overrides which columns the
default text search examines. Here `status` is dropped (it would otherwise be included) and a search
across all comment bodies is added.

> Issues mentioning "accessibility" in their title, description, or any comment

```qd
#issues.__querydown_default_text_search:@x = [
  title:@x
  description:@x
  ++#comments{body:@x}
]
#issues accessibility
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues".*
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('accessibility' COLLATE "C")) > 0, FALSE) OR "cte0"."pk" IS NOT NULL);
```

## Case expressions

### Basic case expression

> Categorize each issue by its status.

```qd
#issues
$title
$ ? status:="open"   ~ "Open"
    status:="closed" ~ "Closed"
    ~~                 "Other"
->category
```

```sql
SELECT
  "issues"."title",
  CASE
    WHEN "issues"."status" = 'open' THEN 'Open'
    WHEN "issues"."status" = 'closed' THEN 'Closed'
    ELSE 'Other'
  END AS "category"
FROM "issues";
```

### Case expression with comparison conditions

> Bucket each issue by its id.

```qd
#issues $ ? id:<10 ~ "low" id:<100 ~ "medium" ~~ "high" ->bucket
```

```sql
SELECT
  CASE
    WHEN "issues"."id" < 10 THEN 'low'
    WHEN "issues"."id" < 100 THEN 'medium'
    ELSE 'high'
  END AS "bucket"
FROM "issues";
```

### Case expression with computed values

> The conditions and the values can each be any expression, including computed ones.

```qd
#issues $id $ ? id:<10 ~ id * 2 ~~ id ->v
```

```sql
SELECT
  "issues"."id",
  CASE
    WHEN "issues"."id" < 10 THEN "issues"."id" * 2
    ELSE "issues"."id"
  END AS "v"
FROM "issues";
```

### Case expression (DuckDB)

```toml options
dialect = "duckdb"
```

> The `CASE` syntax is identical for Postgres and DuckDB.

```qd
#issues
$title
$ ? status:="open"   ~ "Open"
    status:="closed" ~ "Closed"
    ~~                 "Other"
->category
```

```sql
SELECT
  "issues"."title",
  CASE
    WHEN "issues"."status" = 'open' THEN 'Open'
    WHEN "issues"."status" = 'closed' THEN 'Closed'
    ELSE 'Other'
  END AS "category"
FROM "issues";
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
#issues status:="open",created_at:>@2023-03-04
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
#issues project.name:="foo" $id->id
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

### Count of related records is coalesced to zero

> Issues, showing the number of comments on each. An issue with no comments shows `0`, not NULL, because the count is coalesced to zero.

```qd
#issues $title $#comments
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
  COALESCE("cte0"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

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
  COALESCE("cte0"."v1", 0) AS "total_comments_by_author"
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
#users --#issues{created_at:>@now|minus(1y)}
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
#users $#issues{created_at:>@now|minus(1y)}
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
  COALESCE("cte0"."v1", 0)
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Filtered path through inferred intermediate

> Issues that are not labeled bug

```qd
#issues --#labels{name:="bug"} $id
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

## Grouping and aggregation

### Basic grouping

> For each issue status, show the number of issues and the date of the most recently created issue

```qd
#issues $status \g $%count $created_at%max
```

```sql
SELECT
  "issues"."status",
  count(*),
  max("issues"."created_at")
FROM "issues"
GROUP BY "issues"."status";
```

### Grouping by multiple columns

> Grouping ordinals (`\g1`, `\g2`) control the order of the `GROUP BY` columns, independent of the
> order in which the columns appear in the result.

```qd
#issues $status \g2 $author \g1 $%count
```

```sql
SELECT
  "issues"."status",
  "issues"."author",
  count(*)
FROM "issues"
GROUP BY "issues"."author", "issues"."status";
```

### Grouping with an aliased aggregate

> For each status, count the issues and show the earliest and latest creation dates.

```qd
#issues
$status \g
$%count -> total
$created_at%min -> earliest
$created_at%max -> latest
```

```sql
SELECT
  "issues"."status",
  count(*) AS "total",
  min("issues"."created_at") AS "earliest",
  max("issues"."created_at") AS "latest"
FROM "issues"
GROUP BY "issues"."status";
```

### Grouping by a column on a related table

> For each author, count their issues.

```qd
#issues $author.username \g $%count
```

```sql
SELECT
  "users"."username",
  count(*)
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
GROUP BY "users"."username";
```

### Grouping combined with sorting

> For each status, count the issues, showing the most populous statuses first.

```qd
#issues $status \g $%count \sd
```

```sql
SELECT
  "issues"."status",
  count(*)
FROM "issues"
GROUP BY "issues"."status"
ORDER BY
  count(*) DESC NULLS LAST;
```

### Aggregating with sum and avg over base columns

> For each status, show the sum and average of the issue ids.

```qd
#issues $status \g $id%sum $id%avg
```

```sql
SELECT
  "issues"."status",
  sum("issues"."id"),
  avg("issues"."id")
FROM "issues"
GROUP BY "issues"."status";
```

## Function library

### Boolean `and`

> For each issue, whether it is both open and overdue

```qd
#issues $title $(status:="open")|and(due_date:<@now) -> open_and_overdue
```

```sql
SELECT
  "issues"."title",
  "issues"."status" = 'open' AND
  "issues"."due_date" < NOW() AS "open_and_overdue"
FROM "issues";
```

### Boolean `or`

> For each issue, whether it is open or reopened

```qd
#issues $title $(status:="open")|or(status:="reopened") -> needs_attention
```

```sql
SELECT
  "issues"."title",
  "issues"."status" = 'open' OR "issues"."status" = 'reopened' AS "needs_attention"
FROM "issues";
```

### Boolean `xor`

> For each project, whether it is active or empty, but not both

```qd
#projects $name $is_active|xor(#issues:0) -> active_or_empty_but_not_both
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "projects"."is_active" <> ("cte0"."pk" IS NULL) AS "active_or_empty_but_not_both"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Text `concat`

> A label combining each issue's status and title

```qd
#issues $status|concat(": ")|concat(title) -> label
```

```sql
SELECT
  concat(concat("issues"."status", ': '), "issues"."title") AS "label"
FROM "issues";
```

### Text `trim`

> Issue titles with surrounding whitespace removed

```qd
#issues $title|trim
```

```sql
SELECT
  trim("issues"."title")
FROM "issues";
```

### Text `md5`

> The MD5 hash of each user's email

```qd
#users $email|md5 -> email_hash
```

```sql
SELECT
  md5("users"."email") AS "email_hash"
FROM "users";
```

### Aggregate `product`

> The product of the issue ids in each project

```qd
#projects $name $#issues.id%product
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      CASE WHEN bool_or("issues"."id" = 0) THEN 0 ELSE (CASE WHEN count(*) FILTER (WHERE "issues"."id" < 0) % 2 = 1 THEN -1 ELSE 1 END) * round(exp(sum(ln(abs("issues"."id")::double precision)) FILTER (WHERE "issues"."id" <> 0))) END AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Aggregate `product` (DuckDB)

```toml options
dialect = "duckdb"
```

> The product of the issue ids in each project, targeting DuckDB

```qd
#projects $name $#issues.id%product
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      product("issues"."id") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
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
  "users"."team",
  "users"."birth_date"
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
  COALESCE("cte0"."v1", 0)
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

## Computed columns

### Boolean computed column referencing another computed column

> Show all user columns, plus whether each user is old enough to purchase alcohol. `age` and
> `can_purchase_alcohol` are computed columns defined before the query; `can_purchase_alcohol`
> references the earlier `age` definition.

```qd
#users.age = birth_date|age|years|floor
#users.can_purchase_alcohol = age:>=21
#users $* $can_purchase_alcohol
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) >= 21
FROM "users";
```

### Computed column used within a condition

> Find users who are old enough to purchase alcohol, showing their usernames.

```qd
#users.age = birth_date|age|years|floor
#users age:>=21 $username
```

```sql
SELECT
  "users"."username"
FROM "users"
WHERE
  FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) >= 21;
```

### Computed column on a related table

> Show each issue's title alongside whether its author is old enough to purchase alcohol.

```qd
#users.age = birth_date|age|years|floor
#users.can_purchase_alcohol = age:>=21
#issues $title $author.can_purchase_alcohol
```

```sql
SELECT
  "issues"."title",
  FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) >= 21
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id";
```

## User-defined constants

### Constant inlined into a condition

> Show the issues created by user 1234. The `@user_id` constant is defined before the base table and
> its value is inlined into the generated SQL.

```qd
@user_id = 1234
#issues author:@user_id $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
WHERE
  "issues"."author" = 1234;
```

### Constant inlined into an arithmetic expression

> Show each issue's id offset by a constant amount.

```qd
@offset = 100
#issues $id + @offset
```

```sql
SELECT
  "issues"."id" + 100
FROM "issues";
```

### Constant referencing another constant

> A constant's value may itself reference an earlier constant; both are inlined.

```qd
@base = 1000
@user_id = @base
#issues author:@user_id $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
WHERE
  "issues"."author" = 1000;
```

### Constant defined from a subquery

> A constant may be defined as a `#( ... )` subquery that produces a single value. The subquery is
> inlined as a parenthesized scalar subquery wherever the constant is referenced.

```qd
@date_of_latest_comment = #( #comments $created_at%max )
#issues created_at:>@date_of_latest_comment $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."created_at" > (
    SELECT
      max("comments"."created_at")
    FROM "comments"
  );
```

## User-defined functions

### Simple scalar function

> Apply a user-defined `double` function to each issue's id. The function's body is inlined into the
> generated SQL with its parameter bound to the piped-in argument.

```qd
@@double = @x => @x * 2
#issues $id|double
```

```sql
SELECT
  "issues"."id" * 2
FROM "issues";
```

### Function with multiple parameters

> A function may take more than one parameter. When applied via a pipe, the piped-in value is the
> first argument and any parenthesized values supply the rest.

```qd
@@add = @a @b => @a + @b
#issues $id|add(100)
```

```sql
SELECT
  "issues"."id" + 100
FROM "issues";
```

### Function containing an assignment

> A function body may contain local assignments before its result expression. Here `is_adult`
> computes an `age` assignment from its parameter and then compares it.

```qd
@@is_adult = @birth_date =>
  @age = @birth_date|age|years|floor
  @age:>=21
#users $username $birth_date|is_adult
```

```sql
SELECT
  "users"."username",
  FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) >= 21
FROM "users";
```

### Function used within a condition

> A user-defined function may be applied anywhere an expression is allowed, including conditions.

```qd
@@is_adult = @birth_date =>
  @age = @birth_date|age|years|floor
  @age:>=21
#users birth_date|is_adult $username
```

```sql
SELECT
  "users"."username"
FROM "users"
WHERE
  FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) >= 21;
```

## Anonymous functions

### Basic anonymous function

> Categorize each issue by how soon it is due. The piped-in value is bound to the anonymous
> function's `@d` parameter, and the body is inlined wherever `@d` is referenced.

```qd
#issues
$title
$due_date|countdown|days|(@d => ? @d:<0 ~ "overdue" @d:<30 ~ "due soon" ~~ "due later")
```

```sql
SELECT
  "issues"."title",
  CASE
    WHEN EXTRACT(epoch FROM "issues"."due_date" - NOW()) / 86400 < 0 THEN 'overdue'
    WHEN EXTRACT(epoch FROM "issues"."due_date" - NOW()) / 86400 < 30 THEN 'due soon'
    ELSE 'due later'
  END
FROM "issues";
```

### Anonymous function with a simple body

> An anonymous function's body can be any expression. Here the piped-in `id` is doubled.

```qd
#issues $id|(@x => @x * 2)
```

```sql
SELECT
  "issues"."id" * 2
FROM "issues";
```

### Anonymous function used within a condition

> An anonymous function may be applied anywhere an expression is allowed, including conditions.

```qd
#issues id|(@x => @x * 10):>0 $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
WHERE
  "issues"."id" * 10 > 0;
```

### Anonymous function with a local assignment and extra argument

> Like a user-defined function, an anonymous function may take more than one parameter (the piped-in
> value plus parenthesized arguments) and may contain local assignments before its result.

```qd
#issues $id|(@a @b => @s = @a + @b @s * 2)(100)
```

```sql
SELECT
  ("issues"."id" + 100) * 2
FROM "issues";
```

## Function calling

### Basic function call

> Use `@@` to call a function without a pipe. For each issue, show how many days it is overdue,
> displaying zero instead of negative numbers. Arguments are separated by spaces, not commas.

```qd
#issues $title $@@max(due_date|age|days 0)
```

```sql
SELECT
  "issues"."title",
  GREATEST(EXTRACT(epoch FROM NOW() - "issues"."due_date") / 86400, 0)
FROM "issues";
```

### Function call equivalent to a pipe

> The direct call above is equivalent to applying the same function via a pipe.

```qd
#issues $title $due_date|age|days|max(0)
```

```sql
SELECT
  "issues"."title",
  GREATEST(EXTRACT(epoch FROM NOW() - "issues"."due_date") / 86400, 0)
FROM "issues";
```

### Function call on a user-defined function

> The `@@` call syntax works with user-defined functions too.

```qd
@@add = @a @b => @a + @b
#issues $@@add(id 100)
```

```sql
SELECT
  "issues"."id" + 100
FROM "issues";
```

### Function call used within a condition

> A direct call may appear anywhere an expression is allowed, including conditions.

```qd
#issues @@max(id 0):>5 $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
WHERE
  GREATEST("issues"."id", 0) > 5;
```

## Custom comparisons

### Basic custom comparison

> Find issues that have a comment containing the word "workaround". The `comment` custom comparison
> is defined before the base table; using it expands its body in place.

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues comment:workaround $title
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('workaround' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Custom comparison called with a switched operator (regex)

> When a custom comparison is defined with `:` and every comparison in its body also uses `:`, it
> may be called with a different operator, which is substituted throughout the body. Here the regex
> match operator is used.

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues comment:~"work[ -]?around" $title
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      "comments"."body" ~* 'work[ -]?around'
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Custom comparison called with a switched operator (exact equality)

> The match operator can likewise be switched to exact equality.

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues comment:="+1" $title
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      "comments"."body" = '+1'
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Custom comparison with a multi-part body

> A custom comparison's body can be any expression. Here `participant` checks whether a given
> username has commented on, been assigned to, or authored an issue. Because the body contains
> comparisons other than `:`, it must always be called exactly as defined (with `:`).

```qd
#issues.participant:@x = [
  ++#comments{user.username:=@x}
  ++#assignments{user.username:=@x}
  author.username:=@x
]
#issues participant:david $title
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    WHERE
      "users"."username" = 'david'
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "assignments"."issue" AS "pk"
    FROM "assignments"
    LEFT JOIN "users" ON
      "assignments"."user" = "users"."id"
    WHERE
      "users"."username" = 'david'
    GROUP BY "assignments"."issue"
  )
SELECT
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  (
    "cte0"."pk" IS NOT NULL
    OR "cte1"."pk" IS NOT NULL
    OR "users"."username" = 'david'
  );
```

## Query pipelines

A query may be split into multiple stages separated by `~~~`. Each stage operates on the result of the previous stage, which is materialized as a CTE. The columns of a stage's output become the columns available to the next stage.

### Filtering the result of an aggregation

> For each project, count its issues, then keep only the projects with at least two issues.

```qd
#issues $project \g $%count -> issue_count
~~~
issue_count:>=2
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."project" AS "project",
      count(*) AS "issue_count"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "pipe0".*
FROM "pipe0"
WHERE
  "pipe0"."issue_count" >= 2;
```

### Aggregating across stages

> Count each author's issues per project, then sum those counts per author.

```qd
#issues $author \g $project \g $%count -> issue_count
~~~
$author \g $issue_count%sum -> total
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."author" AS "author",
      "issues"."project" AS "project",
      count(*) AS "issue_count"
    FROM "issues"
    GROUP BY "issues"."author", "issues"."project"
  )
SELECT
  "pipe0"."author",
  sum("pipe0"."issue_count") AS "total"
FROM "pipe0"
GROUP BY "pipe0"."author";
```

### Explicit result columns flowing between stages

> Select a few issue columns, then filter and project a subset in the next stage.

```qd
#issues $id $title $status
~~~
status:="open" $id $title
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title",
      "issues"."status" AS "status"
    FROM "issues"
  )
SELECT
  "pipe0"."id",
  "pipe0"."title"
FROM "pipe0"
WHERE
  "pipe0"."status" = 'open';
```

### Three stages

> Three chained stages, each consuming the previous stage's output.

```qd
#issues $project \g $%count -> issue_count
~~~
issue_count:>=2 $project $issue_count
~~~
issue_count:<100
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."project" AS "project",
      count(*) AS "issue_count"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "pipe1" AS (
    SELECT
      "pipe0"."project" AS "project",
      "pipe0"."issue_count" AS "issue_count"
    FROM "pipe0"
    WHERE
      "pipe0"."issue_count" >= 2
  )
SELECT
  "pipe1".*
FROM "pipe1"
WHERE
  "pipe1"."issue_count" < 100;
```

### Nested CTE within a pipeline stage

> A pipeline stage that itself aggregates a related table compiles its aggregation CTE nested within the pipeline CTE.

```qd
#issues $id $#comments -> comment_count
~~~
comment_count:>=10 $id
```

```sql
WITH
  "pipe0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk",
          count(*) AS "v1"
        FROM "comments"
        GROUP BY "comments"."issue"
      )
    SELECT
      "issues"."id" AS "id",
      COALESCE("cte0"."v1", 0) AS "comment_count"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  "pipe0"."comment_count" >= 10;
```

## User-defined tables

A user-defined table, written as `#name = #( query )` before the base table, names a subquery that compiles to a CTE. The name can then be used as the base table of the query, or of a later user-defined table.

### User-defined table as a base table

> For each project, count its issues in a user-defined table, then keep the projects with at least ten issues and count how many such projects each grouping has.

```qd
#project_counts = #(
  #issues $project \g $%count -> issue_count
)
#project_counts issue_count:>=10 $project \g $%count
```

```sql
WITH
  "project_counts" AS (
    SELECT
      "issues"."project" AS "project",
      count(*) AS "issue_count"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "project_counts"."project",
  count(*)
FROM "project_counts"
WHERE
  "project_counts"."issue_count" >= 10
GROUP BY "project_counts"."project";
```

### User-defined table referencing an earlier one

> A user-defined table may use a previously-defined user-defined table as its base table. Each compiles to its own CTE, in definition order.

```qd
#a = #(
  #issues $project \g $%count -> n
)
#b = #(
  #a n:>=5 $project
)
#b $project
```

```sql
WITH
  "a" AS (
    SELECT
      "issues"."project" AS "project",
      count(*) AS "n"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "b" AS (
    SELECT
      "a"."project" AS "project"
    FROM "a"
    WHERE
      "a"."n" >= 5
  )
SELECT
  "b"."project"
FROM "b";
```

## Window functions

### Row number

> Number each issue within its project, ordered by creation date.

```qd
#issues $id $%%(project\p created_at\s)%row_number -> rn
```

```sql
SELECT
  "issues"."id",
  row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST) AS "rn"
FROM "issues";
```

### Running aggregate over a window

> A running total of issue ids within each project, in creation-date order.

```qd
#issues $id $%%(project\p created_at\s)%sum(id) -> running
```

```sql
SELECT
  "issues"."id",
  sum("issues"."id") OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST) AS "running"
FROM "issues";
```

### Window function with value arguments

> The previous issue's id within each project (offset 1, default 0), most recent first.

```qd
#issues $id $%%(project\p created_at\sd)%lag(id 1 0) -> prev
```

```sql
SELECT
  "issues"."id",
  lag("issues"."id", 1, 0) OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "prev"
FROM "issues";
```

### Count over a partition

> How many issues share each project (a partition with no ordering). `count` may be used on its own to mean `count(*)`.

```qd
#issues $id $%%(project\p)%count -> n_in_project
```

```sql
SELECT
  "issues"."id",
  count(*) OVER (PARTITION BY "issues"."project") AS "n_in_project"
FROM "issues";
```

### Ntile bucketing

> Split issues into four buckets per status, ordered by id.

```qd
#issues $id $%%(status\p id\s)%ntile(4) -> quartile
```

```sql
SELECT
  "issues"."id",
  ntile(4) OVER (PARTITION BY "issues"."status" ORDER BY "issues"."id" ASC NULLS LAST) AS "quartile"
FROM "issues";
```

### Rank (DuckDB)

```toml options
dialect = "duckdb"
```

> Window functions compile identically on DuckDB.

```qd
#issues $id $%%(status\p id\s)%rank -> r
```

```sql
SELECT
  "issues"."id",
  rank() OVER (PARTITION BY "issues"."status" ORDER BY "issues"."id" ASC NULLS LAST) AS "r"
FROM "issues";
```

### Filtering on a window function

> The most recent issue in each project. Because SQL forbids window functions in `WHERE`, the window value is computed in a wrapping subquery and filtered in the outer query.

```qd
#issues %%(project\p created_at\sd)%row_number:1 $id $title $project
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title",
      "issues"."project" AS "project",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
  )
SELECT
  "qdwin_src"."id" AS "id",
  "qdwin_src"."title" AS "title",
  "qdwin_src"."project" AS "project"
FROM "qdwin_src"
WHERE
  "qdwin_src"."qdwin0" = 1;
```

### Filtering on a window function alongside an ordinary filter

> Non-window conditions stay in the inner query; only the window predicate is applied in the outer query.

```qd
#issues status:="open" %%(project\p created_at\sd)%row_number:1 $id $project
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."project" AS "project",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
    WHERE
      "issues"."status" = 'open'
  )
SELECT
  "qdwin_src"."id" AS "id",
  "qdwin_src"."project" AS "project"
FROM "qdwin_src"
WHERE
  "qdwin_src"."qdwin0" = 1;
```

### Window column flowing between pipeline stages

> Compute a per-group sequence number in one stage, then filter it in the next stage as an ordinary column.

```qd
#comments $issue $user $%%(issue\p user\p created_at\s)%row_number -> seq
~~~
seq:1 $issue $user
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "comments"."issue" AS "issue",
      "comments"."user" AS "user",
      row_number() OVER (PARTITION BY "comments"."issue", "comments"."user" ORDER BY "comments"."created_at" ASC NULLS LAST) AS "seq"
    FROM "comments"
  )
SELECT
  "pipe0"."issue",
  "pipe0"."user"
FROM "pipe0"
WHERE
  "pipe0"."seq" = 1;
```
