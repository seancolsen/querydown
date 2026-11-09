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
SELECT
  "Patrons"."id",
  "Patrons"."First Name",
  "Patrons"."Last Name",
  "Patrons"."Email"
FROM "Patrons";
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
  "Checkouts"."id",
  "Checkouts"."Item",
  "Checkouts"."Patron",
  "Checkouts"."Checkout Time",
  "Checkouts"."Due Date",
  "Checkouts"."Check In Time"
FROM "Checkouts";
```

### snake_case

> Checkouts from over one month ago and not yet returned

```qd
#checkouts check_in_time:@null check_out_time:<@now|minus(1m)
```

```sql
SELECT
  "Checkouts"."id",
  "Checkouts"."Item",
  "Checkouts"."Patron",
  "Checkouts"."Checkout Time",
  "Checkouts"."Due Date",
  "Checkouts"."Check In Time"
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
  "Checkouts"."id",
  "Checkouts"."Item",
  "Checkouts"."Patron",
  "Checkouts"."Checkout Time",
  "Checkouts"."Due Date",
  "Checkouts"."Check In Time"
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
      lower("Patrons"."First Name" COLLATE "C") = lower('Foo' COLLATE "C")
    GROUP BY "Checkouts"."Item"
  )
SELECT
  "Items"."id",
  "Items"."Barcode",
  "Items"."Acquisition Date",
  "Items"."Acquisition Price",
  "Items"."Book"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at")) < EXTRACT(epoch FROM to_months(6));
```

### Aggregated date within a duration of now

> Issues whose most recent comment was made within the past week. Type inference recognizes that a
> datetime column run through `max` is still a datetime, so the date/duration comparison magic
> applies to the aggregated value just as it would to a plain column.

```qd
#issues #comments.created_at%max:<1w
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
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "cte0"."v1")) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Comparing a zoned timestamp to now

> Users seen before now. `last_seen` is a `timestamptz`, the same zoned type as `@now`, so no
> reconciliation is needed and the comparison compiles identically on every dialect.

```qd
#users last_seen:<@now
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
FROM "users"
WHERE
  "users"."last_seen" < NOW();
```

### Comparing a zoned timestamp to now (DuckDB)

> The same query under DuckDB, which (without the ICU extension) can't operate on zoned timestamps
> at all. It works in naive timestamp space: `@now` is coerced to naive at its source, and the
> zoned `last_seen` column is coerced to match.

```toml options
dialect = "duckdb"
```

```qd
#users last_seen:<@now
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
FROM "users"
WHERE
  CAST("users"."last_seen" AS TIMESTAMP) < CAST(NOW() AS TIMESTAMP);
```

### Mixing a zoned and a naive timestamp

> Users last seen after a given date. `last_seen` is zoned (`timestamptz`) while the date literal is
> naive, but Postgres reconciles the two natively, so no cast appears.

```qd
#users last_seen:>@2023-03-04
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
FROM "users"
WHERE
  "users"."last_seen" > DATE '2023-03-04';
```

### Mixing a zoned and a naive timestamp (DuckDB)

> The same query under DuckDB, which (without the ICU extension) can't compare a zoned `timestamptz`
> against a naive value. The zoned operand is cast down to a naive `timestamp` so the comparison is
> valid.

```toml options
dialect = "duckdb"
```

```qd
#users last_seen:>@2023-03-04
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
FROM "users"
WHERE
  CAST("users"."last_seen" AS TIMESTAMP) > DATE '2023-03-04';
```

### Negated comparison

> Issues whose status is not "open"

```qd
#issues !status:="open"
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
FROM "issues"
WHERE
  NOT lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C");
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  NOT (lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") AND
  lower("issues"."status" COLLATE "C") = lower('high' COLLATE "C"));
```

### Regex

> Issues with titles containing "foo"

```qd
#issues title:~"foo"
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
FROM "issues"
WHERE
  "issues"."title" ~* 'foo';
```

### Expansion

The expansion set is on the right-hand side of the comparison, so its bare words `color` and
`colour` are string literals — the same as if each were quoted.

```qd
#issues title:~[color colour]
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
FROM "issues"
WHERE
  ("issues"."title" ~* 'color' OR "issues"."title" ~* 'colour');
```

### Dual expansion

The left-hand expansion `{title description}` references columns (it is on the left), while the
right-hand expansion `[color colour]` holds string literals (it is on the right).

```qd
#issues {title description}:~[color colour]
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
FROM "issues"
WHERE
  ("issues"."title" ~* 'color' OR "issues"."title" ~* 'colour') AND
  ("issues"."description" ~* 'color' OR "issues"."description" ~* 'colour');
```

### Expansion of a boolean comparison

A condition set on the left of a comparison expands automatically, distributing the comparison across
each entry. This finds projects that are neither active nor pro-bono.

```qd
#projects {is_pro_bono is_active}:@false
```

```sql
SELECT
  "projects"."id",
  "projects"."name",
  "projects"."product",
  "projects"."is_active",
  "projects"."is_pro_bono"
FROM "projects"
WHERE
  "projects"."is_pro_bono" = FALSE AND
  "projects"."is_active" = FALSE;
```

### Boolean logic before comparison

Without expansion, boolean logic can be applied to the operands _before_ they are compared by using
functions instead of a condition set. This produces different results from the expansion above.

```qd
#projects is_pro_bono|and(is_active):@false
```

```sql
SELECT
  "projects"."id",
  "projects"."name",
  "projects"."product",
  "projects"."is_active",
  "projects"."is_pro_bono"
FROM "projects"
WHERE
  ("projects"."is_pro_bono" AND
  "projects"."is_active") = FALSE;
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  "issues"."created_at" > CAST(NOW() AS TIMESTAMP) - to_years(6);
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  "issues"."created_at" > CAST(NOW() AS TIMESTAMP) - (to_years(1) + to_days(2));
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  lower(strip_accents("issues"."title")) = lower(strip_accents('can''t'));
```

### Infinity

> The `@infinity` constant casts the `'infinity'` string literal, since a bare keyword is invalid.

```qd
#issues id:<@infinity
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  "issues"."created_at" >= NOW() - make_interval(years => 2) AND
  "issues"."created_at" <= NOW() - make_interval(years => 1);
```

### Range vs expansion

```qd
#comments [created_at issue.created_at]:@2000-01-01..<@2000-02-01
```

```sql
SELECT
  "comments"."id",
  "comments"."issue",
  "comments"."user",
  "comments"."body",
  "comments"."created_at"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  ("comments"."created_at" >= DATE '2000-01-01' AND
  "comments"."created_at" < DATE '2000-02-01' OR "issues"."created_at" >= DATE '2000-01-01' AND
  "issues"."created_at" < DATE '2000-02-01');
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  COALESCE(contains(lower(strip_accents("issues"."title")), lower(strip_accents('performance'))), FALSE);
```

### Text match against an empty string is exact equality

An empty string literal on the right-hand side of `:` is matched via strict equality rather than
"contains" (which would match every row). This makes `description:""` find rows with an empty
description, the same as the explicit `description:==""`.

> Issues with an empty description

```qd
#issues description:""
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
FROM "issues"
WHERE
  "issues"."description" = '';
```

### Case-insensitive equality on text

The `:=` operator performs a case-insensitive equality comparison on text columns (unlike `:`, which
matches a substring). It applies the same normalization as `:`, so `title:="performance"` matches a
title of "Performance" but not "performance issues".

> Issues whose title equals "performance", ignoring case

```qd
#issues title:="performance"
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
FROM "issues"
WHERE
  lower("issues"."title" COLLATE "C") = lower('performance' COLLATE "C");
```

### Case-sensitive equality on text

The `:==` operator forces exact, case-sensitive equality, even for text columns.

> Issues whose title is exactly "performance"

```qd
#issues title:=="performance"
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
FROM "issues"
WHERE
  "issues"."title" = 'performance';
```

### Case-insensitive equality on text (DuckDB)

```toml options
dialect = "duckdb"
```

On DuckDB, `:=` also ignores accents, mirroring the `:` operator's `lower(strip_accents(...))`
normalization.

> Issues whose title equals "performance", ignoring case and accents

```qd
#issues title:="performance"
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
FROM "issues"
WHERE
  lower(strip_accents("issues"."title")) = lower(strip_accents('performance'));
```

### Match falls back to equality for non-text values

When the left-hand side is not text, `:` behaves as exact equality.

> Issues with id 50

```qd
#issues id:50
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
FROM "issues"
WHERE
  "issues"."id" = 50;
```

### Case-insensitive equality falls back to plain equality for non-text values

The `:=` and `:==` operators both reduce to a plain `=` comparison when the left-hand side is not
text, since case sensitivity is meaningless there.

> Issues with id 50

```qd
#issues id:=50
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE));
```

### Negated default text search

Prefixing a search term with `!` negates it: the whole per-column `OR` is wrapped in `NOT`, so this
finds issues that do _not_ contain the term in any of their text columns.

> Issues not mentioning "backend"

```qd
#issues !backend
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
FROM "issues"
WHERE
  NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('backend' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('backend' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('backend' COLLATE "C")) > 0, FALSE));
```

### Negated default text search in the comma "OR" shorthand

A `!`-prefixed operand of the comma `,` shorthand is a negated search, while a bare sibling operand
remains an ordinary (positive) search.

> Issues mentioning "foo" or not mentioning "bar"

```qd
#issues foo,!bar
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
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)));
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
  lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C");
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
    WHEN lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") THEN 'Open'
    WHEN lower("issues"."status" COLLATE "C") = lower('closed' COLLATE "C") THEN 'Closed'
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

> The `CASE` syntax is identical for Postgres and DuckDB, though the `:=` conditions normalize text
> per dialect.

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
    WHEN lower(strip_accents("issues"."status")) = lower(strip_accents('open')) THEN 'Open'
    WHEN lower(strip_accents("issues"."status")) = lower(strip_accents('closed')) THEN 'Closed'
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
WHERE
  (lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") OR "issues"."created_at" > DATE '2023-03-04');
```

## Scoped comparisons

A path written immediately before a condition set — with **no space** between them — scopes every
entry of the set to the related record the path points at. So `issue{title:dashboard}` is the same as
`issue.title:dashboard`. Because the scope is resolved against the related table, anything you could
write as a top-level condition on that table works inside the braces — including a bare
[default text search](#default-text-search), which searches the related table's text columns (see
["Scoped default text search"](#scoped-default-text-search) below).

### Basic scoped comparison

> Comments whose issue's title contains "dashboard". Equivalent to `#comments issue.title:dashboard`.

```qd
#comments issue{title:dashboard}
```

```sql
SELECT
  "comments"."id",
  "comments"."issue",
  "comments"."user",
  "comments"."body",
  "comments"."created_at"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('dashboard' COLLATE "C")) > 0, FALSE);
```

### Scoped comparison with several entries

> The `{ }` scope composes its entries with `AND`, applying `issue.` to each. Equivalent to
> `#comments issue.title:dashboard issue.status:=open`.

```qd
#comments issue{title:dashboard status:=open}
```

```sql
SELECT
  "comments"."id",
  "comments"."issue",
  "comments"."user",
  "comments"."body",
  "comments"."created_at"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('dashboard' COLLATE "C")) > 0, FALSE) AND
  lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C");
```

### Scoped comparison with "OR"

> An `[ ]` scope composes its entries with `OR`. Equivalent to
> `#comments [issue.title:dashboard issue.description:dashboard]`.

```qd
#comments issue[title:dashboard description:dashboard]
```

```sql
SELECT
  "comments"."id",
  "comments"."issue",
  "comments"."user",
  "comments"."body",
  "comments"."created_at"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('dashboard' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('dashboard' COLLATE "C")) > 0, FALSE));
```

### Scoped default text search

A bare word inside the scope is a [default text search](#default-text-search) — but against the
_related_ table. There is no way to write this by prefixing a path (a bare search term has no column
to prefix), which is why scoping is resolved in the compiler rather than desugared. Here the search
runs across every text column of `issues`.

> Comments whose issue matches "dashboard" in any of its text columns

```qd
#comments issue{dashboard}
```

```sql
SELECT
  "comments"."id",
  "comments"."issue",
  "comments"."user",
  "comments"."body",
  "comments"."created_at"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('dashboard' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('dashboard' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('dashboard' COLLATE "C")) > 0, FALSE));
```

### Scoped "has" condition

A ["has" condition](#has-conditions) inside the scope looks for records related to the _scoped_
record, not to the base record. Equivalent to `#issues ++author.#comments{id:5}`.

> Issues whose author wrote comment 5

```qd
#issues author{++#comments{id:5}} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk"
    FROM "comments"
    WHERE
      "comments"."id" = 5
    GROUP BY "comments"."user"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Scoped "has none" condition

> Issues whose author has written no comments. Equivalent to `#issues --author.#comments`.

```qd
#issues author{--#comments} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk"
    FROM "comments"
    GROUP BY "comments"."user"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### Scoped "has" condition through several hops

> Comments on any issue in the same project as issue 42 (not just comments on issue 42). Equivalent
> to `#comments ++issue.project.#issues{id:42}`.

```qd
#comments issue.project{++#issues{id:42}} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    WHERE
      "issues"."id" = 42
    GROUP BY "issues"."project"
  )
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
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
  lower("projects"."name" COLLATE "C") = lower('foo' COLLATE "C");
```

### Comparing an FK column to NULL

```qd
#issues author:@null
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
FROM "issues"
WHERE
  "issues"."author" IS NULL;
```

### Self-referential foreign key

> `issues.duplicate_of` links `issues` to itself. The joined instance must get an alias distinct from
> the base table, or the generated SQL references `issues` twice with nothing to disambiguate them.

```qd
#issues duplicate_of.title:="foo" $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id"
WHERE
  lower("issues_1"."title" COLLATE "C") = lower('foo' COLLATE "C");
```

### Self-referential foreign key, chained through another link

> A path may continue past the self-join. Here `duplicate_of.author` reaches `users` through the
> aliased `issues_1`.

```qd
#issues duplicate_of.author.username:="alice" $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id"
LEFT JOIN "users" ON
  "issues_1"."author" = "users"."id"
WHERE
  lower("users"."username" COLLATE "C") = lower('alice' COLLATE "C");
```

### Self-referential foreign key, followed twice

> A to-one path may revisit a table more than once. Each instance of `issues` gets its own alias.

```qd
#issues $id->id $duplicate_of.duplicate_of.title->t
```

```sql
SELECT
  "issues"."id" AS "id",
  "issues_1"."title" AS "t"
FROM "issues"
LEFT JOIN "issues" AS "issues_2" ON
  "issues"."duplicate_of" = "issues_2"."id"
LEFT JOIN "issues" AS "issues_1" ON
  "issues_2"."duplicate_of" = "issues_1"."id";
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

### Path through many, many, revisiting the base table

> Issues, showing the date of the most recently created issue that shares a label with the issue.
> The path leaves `issues` and comes back to it, so the second visit to `issue_labels` gets its own
> alias within the CTE.

```qd
#issues $id->id $#labels.#issues.created_at%max->v
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      max("issues"."created_at") AS "v1"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    JOIN "issue_labels" AS "issue_labels_1" ON
      "labels"."id" = "issue_labels_1"."label"
    JOIN "issues" ON
      "issue_labels_1"."issue" = "issues"."id"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id" AS "id",
  "cte0"."v1" AS "v"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Path through many, one, many, revisiting the aggregated table

> Users, listing the titles of open issues in any project the user has created an issue in. The
> second visit to `issues` is aliased `issues_1`, and both the filter and the aggregated value
> refer to that instance.

```qd
#users $id->id $#issues.project.#issues{status:="open"}.title%list->v
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk",
      array_agg(DISTINCT "issues_1"."title") AS "v1"
    FROM "issues"
    JOIN "projects" ON
      "issues"."project" = "projects"."id"
    JOIN "issues" AS "issues_1" ON
      "projects"."id" = "issues_1"."project"
    WHERE
      lower("issues_1"."status" COLLATE "C") = lower('open' COLLATE "C")
    GROUP BY "issues"."author"
  )
SELECT
  "users"."id" AS "id",
  "cte0"."v1" AS "v"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### List aggregate (array_agg)

> Issues with an array of their distinct label names.

```qd
#issues $id $#issue_labels.label.name%list
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "labels"."name") AS "v1"
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

> Issues with an array of their distinct label names sorted alphabetically. A sorting expression
> attached to an aggregate function is evaluated from the context of the first `#` table in the
> aggregated path, which here is `labels` (reached via the elided join through `issue_labels`).

```qd
#issues $id $#labels.name%list(\\name)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "labels"."name" ORDER BY "labels"."name" ASC NULLS LAST) AS "v1"
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

> Issues with an array of their distinct label names sorted reverse-alphabetically.

```qd
#issues $id $#labels.name%list(\\name \d)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "labels"."name" ORDER BY "labels"."name" DESC NULLS LAST) AS "v1"
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

### List aggregate sorted by its own value via column flags

> Issues with an array of their distinct label names sorted reverse-alphabetically. Within an
> aggregate's parentheses, the column flags `\s` (plus optional `d` and `n`) sort by the value being
> aggregated, without needing to spell out its path.

```qd
#issues $id $#labels.name%list(\sd)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "labels"."name" ORDER BY "labels"."name" DESC NULLS LAST) AS "v1"
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

### List aggregate sorted via a path from the join table

> The sorting expression may itself follow a path out of the first `#` table. Here `\\label.name`
> walks from `issue_labels` to its `label`. That resolves to the very column being aggregated (via
> the same route), so it's recognized as a sort by the aggregated value: the existing `labels` join
> is reused and the `DISTINCT` is kept.

```qd
#issues $id $#issue_labels.label.name%list(\\label.name)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "labels"."name" ORDER BY "labels"."name" ASC NULLS LAST) AS "v1"
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

### List aggregate sorted via a to-one path to its value

> Issues with the distinct usernames of their commenters, sorted alphabetically. The sort is
> evaluated from `comments`, so the username is reached via `user.username` — the same route as the
> aggregated value — and `DISTINCT` is kept.

```qd
#issues $id $title $#comments.user.username%list(\\user.username)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg(DISTINCT "users"."username" ORDER BY "users"."username" ASC NULLS LAST) AS "v1"
    FROM "comments"
    JOIN "users" ON
      "comments"."user" = "users"."id"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  "issues"."title",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### List aggregate sorted by an aggregated sort key

> Issues with the distinct usernames of their commenters, in the order of each user's first comment
> on the issue. Because each username may appear on several comments, the sort key is aggregated
> (`%min`) per distinct username. SQL can't express that with a single `DISTINCT` aggregate, so it
> compiles to a two-level aggregation: an inner CTE groups the comments by issue and username,
> computing the sort key, and the outer query collects the usernames ordered by it.

```qd
#issues $id $#comments.user.username%list(\\created_at%min)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "comments"."issue" AS "pk",
          "users"."username" AS "value",
          min("comments"."created_at") AS "sort1"
        FROM "comments"
        JOIN "users" ON
          "comments"."user" = "users"."id"
        GROUP BY "comments"."issue", "users"."username"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### List aggregate sorted by several sort keys

> Issues with the distinct usernames of their commenters, most frequent commenter first, breaking
> ties by the most recent comment and then alphabetically. Aggregated sort keys and sorts by the
> value itself (`\s`) may be mixed freely.

```qd
#issues $id $#comments.user.username%list(\\%count \d \\created_at%max \d \s)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "comments"."issue" AS "pk",
          "users"."username" AS "value",
          count(*) AS "sort1",
          max("comments"."created_at") AS "sort2"
        FROM "comments"
        JOIN "users" ON
          "comments"."user" = "users"."id"
        GROUP BY "comments"."issue", "users"."username"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" DESC NULLS LAST, "distinct_values"."sort2" DESC NULLS LAST, "distinct_values"."value" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### List aggregate sorted by an aggregated sort key over a filtered path

> Conditions on the aggregated path apply within the inner CTE of the two-level aggregation, so
> only the matching records contribute to the sort keys.

```qd
#issues $id $#comments{!user:issue.author}.user.username%list(\\%count \d)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "comments"."issue" AS "pk",
          "users"."username" AS "value",
          count(*) AS "sort1"
        FROM "comments"
        JOIN "users" ON
          "comments"."user" = "users"."id"
        LEFT JOIN "issues" ON
          "comments"."issue" = "issues"."id"
        WHERE
          NOT "comments"."user" = "issues"."author"
        GROUP BY "comments"."issue", "users"."username"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" DESC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Top aggregate by an aggregated sort key

> Issues with the usernames of the users who tie for making the most comments on each issue. Like
> `list` sorted by an aggregated sort key, `top` compiles to a two-level aggregation: an inner CTE
> groups the comments by issue and username, ranking each group within its issue by the sort keys,
> and the outer query collects the usernames ranked first, ordered by value since they all tie.

```qd
#issues $id $#comments.user.username%top(\\%count \d)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "comments"."issue" AS "pk",
          "users"."username" AS "value",
          rank() OVER (PARTITION BY "comments"."issue" ORDER BY count(*) DESC NULLS LAST) AS "rank"
        FROM "comments"
        JOIN "users" ON
          "comments"."user" = "users"."id"
        GROUP BY "comments"."issue", "users"."username"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."value" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    WHERE
      "distinct_values"."rank" = 1
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Top aggregate by several sort keys

> Issues with the usernames of the most frequent commenters, keeping only those among them who
> commented most recently. Values tie only if they tie on every sort key.

```qd
#issues $id $#comments.user.username%top(\\%count \d \\created_at%max \d)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "comments"."issue" AS "pk",
          "users"."username" AS "value",
          rank() OVER (PARTITION BY "comments"."issue" ORDER BY count(*) DESC NULLS LAST, max("comments"."created_at") DESC NULLS LAST) AS "rank"
        FROM "comments"
        JOIN "users" ON
          "comments"."user" = "users"."id"
        GROUP BY "comments"."issue", "users"."username"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."value" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    WHERE
      "distinct_values"."rank" = 1
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Top aggregate by the value itself

> Projects with the alphabetically last issue title. Sorting by the value itself (`\s`) works too,
> though the value is still collected into an array.

```qd
#projects $name $#issues.title%top(\sd)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "issues"."project" AS "pk",
          "issues"."title" AS "value",
          rank() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."title" DESC NULLS LAST) AS "rank"
        FROM "issues"
        GROUP BY "issues"."project", "issues"."title"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."value" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    WHERE
      "distinct_values"."rank" = 1
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Top aggregate over a filtered path

> Conditions on the aggregated path apply within the inner CTE, so only the matching records
> contribute to the ranking.

```qd
#issues $id $#comments{!user:issue.author}.user.username%top(\\%count \d)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "comments"."issue" AS "pk",
          "users"."username" AS "value",
          rank() OVER (PARTITION BY "comments"."issue" ORDER BY count(*) DESC NULLS LAST) AS "rank"
        FROM "comments"
        JOIN "users" ON
          "comments"."user" = "users"."id"
        LEFT JOIN "issues" ON
          "comments"."issue" = "issues"."id"
        WHERE
          NOT "comments"."user" = "issues"."author"
        GROUP BY "comments"."issue", "users"."username"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."value" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    WHERE
      "distinct_values"."rank" = 1
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### List all aggregate sorted by a join-table column

> Issues with an array of their label names, sorted by a column of the join table rather than a
> column of the labels themselves. Because the aggregated path begins with `#issue_labels`, the
> sorting expression `\\id` is evaluated from the context of the `issue_labels` join table. Such a
> per-record sort key is only allowed with `list_all`: `list` removes duplicates, which may disagree
> on the sort key.

```qd
#issues $id $#issue_labels.label.name%list_all(\\id)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg("labels"."name" ORDER BY "issue_labels"."id" ASC NULLS LAST) AS "v1"
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

### Aggregate sorting that joins a table a path filter already joined

> The filter on `#comments` joins `users` (for `user.username`), and the sort joins `users` again
> (for `author.username`), via a different foreign key. The second join gets its own alias.

```qd
#projects $#issues.#comments{user.username:x}.body%list_all(\\author.username)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg("comments"."body" ORDER BY "users_1"."username" ASC NULLS LAST) AS "v1"
    FROM "issues"
    JOIN "comments" ON
      "issues"."id" = "comments"."issue"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    LEFT JOIN "users" AS "users_1" ON
      "issues"."author" = "users_1"."id"
    WHERE
      COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### List aggregate of grouped records sorted by its value

> Within a grouped query, `list` collects a column of each group's records. The `\s` flags sort it
> by that column.

```qd
#issues $status \g $title%list(\s)
```

```sql
SELECT
  "issues"."status",
  array_agg(DISTINCT "issues"."title" ORDER BY "issues"."title" ASC NULLS LAST)
FROM "issues"
GROUP BY "issues"."status";
```

### List all aggregate (non-distinct array_agg)

> Issues with an array of all their label names, including duplicates.

```qd
#issues $id $#issue_labels.label.name%list_all
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

### List all aggregate with ORDER BY

> Issues with an array of all their label names, including duplicates, sorted alphabetically.

```qd
#issues $id $#labels.name%list_all(\\name)
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NOT NULL;
```

## Linking columns

### Linking column for a reverse link

> Issues that are not blocking any other issues. `blocks` refers to `issues` twice, so the
> linking column chooses which foreign key to follow.

```qd
#issues --#blocks(blocker) $id->id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "blocks"."blocker" AS "pk"
    FROM "blocks"
    GROUP BY "blocks"."blocker"
  )
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### The other linking column

> Issues that are not blocked by any other issues

```qd
#issues --#blocks(blocking) $id->id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "blocks"."blocking" AS "pk"
    FROM "blocks"
    GROUP BY "blocks"."blocking"
  )
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### Self-referencing link

> Issues, showing how many other issues are marked as duplicates of each one

```qd
#issues $id->id $#issues(duplicate_of)->duplicates
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."duplicate_of" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."duplicate_of"
  )
SELECT
  "issues"."id" AS "id",
  COALESCE("cte0"."v1", 0) AS "duplicates"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Self-referencing link without a linking column

> `issues.duplicate_of` is the only foreign key from `issues` to itself, so it's the shortest
> unambiguous route from `issues` back to `issues`, even without naming it.

```qd
#issues $id->id $#issues->duplicates
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."duplicate_of" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."duplicate_of"
  )
SELECT
  "issues"."id" AS "id",
  COALESCE("cte0"."v1", 0) AS "duplicates"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Self-referencing link at the end of a longer route

> Issues, listing the titles of issues that are duplicates of issues sharing a label with each
> issue. The route reaches `issues` via `issue_labels.issue`, which isn't the named linking column,
> and then follows `duplicate_of` back to `issues`.

```qd
#issues $id->id $#labels.#issues(duplicate_of).title%list->v
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "issues_1"."title") AS "v1"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    JOIN "issue_labels" AS "issue_labels_1" ON
      "labels"."id" = "issue_labels_1"."label"
    JOIN "issues" ON
      "issue_labels_1"."issue" = "issues"."id"
    JOIN "issues" AS "issues_1" ON
      "issues"."id" = "issues_1"."duplicate_of"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id" AS "id",
  "cte0"."v1" AS "v"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
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
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
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
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
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
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
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
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
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
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen"
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
      lower("labels"."name" COLLATE "C") = lower('bug' COLLATE "C")
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

### A filter that aligns with the join

> Issues, showing the total number of comments made on the issue by the issue's author

```qd
#issues $#comments{user:issue.author}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    WHERE
      "comments"."user" = "issues"."author"
    GROUP BY "comments"."issue"
  )
SELECT
  COALESCE("cte0"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Nested filtered path

> Clients that don't have any issues without comments

```qd
#clients --#issues{--#comments}
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk"
        FROM "comments"
        GROUP BY "comments"."issue"
      )
    SELECT
      "products"."client" AS "pk"
    FROM "products"
    JOIN "projects" ON
      "products"."id" = "projects"."product"
    JOIN "issues" ON
      "projects"."id" = "issues"."project"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    WHERE
      "cte0"."pk" IS NULL
    GROUP BY "products"."client"
  )
SELECT
  "clients"."id",
  "clients"."name"
FROM "clients"
LEFT JOIN "cte0" ON
  "clients"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### Filtered path with circular join traversal

> Issues with labels having fewer than 5 issues

```qd
#issues ++#labels{#issues:<5}
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "issue_labels"."label" AS "pk",
          count(*) AS "v1"
        FROM "issue_labels"
        JOIN "issues" ON
          "issue_labels"."issue" = "issues"."id"
        GROUP BY "issue_labels"."label"
      )
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    LEFT JOIN "cte0" ON
      "labels"."id" = "cte0"."pk"
    WHERE
      COALESCE("cte0"."v1", 0) < 5
    GROUP BY "issue_labels"."issue"
  )
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
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
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

## Scoped sorting expressions

### Basic scoping

> Issues sorted by project status, project name, and issue due date. The `\\project.( ... )` group
> applies `project.` to the start of every sorting expression inside it, so this is the same as
> writing `\\project.is_active`, `\\project.name`, then `\\due_date`.

```qd
#issues
\\project.(
  \\is_active
  \\name
)
\\due_date
$title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
ORDER BY
  "projects"."is_active" ASC NULLS LAST,
  "projects"."name" ASC NULLS LAST,
  "issues"."due_date" ASC NULLS LAST;
```

### Scoping within scoping

> Groups may be nested within groups; the paths compose.

```qd
#issues \\project.( \\product.( \\name ) ) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id"
ORDER BY
  "products"."name" ASC NULLS LAST;
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
  lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") AND
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
  lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") OR lower("issues"."status" COLLATE "C") = lower('reopened' COLLATE "C") AS "needs_attention"
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

### Math `pow`

> Each issue id, squared

```qd
#issues $id|pow(2)
```

```sql
SELECT
  POWER("issues"."id", 2)
FROM "issues";
```

### Math `exp`

> e raised to the power of each issue id

```qd
#issues $id|exp
```

```sql
SELECT
  EXP("issues"."id")
FROM "issues";
```

### Math `sqrt`

> The square root of each issue id

```qd
#issues $id|sqrt
```

```sql
SELECT
  SQRT("issues"."id")
FROM "issues";
```

### Math `unit_hash`

> A deterministic pseudo-random number between 0 and 1, derived from each user's email

```qd
#users $email|unit_hash -> email_bucket
```

```sql
SELECT
  (CAST(hashtext("users"."email") AS DOUBLE PRECISION) + 2147483648) / 4294967295.0 AS "email_bucket"
FROM "users";
```

### Math `unit_hash` (DuckDB)

```toml options
dialect = "duckdb"
```

> A deterministic pseudo-random number between 0 and 1, derived from each user's email, targeting DuckDB

```qd
#users $email|unit_hash -> email_bucket
```

```sql
SELECT
  CAST(hash("users"."email") AS DOUBLE) / 18446744073709551615.0 AS "email_bucket"
FROM "users";
```

### Date `year`, `month`, and `day`

> The calendar components of each issue's creation date, as integers

```qd
#issues $id $created_at|year $created_at|month $created_at|day
```

```sql
SELECT
  "issues"."id",
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER),
  CAST(EXTRACT(month FROM "issues"."created_at") AS INTEGER),
  CAST(EXTRACT(day FROM "issues"."created_at") AS INTEGER)
FROM "issues";
```

### Date `year` within a condition

> Issues created in 2023

```qd
#issues created_at|year:2023 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) = 2023;
```

### Date `year` of a date column

> Users born before 2000, applying `year` to a `date` (rather than a timestamp)

```qd
#users birth_date|year:<2000 $username
```

```sql
SELECT
  "users"."username"
FROM "users"
WHERE
  CAST(EXTRACT(year FROM "users"."birth_date") AS INTEGER) < 2000;
```

### Date `year_month`

> The number of issues created in each month

```qd
#issues $created_at|year_month \g $%count
```

```sql
SELECT
  to_char("issues"."created_at", 'YYYY-MM'),
  count(*)
FROM "issues"
GROUP BY to_char("issues"."created_at", 'YYYY-MM');
```

### Date `year_month` (DuckDB)

```toml options
dialect = "duckdb"
```

> The number of issues created in each month, targeting DuckDB

```qd
#issues $created_at|year_month \g $%count
```

```sql
SELECT
  strftime("issues"."created_at", '%Y-%m'),
  count(*)
FROM "issues"
GROUP BY strftime("issues"."created_at", '%Y-%m');
```

### Date components of a zoned timestamp

> `last_seen` is a zoned `timestamptz`. Postgres reads its components in the session time zone.

```qd
#users $username $last_seen|year $last_seen|year_month
```

```sql
SELECT
  "users"."username",
  CAST(EXTRACT(year FROM "users"."last_seen") AS INTEGER),
  to_char("users"."last_seen", 'YYYY-MM')
FROM "users";
```

### Date components of a zoned timestamp (DuckDB)

> DuckDB (without the ICU extension) can't operate on zoned timestamps, so `last_seen` is first
> coerced to a naive timestamp, and its components are read from the UTC wall-clock value.

```toml options
dialect = "duckdb"
```

```qd
#users $username $last_seen|year $last_seen|year_month
```

```sql
SELECT
  "users"."username",
  CAST(EXTRACT(year FROM CAST("users"."last_seen" AS TIMESTAMP)) AS INTEGER),
  strftime(CAST("users"."last_seen" AS TIMESTAMP), '%Y-%m')
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
  "users"."birth_date",
  "users"."last_seen"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
ORDER BY
  char_length("issues"."description") DESC NULLS LAST,
  "users"."username" DESC NULLS LAST,
  "issues"."title" DESC NULLS LAST;
```

## Result column spec nesting

### Basic nesting

> Comments, showing details of each comment's issue. The `$issue.( ... )` group applies `issue.`
> to the start of every result column inside it, so this is the same as writing `$issue.id`,
> `$issue.title`, `$issue.#labels.name%list(\\name)`, and `$issue.project.name`.

```qd
#comments
$id @{hide:yes} // the comment id
$created_at
$issue.(
  $id @{hide:yes} // the issue id
  $title @{width:400}
  $#labels.name%list(\\name)
  $project.name
)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "labels"."name" ORDER BY "labels"."name" ASC NULLS LAST) AS "v1"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "comments"."id",
  "comments"."created_at",
  "issues"."id",
  "issues"."title",
  "cte0"."v1",
  "projects"."name"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id";
```

```json
{
  "columnAnnotations": [
    { "hide": "yes" },
    null,
    { "hide": "yes" },
    { "width": 400 },
    null,
    null
  ]
}
```

### Nesting within nesting

> Groups may be nested within groups; the paths compose.

```qd
#comments $id $issue.( $title $project.( $name->project_name ) )
```

```sql
SELECT
  "comments"."id",
  "issues"."title",
  "projects"."name" AS "project_name"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id";
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
  "users"."last_seen",
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

### Aggregating a computed column across a to-many path

> Show each project alongside the furthest stage reached by any of its issues. The `stage` computed
> column is evaluated per issue within the CTE that aggregates the issues, including the nested CTE
> it needs for its own `#assignments` count.

```qd
#issues.stage = ? #assignments:>0~2 !description:@null~1 ~~0
#projects $name $#issues.stage%max
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "assignments"."issue" AS "pk"
        FROM "assignments"
        GROUP BY "assignments"."issue"
      )
    SELECT
      "issues"."project" AS "pk",
      max(CASE
        WHEN "cte0"."pk" IS NOT NULL THEN 2
        WHEN NOT "issues"."description" IS NULL THEN 1
        ELSE 0
      END) AS "v1"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Aggregating a computed column that reaches through a to-one link

> The average age of the authors of each project's issues. `author_age` follows the `author` link
> from each issue, so the CTE joins `users`.

```qd
#users.age = birth_date|age|years|floor
#issues.author_age = author.age
#projects $name $#issues.author_age%avg
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      avg(FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600)) AS "v1"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Comparing an aggregated computed column according to its type

> Projects with an issue opened within the past week. Because `opened` is defined as a datetime
> column, its aggregate compares against `1w` just as `#issues.created_at%max` would.

```qd
#issues.opened = created_at
#projects #issues.opened%max:<1w $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      max("issues"."created_at") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "cte0"."v1")) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Aggregating a computed column in a grouped query

> The furthest stage reached by any issue in each project, computed by grouping the issues
> themselves rather than through a `#` path.

```qd
#issues.stage = ? #assignments:>0~2 !description:@null~1 ~~0
#issues $project \g $stage%max
```

```sql
WITH
  "cte0" AS (
    SELECT
      "assignments"."issue" AS "pk"
    FROM "assignments"
    GROUP BY "assignments"."issue"
  )
SELECT
  "issues"."project",
  max(CASE
    WHEN "cte0"."pk" IS NOT NULL THEN 2
    WHEN NOT "issues"."description" IS NULL THEN 1
    ELSE 0
  END)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
GROUP BY "issues"."project";
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
      lower("comments"."body" COLLATE "C") = lower('+1' COLLATE "C")
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
      lower("users"."username" COLLATE "C") = lower('david' COLLATE "C")
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "assignments"."issue" AS "pk"
    FROM "assignments"
    LEFT JOIN "users" ON
      "assignments"."user" = "users"."id"
    WHERE
      lower("users"."username" COLLATE "C") = lower('david' COLLATE "C")
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
    OR lower("users"."username" COLLATE "C") = lower('david' COLLATE "C")
  );
```

### Linked record comparison

> A table can define a `__querydown_linked_record_comparison` custom comparison to give meaning to
> comparing directly against one of its records. Here `author:alice` means `author.username:alice`
> because the `users` table's linked-record comparison matches on `username`.

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#issues author:alice $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE);
```

### Linked record comparison reached through another link

> The left-hand side may be a multi-hop chain of to-one links. Here the comparison is defined on
> `products`, and `project.product` reaches a single `products` record via `issues` → `projects` →
> `products`, so `project.product:widget` means `project.product.name:widget`.

```qd
#products.__querydown_linked_record_comparison:@x = name:@x
#issues project.product:widget $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id"
WHERE
  COALESCE(strpos(lower("products"."name" COLLATE "C"), lower('widget' COLLATE "C")) > 0, FALSE);
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
  "pipe0"."project",
  "pipe0"."issue_count"
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
  "pipe1"."project",
  "pipe1"."issue_count"
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

> The most recent issue in each project. Because SQL forbids window functions in `WHERE`, the
> window value is computed in a CTE, which the query then reads from in place of its base table,
> under the base table's own name.

```qd
#issues %%(project\p created_at\sd)%row_number:1 $id $title $project
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "issues"."id",
      "issues"."title",
      "issues"."description",
      "issues"."created_at",
      "issues"."author",
      "issues"."status",
      "issues"."project",
      "issues"."duplicate_of",
      "issues"."due_date",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
  )
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."project"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Filtering on a window function alongside an ordinary filter

> Non-window conditions are applied within the CTE, before the window function is computed; only
> the window predicate is applied in the outer query.

```qd
#issues status:="open" %%(project\p created_at\sd)%row_number:1 $id $project
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "issues"."id",
      "issues"."title",
      "issues"."description",
      "issues"."created_at",
      "issues"."author",
      "issues"."status",
      "issues"."project",
      "issues"."duplicate_of",
      "issues"."due_date",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
    WHERE
      lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C")
  )
SELECT
  "issues"."id",
  "issues"."project"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Filtering on a window function with an empty display section

> With no result columns, every column of the base table is listed explicitly.

```qd
#issues %%(project\p created_at\sd)%row_number:1
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "issues"."id",
      "issues"."title",
      "issues"."description",
      "issues"."created_at",
      "issues"."author",
      "issues"."status",
      "issues"."project",
      "issues"."duplicate_of",
      "issues"."due_date",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
  )
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
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Filtering on a window function with sorting and related records

> Everything apart from the window predicate compiles as it would without it: here, sorting and
> a join to a related record.

```qd
#issues %%(project\p created_at\sd)%row_number:1 $title \s $author.username
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "issues"."id",
      "issues"."title",
      "issues"."description",
      "issues"."created_at",
      "issues"."author",
      "issues"."status",
      "issues"."project",
      "issues"."duplicate_of",
      "issues"."due_date",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
  )
SELECT
  "issues"."title",
  "users"."username"
FROM "qdwin_src" AS "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  "issues"."qdwin0" = 1
ORDER BY
  "issues"."title" ASC NULLS LAST;
```

### Filtering on a window function before grouping

> Conditions filter the rows before they're grouped, including window conditions. Here, the
> second stage keeps the users who tie for the most comments on each issue, then groups them by
> issue.

```qd
#comments $issue \g $user.username \g $%count -> n
~~~
%%(issue\p n\sd)%rank:1 $issue \g $username%list
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "comments"."issue" AS "issue",
      "users"."username" AS "username",
      count(*) AS "n"
    FROM "comments"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    GROUP BY "comments"."issue", "users"."username"
  ),
  "qdwin_src" AS (
    SELECT
      "pipe0"."issue",
      "pipe0"."username",
      "pipe0"."n",
      rank() OVER (PARTITION BY "pipe0"."issue" ORDER BY "pipe0"."n" DESC NULLS LAST) AS "qdwin0"
    FROM "pipe0"
  )
SELECT
  "pipe0"."issue",
  array_agg(DISTINCT "pipe0"."username")
FROM "qdwin_src" AS "pipe0"
WHERE
  "pipe0"."qdwin0" = 1
GROUP BY "pipe0"."issue";
```

### Filtering on a window function within a function call

> A window function anywhere within a condition's expressions is computed in the CTE.

```qd
#issues %%(project\p created_at\sd)%row_number|plus(1):2 $id
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "issues"."id",
      "issues"."title",
      "issues"."description",
      "issues"."created_at",
      "issues"."author",
      "issues"."status",
      "issues"."project",
      "issues"."duplicate_of",
      "issues"."due_date",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" + 1 = 2;
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
