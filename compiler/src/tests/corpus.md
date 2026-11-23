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

### Raw string

> Strings are raw: a backslash is literal, so `\d` reaches the regex intact and `\n` is a backslash
> followed by `n`, not a newline.

```qd
#issues title:~'v\d+' description:="a\nb"
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
  "issues"."title" ~* '(?p)v\d+' AND
  lower("issues"."description" COLLATE "C") = lower('a\nb' COLLATE "C");
```

### String containing a quote

> A string can't contain its own quote character, so one containing `'` is quoted with `"`.

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
  lower("issues"."title" COLLATE "C") = lower('can''t' COLLATE "C");
```

### Flagged string with a custom quote

> A flagged string can be quoted with `^` (among others), so it can contain both kinds of quote.

```qd
#issues title:=^^it's "done"^ $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  lower("issues"."title" COLLATE "C") = lower('it''s "done"' COLLATE "C");
```

### Flagged string with escape sequences

> The `e` flag interprets escape sequences.

```qd
#issues title:=^e'it\'s caf\u{e9}' $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  lower("issues"."title" COLLATE "C") = lower('it''s café' COLLATE "C");
```

### Format string

> The `f` flag interpolates expressions into the string, via SQL `concat`.

```qd
#users $^f"{username} <{email}>" -> contact
```

```sql
SELECT
  concat("users"."username", ' <', "users"."email", '>') AS "contact"
FROM "users";
```

### Format string (DuckDB)

```toml options
dialect = "duckdb"
```

```qd
#users $^f"{username} <{email}>" -> contact
```

```sql
SELECT
  concat("users"."username", ' <', "users"."email", '>') AS "contact"
FROM "users";
```

### Format string on the right-hand side of a comparison

> A bare word within an interpolation is a column reference, even on the right-hand side of a
> comparison.

```qd
#issues description:^f"see {title}" $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower(concat('see ', "issues"."title") COLLATE "C")) > 0, FALSE);
```

### Format string containing its quote character within an interpolation

> The string ends at the first quote character outside of an interpolation, so `|` can quote a
> string whose interpolations use pipes.

```qd
#issues $^f|{title|uppercase}!| -> shout
```

```sql
SELECT
  concat(upper("issues"."title"), '!') AS "shout"
FROM "issues";
```

### Format string as a search term

> Like any string standing alone as a condition, a format string searches the table's text
> columns.

```qd
#issues ^f"#{id}" $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower(concat('#', "issues"."id") COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower(concat('#', "issues"."id") COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower(concat('#', "issues"."id") COLLATE "C")) > 0, FALSE));
```

### Format string with an aggregate

```qd
#issues $project \g $^f"{%count} issues" -> summary
```

```sql
SELECT
  "issues"."project",
  concat(count(*), ' issues') AS "summary"
FROM "issues"
GROUP BY "issues"."project";
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
  "users"."last_seen",
  "users"."preferences"
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
  "users"."last_seen",
  "users"."preferences"
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
  "users"."last_seen",
  "users"."preferences"
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
  "users"."last_seen",
  "users"."preferences"
FROM "users"
WHERE
  CAST("users"."last_seen" AS TIMESTAMP) > DATE '2023-03-04';
```

### Negated comparison

> Issues whose status is not "open", including issues with no status

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
  (lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C")) IS NOT TRUE;
```

### Negated expression as a result column

> Whether each project is inactive, as a boolean column

```qd
#projects $!is_active->inactive
```

```sql
SELECT
  "projects"."is_active" IS NOT TRUE AS "inactive"
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
  (lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") AND
  lower("issues"."status" COLLATE "C") = lower('high' COLLATE "C")) IS NOT TRUE;
```

### Negated comparison through a to-one path

> Issues that aren't in the "Website" project, including issues with no project. The LEFT JOIN gives an issue with no project a NULL name, and `IS NOT TRUE` keeps it.

```qd
#issues !project.name:="Website" $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  (lower("projects"."name" COLLATE "C") = lower('Website' COLLATE "C")) IS NOT TRUE;
```

### Negated aggregate comparison

> Issues with no comment in the past week, including issues with no comments at all (whose `max` is NULL)

```qd
#issues !#comments.created_at%max:<1w $id
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
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  (ABS(EXTRACT(epoch FROM NOW() - "cte0"."v1")) < EXTRACT(epoch FROM make_interval(weeks => 1))) IS NOT TRUE;
```

### Negated OR condition set

> Issues that are neither in project 3 nor closed. An open issue with no project gives `NULL OR FALSE`, which is NULL, so the negation must still keep it.

```qd
#issues ![project.id:3 status:="closed"] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  ("projects"."id" = 3 OR lower("issues"."status" COLLATE "C") = lower('closed' COLLATE "C")) IS NOT TRUE;
```

### Negated condition that is never NULL

> `IS NULL` is never NULL, so a plain `NOT` already negates it correctly

```qd
#issues !duplicate_of:@null $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  NOT "issues"."duplicate_of" IS NULL;
```

### Double negation

```qd
#issues !!project.is_active $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  NOT ("projects"."is_active" IS NOT TRUE);
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
  "issues"."title" ~* '(?p)foo';
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
  ("issues"."title" ~* '(?p)color' OR "issues"."title" ~* '(?p)colour');
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
  ("issues"."title" ~* '(?p)color' OR "issues"."title" ~* '(?p)colour') AND
  ("issues"."description" ~* '(?p)color' OR "issues"."description" ~* '(?p)colour');
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

### Regex with the case-sensitive flag

> The `c` flag makes the match case-sensitive, using `~` rather than `~*`. Every Postgres regex
> begins with an embedded option setting its newline handling; with no `m` or `s` flag, that's `p`
> (partial newline-sensitive), so that `.` doesn't match a newline.

```qd
#issues title:\c~'^Foo' $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."title" ~ '(?p)^Foo';
```

### Regex with the multi-line flag

> With `m`, `^` and `$` match at line breaks (Postgres's `n` option).

```qd
#issues description:\m~'^todo:' $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."description" ~* '(?n)^todo:';
```

### Regex with the dot-all flag

> With `s`, `.` matches a newline, which is Postgres's default, so the pattern needs no option.

```qd
#issues description:\s~'a.b' $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."description" ~* 'a.b';
```

### Regex with all flags

> The flags may be given in any order. `m` with `s` is Postgres's `w` option.

```qd
#issues description:\smc~'^a.b$' $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."description" ~ '(?w)^a.b$';
```

### Regex flags with a pattern from a column

> A literal pattern absorbs the option at compile time; any other is concatenated with it.

```qd
#issues title:\s~`description` description:~`title` $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."title" ~* "issues"."description" AND
  "issues"."description" ~* ('(?p)' || "issues"."title");
```

### Negated regex with flags

```qd
#issues !title:\c~foo $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ("issues"."title" ~ '(?p)foo') IS NOT TRUE;
```

### Regex flags (DuckDB)

```toml options
dialect = "duckdb"
```

> RE2 shares Python's defaults, so a pattern needs an inline flag only for `m` or `s`.

```qd
#issues title:\c~'^Foo' description:\m~'^todo:' $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  regexp_matches("issues"."title", '^Foo') AND
  regexp_matches("issues"."description", '(?m)^todo:', 'i');
```

### Regex flags with a pattern from a column (DuckDB)

```toml options
dialect = "duckdb"
```

```qd
#issues description:\ms~'^a.b$' title:~`description` title:\s~`description` $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  regexp_matches("issues"."description", '(?ms)^a.b$', 'i') AND
  regexp_matches("issues"."title", "issues"."description", 'i') AND
  regexp_matches("issues"."title", ('(?s)' || "issues"."description"), 'i');
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
          ("comments"."user" = "issues"."author") IS NOT TRUE
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
          ("comments"."user" = "issues"."author") IS NOT TRUE
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
  "users"."last_seen",
  "users"."preferences"
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
  "users"."last_seen",
  "users"."preferences"
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
  "users"."last_seen",
  "users"."preferences"
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
  "users"."last_seen",
  "users"."preferences"
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

### Math `ln`

> The natural logarithm of each issue id

```qd
#issues $id|ln
```

```sql
SELECT
  LN("issues"."id")
FROM "issues";
```

### Math `gcd` and `lcm`

> The greatest common divisor and least common multiple of each issue id and 12

```qd
#issues $id|gcd(12) $id|lcm(12)
```

```sql
SELECT
  gcd("issues"."id", 12),
  lcm("issues"."id", 12)
FROM "issues";
```

### Math `round`

> How many days old each issue is, to one decimal place

```qd
#issues $title $created_at|age|days|round(1) -> days_old
```

```sql
SELECT
  "issues"."title",
  ROUND(CAST(EXTRACT(epoch FROM NOW() - "issues"."created_at") / 86400 AS numeric), 1) AS "days_old"
FROM "issues";
```

### Math `round` (DuckDB)

```toml options
dialect = "duckdb"
```

> How many days old each issue is, to one decimal place, targeting DuckDB

```qd
#issues $title $created_at|age|days|round(1) -> days_old
```

```sql
SELECT
  "issues"."title",
  round(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at") / 86400, 1) AS "days_old"
FROM "issues";
```

### Math `round` without digits

> How many whole days old each issue is, to the nearest day

```qd
#issues $title $created_at|age|days|round
```

```sql
SELECT
  "issues"."title",
  ROUND(CAST(EXTRACT(epoch FROM NOW() - "issues"."created_at") / 86400 AS numeric))
FROM "issues";
```

### Math `round` without digits (DuckDB)

```toml options
dialect = "duckdb"
```

> How many whole days old each issue is, to the nearest day, targeting DuckDB

```qd
#issues $title $created_at|age|days|round
```

```sql
SELECT
  "issues"."title",
  round(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at") / 86400)
FROM "issues";
```

### Math `round` to hundreds

> Each invoice's amount, to the nearest hundred

```qd
#/billing/invoices $id $amount|round(-2)
```

```sql
SELECT
  "invoices"."id",
  ROUND(CAST("invoices"."amount" AS numeric), -2)
FROM "billing"."invoices" AS "invoices";
```

### Math `round` to hundreds (DuckDB)

```toml options
dialect = "duckdb"
```

> Each invoice's amount, to the nearest hundred, targeting DuckDB

```qd
#/billing/invoices $id $amount|round(-2)
```

```sql
SELECT
  "invoices"."id",
  CASE
    WHEN isfinite("invoices"."amount") THEN round("invoices"."amount", -2)
    ELSE "invoices"."amount"
  END
FROM "billing"."invoices" AS "invoices";
```

### Math `round` to a computed number of digits

> Each invoice's amount, rounded to as many decimal places as its id

```qd
#/billing/invoices $id $amount|round(id)
```

```sql
SELECT
  "invoices"."id",
  ROUND(CAST("invoices"."amount" AS numeric), CAST("invoices"."id" AS INTEGER))
FROM "billing"."invoices" AS "invoices";
```

### Math `round` to a computed number of digits (DuckDB)

```toml options
dialect = "duckdb"
```

> Each invoice's amount, rounded to as many decimal places as its id, targeting DuckDB

```qd
#/billing/invoices $id $amount|round(id)
```

```sql
SELECT
  "invoices"."id",
  CASE
    WHEN isfinite("invoices"."amount") THEN round(CAST("invoices"."amount" AS DOUBLE), CAST("invoices"."id" AS INTEGER))
    ELSE "invoices"."amount"
  END
FROM "billing"."invoices" AS "invoices";
```

### Math `is_nan`

> Invoices whose amount is NaN

```qd
#/billing/invoices amount|is_nan $id
```

```sql
SELECT
  "invoices"."id"
FROM "billing"."invoices" AS "invoices"
WHERE
  "invoices"."amount" = CAST('NaN' AS double precision);
```

### Math `is_nan` (DuckDB)

```toml options
dialect = "duckdb"
```

> Invoices whose amount is NaN, targeting DuckDB

```qd
#/billing/invoices amount|is_nan $id
```

```sql
SELECT
  "invoices"."id"
FROM "billing"."invoices" AS "invoices"
WHERE
  isnan("invoices"."amount");
```

### Duration `seconds_to_duration`

> An hour after each issue was created

```qd
#issues $created_at|plus(3600|seconds_to_duration) -> an_hour_later
```

```sql
SELECT
  "issues"."created_at" + make_interval(secs => 3600) AS "an_hour_later"
FROM "issues";
```

### Duration `seconds_to_duration` (DuckDB)

```toml options
dialect = "duckdb"
```

> An hour after each issue was created, targeting DuckDB

```qd
#issues $created_at|plus(3600|seconds_to_duration) -> an_hour_later
```

```sql
SELECT
  "issues"."created_at" + to_seconds(3600) AS "an_hour_later"
FROM "issues";
```

### Text `split` and array `at`

> The domain of each user's email address

```qd
#users $email|split("@")|at(1) -> domain
```

```sql
SELECT
  (string_to_array("users"."email", '@'))[2] AS "domain"
FROM "users";
```

### Text `split` and array `at` (DuckDB)

```toml options
dialect = "duckdb"
```

> The domain of each user's email address, targeting DuckDB

```qd
#users $email|split("@")|at(1) -> domain
```

```sql
SELECT
  list_extract(CASE
    WHEN "users"."email" = '' THEN []
    ELSE string_split("users"."email", '@')
  END, 2) AS "domain"
FROM "users";
```

### Text `split` at a column's value

> The parts of each user's email between occurrences of their username

```qd
#users $email|split(username)
```

```sql
SELECT
  CASE
    WHEN "users"."username" IS NOT NULL THEN string_to_array("users"."email", NULLIF("users"."username", ''))
    ELSE NULL
  END
FROM "users";
```

### Text `split` at a column's value (DuckDB)

```toml options
dialect = "duckdb"
```

> The parts of each user's email between occurrences of their username, targeting DuckDB

```qd
#users $email|split(username)
```

```sql
SELECT
  CASE
    WHEN "users"."username" IS NULL THEN NULL
    WHEN "users"."email" = '' THEN []
    ELSE string_split("users"."email", "users"."username")
  END
FROM "users";
```

### Text `split` into characters

> The characters of each username

```qd
#users $username|split("")
```

```sql
SELECT
  string_to_array("users"."username", NULL)
FROM "users";
```

### Text `split` into characters (DuckDB)

```toml options
dialect = "duckdb"
```

> The characters of each username, targeting DuckDB

```qd
#users $username|split("")
```

```sql
SELECT
  CASE
    WHEN "users"."username" = '' THEN []
    ELSE string_split("users"."username", '')
  END
FROM "users";
```

### Array `at` from the end

> The body of each issue's latest comment

```qd
#issues $#comments.body%list_all(\\created_at)|at(-1) -> latest_comment
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg("comments"."body" ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  ("cte0"."v1")[cardinality("cte0"."v1")] AS "latest_comment"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Array `at` from the end (DuckDB)

```toml options
dialect = "duckdb"
```

> The body of each issue's latest comment, targeting DuckDB

```qd
#issues $#comments.body%list_all(\\created_at)|at(-1) -> latest_comment
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg("comments"."body" ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  list_extract("cte0"."v1", -1) AS "latest_comment"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Array `at` from the end, second to last

> The second word from the end of each issue's title

```qd
#issues $title|split(" ")|at(-2)
```

```sql
SELECT
  (string_to_array("issues"."title", ' '))[cardinality(string_to_array("issues"."title", ' ')) - 1]
FROM "issues";
```

### Array `at` from the end, second to last (DuckDB)

```toml options
dialect = "duckdb"
```

> The second word from the end of each issue's title, targeting DuckDB

```qd
#issues $title|split(" ")|at(-2)
```

```sql
SELECT
  list_extract(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, -2)
FROM "issues";
```

### Array `at` a computed position

> A word of each issue's title, chosen by its id

```qd
#issues $title|split(" ")|at(id|mod(3))
```

```sql
SELECT
  (string_to_array("issues"."title", ' '))[CASE
    WHEN "issues"."id" % 3 < 0 THEN cardinality(string_to_array("issues"."title", ' ')) + "issues"."id" % 3 + 1
    ELSE "issues"."id" % 3 + 1
  END]
FROM "issues";
```

### Array `at` a computed position (DuckDB)

```toml options
dialect = "duckdb"
```

> A word of each issue's title, chosen by its id, targeting DuckDB

```qd
#issues $title|split(" ")|at(id|mod(3))
```

```sql
SELECT
  list_extract(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, CASE
    WHEN "issues"."id" % 3 < 0 THEN "issues"."id" % 3
    ELSE "issues"."id" % 3 + 1
  END)
FROM "issues";
```

### Array element compared with the match operator

> Users whose email domain contains "example"

```qd
#users email|split("@")|at(1):example $id
```

```sql
SELECT
  "users"."id"
FROM "users"
WHERE
  COALESCE(strpos(lower((string_to_array("users"."email", '@'))[2] COLLATE "C"), lower('example' COLLATE "C")) > 0, FALSE);
```

### Array `join`

> The names of each issue's labels, in one string

```qd
#issues $title $#labels.name%list(\s)|join(", ") -> labels
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
  "issues"."title",
  array_to_string("cte0"."v1", ', ') AS "labels"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Array `join` (DuckDB)

```toml options
dialect = "duckdb"
```

> The names of each issue's labels, in one string, targeting DuckDB

```qd
#issues $title $#labels.name%list(\s)|join(", ") -> labels
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
  "issues"."title",
  array_to_string(list_filter("cte0"."v1", lambda e: e IS NOT NULL), ', ') AS "labels"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Array `join` with a column's value

> The words of each issue's title, joined by its status

```qd
#issues $title|split(" ")|join(status)
```

```sql
SELECT
  array_to_string(string_to_array("issues"."title", ' '), "issues"."status")
FROM "issues";
```

### Array `join` with a column's value (DuckDB)

```toml options
dialect = "duckdb"
```

> The words of each issue's title, joined by its status, targeting DuckDB

```qd
#issues $title|split(" ")|join(status)
```

```sql
SELECT
  CASE
    WHEN "issues"."status" IS NOT NULL THEN substr(list_reduce(list_transform(list_filter(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda e: e IS NOT NULL), lambda e: CAST(e AS VARCHAR)), lambda acc, e: acc || "issues"."status" || e, ''), length("issues"."status") + 1)
    ELSE NULL
  END
FROM "issues";
```

### Array `array`

> Each user's username and email, as an array

```qd
#users $@@array(username email) -> names
```

```sql
SELECT
  ARRAY["users"."username", "users"."email"] AS "names"
FROM "users";
```

### Array `array` (DuckDB)

```toml options
dialect = "duckdb"
```

> Each user's username and email, as an array, targeting DuckDB

```qd
#users $@@array(username email) -> names
```

```sql
SELECT
  ["users"."username", "users"."email"] AS "names"
FROM "users";
```

### Array `length`

> The number of users who commented on each issue

```qd
#issues $title $#comments.user%list|length -> commenters
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg(DISTINCT "comments"."user") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title",
  cardinality("cte0"."v1") AS "commenters"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Array `length` (DuckDB)

```toml options
dialect = "duckdb"
```

> The number of users who commented on each issue, targeting DuckDB

```qd
#issues $title $#comments.user%list|length -> commenters
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg(DISTINCT "comments"."user") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title",
  len("cte0"."v1") AS "commenters"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Array `length` of an array from a function

> The number of words in each issue's title

```qd
#issues $title|split(" ")|length -> words
```

```sql
SELECT
  cardinality(string_to_array("issues"."title", ' ')) AS "words"
FROM "issues";
```

### Array `length` of an array from a function (DuckDB)

```toml options
dialect = "duckdb"
```

> The number of words in each issue's title, targeting DuckDB

```qd
#issues $title|split(" ")|length -> words
```

```sql
SELECT
  len(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END) AS "words"
FROM "issues";
```

### Array `map`

> The words of each issue's title, in upper case

```qd
#issues $title|split(" ")|map(@w => @w|uppercase) -> words
```

```sql
SELECT
  (SELECT ARRAY(SELECT upper("w") FROM unnest("array") WITH ORDINALITY AS "w"("w", "i") ORDER BY "i") FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL) AS "words"
FROM "issues";
```

### Array `map` (DuckDB)

```toml options
dialect = "duckdb"
```

> The words of each issue's title, in upper case, targeting DuckDB

```qd
#issues $title|split(" ")|map(@w => @w|uppercase) -> words
```

```sql
SELECT
  list_transform(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w": upper("w")) AS "words"
FROM "issues";
```

### Array `filter` by position

> The first three words of each issue's title

```qd
#issues $title|split(" ")|filter(@w @i => @i:<3) -> first_words
```

```sql
SELECT
  (SELECT ARRAY(SELECT "w" FROM unnest("array") WITH ORDINALITY AS "w"("w", "i") WHERE "i" - 1 < 3 ORDER BY "i") FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL) AS "first_words"
FROM "issues";
```

### Array `filter` by position (DuckDB)

```toml options
dialect = "duckdb"
```

> The first three words of each issue's title, targeting DuckDB

```qd
#issues $title|split(" ")|filter(@w @i => @i:<3) -> first_words
```

```sql
SELECT
  list_filter(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w", "i": "i" - 1 < 3) AS "first_words"
FROM "issues";
```

### Array `some` compared with a column

> Issues with a word in their title that is their status

```qd
#issues title|split(" ")|some(@w => @w:==`status`) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (SELECT EXISTS (SELECT 1 FROM unnest("array") AS "w"("w") WHERE "w" = "issues"."status") FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL);
```

### Array `some` compared with a column (DuckDB)

```toml options
dialect = "duckdb"
```

> Issues with a word in their title that is their status, targeting DuckDB

```qd
#issues title|split(" ")|some(@w => @w:==`status`) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  len(list_filter(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w": "w" = "issues"."status")) > 0;
```

### Array `every`

> Issues whose title has no word of ten characters or more

```qd
#issues title|split(" ")|every(@w => @w|length:<10) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (SELECT NOT EXISTS (SELECT 1 FROM unnest("array") AS "w"("w") WHERE (char_length("w") < 10) IS NOT TRUE) FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL);
```

### Array `every` (DuckDB)

```toml options
dialect = "duckdb"
```

> Issues whose title has no word of ten characters or more, targeting DuckDB

```qd
#issues title|split(" ")|every(@w => @w|length:<10) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  len(list_filter(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w": (char_length("w") < 10) IS NOT TRUE)) = 0;
```

### Array `reduce`

> The words of each issue's title, in reverse order

```qd
#issues $title|split(" ")|reduce(@a @b => ^f"{@b} {@a}") -> reversed
```

```sql
SELECT
  (WITH RECURSIVE "a"("i", "a", "array") AS (SELECT 1, "array"[1], "array" FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") UNION ALL SELECT "i" + 1, concat("array"["i" + 1], ' ', "a"), "array" FROM "a" WHERE "i" < cardinality("array")) SELECT "a" FROM "a" ORDER BY "i" DESC LIMIT 1) AS "reversed"
FROM "issues";
```

### Array `reduce` (DuckDB)

```toml options
dialect = "duckdb"
```

> The words of each issue's title, in reverse order, targeting DuckDB

```qd
#issues $title|split(" ")|reduce(@a @b => ^f"{@b} {@a}") -> reversed
```

```sql
SELECT
  list_reduce(CASE
    WHEN len(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END) = 0 THEN NULL
    ELSE CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END
  END, lambda "a", "b": concat("b", ' ', "a")) AS "reversed"
FROM "issues";
```

### Array functions within each other's functions

> The words of each issue's title that contain an "x", and the length of the longest word of each part of its description between commas

```qd
#issues $title|split(" ")|filter(@w => @w|split("")|some(@c => @c:=="x")) $description|split(",")|map(@p => @p|split(" ")|map(@w => @w|length)|reduce(@a @b => @a|max(@b)))
```

```sql
SELECT
  (SELECT ARRAY(SELECT "w" FROM unnest("array") WITH ORDINALITY AS "w"("w", "i") WHERE (SELECT EXISTS (SELECT 1 FROM unnest("array_1") AS "c"("c") WHERE "c" = 'x') FROM (SELECT string_to_array("w", NULL)) AS "array_1"("array_1") WHERE "array_1" IS NOT NULL) ORDER BY "i") FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL),
  (SELECT ARRAY(SELECT (WITH RECURSIVE "a"("i_4", "a", "array_4") AS (SELECT 1, "array_4"[1], "array_4" FROM (SELECT (SELECT ARRAY(SELECT char_length("w_1") FROM unnest("array_3") WITH ORDINALITY AS "w_1"("w_1", "i_3") ORDER BY "i_3") FROM (SELECT string_to_array("p", ' ')) AS "array_3"("array_3") WHERE "array_3" IS NOT NULL)) AS "array_4"("array_4") UNION ALL SELECT "i_4" + 1, GREATEST("a", "array_4"["i_4" + 1]), "array_4" FROM "a" WHERE "i_4" < cardinality("array_4")) SELECT "a" FROM "a" ORDER BY "i_4" DESC LIMIT 1) FROM unnest("array_2") WITH ORDINALITY AS "p"("p", "i_2") ORDER BY "i_2") FROM (SELECT string_to_array("issues"."description", ',')) AS "array_2"("array_2") WHERE "array_2" IS NOT NULL)
FROM "issues";
```

### Array functions within each other's functions (DuckDB)

```toml options
dialect = "duckdb"
```

> The words of each issue's title that contain an "x", and the length of the longest word of each part of its description between commas, targeting DuckDB

```qd
#issues $title|split(" ")|filter(@w => @w|split("")|some(@c => @c:=="x")) $description|split(",")|map(@p => @p|split(" ")|map(@w => @w|length)|reduce(@a @b => @a|max(@b)))
```

```sql
SELECT
  list_filter(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w": len(list_filter(CASE
    WHEN "w" = '' THEN []
    ELSE string_split("w", '')
  END, lambda "c": "c" = 'x')) > 0),
  list_transform(CASE
    WHEN "issues"."description" = '' THEN []
    ELSE string_split("issues"."description", ',')
  END, lambda "p": list_reduce(CASE
    WHEN len(list_transform(CASE
    WHEN "p" = '' THEN []
    ELSE string_split("p", ' ')
  END, lambda "w_1": char_length("w_1"))) = 0 THEN NULL
    ELSE list_transform(CASE
    WHEN "p" = '' THEN []
    ELSE string_split("p", ' ')
  END, lambda "w_1": char_length("w_1"))
  END, lambda "a", "b": GREATEST("a", "b")))
FROM "issues";
```

### Array function parameter that shadows a constant

> Each word of an issue's title after the issue's id, through a function whose parameter has the name of a constant, within an anonymous function

```qd
@w = "x"
#issues $id|(@i => title|split(" ")|map(@w => ^f"{@i}:{@w}"))
```

```sql
SELECT
  (SELECT ARRAY(SELECT concat("issues"."id", ':', "w") FROM unnest("array") WITH ORDINALITY AS "w"("w", "i") ORDER BY "i") FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL)
FROM "issues";
```

### Array function parameter that shadows a constant (DuckDB)

```toml options
dialect = "duckdb"
```

> Each word of an issue's title after the issue's id, through a function whose parameter has the name of a constant, within an anonymous function, targeting DuckDB

```qd
@w = "x"
#issues $id|(@i => title|split(" ")|map(@w => ^f"{@i}:{@w}"))
```

```sql
SELECT
  list_transform(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w": concat("issues"."id", ':', "w"))
FROM "issues";
```

### Array functions in definitions

> The long words of each issue's title, and whether its description has one, through a computed column and a function

```qd
#issues.long_words = title|split(" ")|filter(@w => @w|length:>8)
@@has_long_word = @t => @t|split(" ")|some(@w => @w|length:>8)
#issues $long_words $description|has_long_word
```

```sql
SELECT
  (SELECT ARRAY(SELECT "w" FROM unnest("array") WITH ORDINALITY AS "w"("w", "i") WHERE char_length("w") > 8 ORDER BY "i") FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL),
  (SELECT EXISTS (SELECT 1 FROM unnest("array_1") AS "w_1"("w_1") WHERE char_length("w_1") > 8) FROM (SELECT string_to_array("issues"."description", ' ')) AS "array_1"("array_1") WHERE "array_1" IS NOT NULL)
FROM "issues";
```

### Array functions in definitions (DuckDB)

```toml options
dialect = "duckdb"
```

> The long words of each issue's title, and whether its description has one, through a computed column and a function, targeting DuckDB

```qd
#issues.long_words = title|split(" ")|filter(@w => @w|length:>8)
@@has_long_word = @t => @t|split(" ")|some(@w => @w|length:>8)
#issues $long_words $description|has_long_word
```

```sql
SELECT
  list_filter(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w": char_length("w") > 8),
  len(list_filter(CASE
    WHEN "issues"."description" = '' THEN []
    ELSE string_split("issues"."description", ' ')
  END, lambda "w_1": char_length("w_1") > 8)) > 0
FROM "issues";
```

### Array functions on an aggregate in a grouped query

> For each status, its issues' titles in upper case

```qd
#issues $status \g $title%list|map(@t => @t|uppercase) -> titles
```

```sql
SELECT
  "issues"."status",
  (SELECT ARRAY(SELECT upper("t") FROM unnest("array") WITH ORDINALITY AS "t"("t", "i") ORDER BY "i") FROM (SELECT array_agg(DISTINCT "issues"."title")) AS "array"("array") WHERE "array" IS NOT NULL) AS "titles"
FROM "issues"
GROUP BY "issues"."status";
```

### Array functions on an aggregate in a grouped query (DuckDB)

```toml options
dialect = "duckdb"
```

> For each status, its issues' titles in upper case, targeting DuckDB

```qd
#issues $status \g $title%list|map(@t => @t|uppercase) -> titles
```

```sql
SELECT
  "issues"."status",
  list_transform(array_agg(DISTINCT "issues"."title"), lambda "t": upper("t")) AS "titles"
FROM "issues"
GROUP BY "issues"."status";
```

### Array function using an aggregate of related records

> Issues whose title has a word that is the body of their latest comment

```qd
#issues title|split(" ")|some(@w => #comments.body%list_all(\\created_at)|at(-1):==@w) $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg("comments"."body" ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  (SELECT EXISTS (SELECT 1 FROM unnest("array") AS "w"("w") WHERE ("cte0"."v1")[cardinality("cte0"."v1")] = "w") FROM (SELECT string_to_array("issues"."title", ' ')) AS "array"("array") WHERE "array" IS NOT NULL);
```

### Array function using an aggregate of related records (DuckDB)

```toml options
dialect = "duckdb"
```

> Issues whose title has a word that is the body of their latest comment, targeting DuckDB

```qd
#issues title|split(" ")|some(@w => #comments.body%list_all(\\created_at)|at(-1):==@w) $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg("comments"."body" ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  len(list_filter(CASE
    WHEN "issues"."title" = '' THEN []
    ELSE string_split("issues"."title", ' ')
  END, lambda "w": list_extract("cte0"."v1", -1) = "w")) > 0;
```

### Array function within the value of an aggregate

> Each issue, and whether a comment on it has the word "thanks"

```qd
#issues $title $#comments.body|split(" ")|some(@w => @w:="thanks")%any_true -> thanked
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      bool_or((SELECT EXISTS (SELECT 1 FROM unnest("array") AS "w"("w") WHERE lower("w" COLLATE "C") = lower('thanks' COLLATE "C")) FROM (SELECT string_to_array("comments"."body", ' ')) AS "array"("array") WHERE "array" IS NOT NULL)) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title",
  "cte0"."v1" AS "thanked"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Array function within the value of an aggregate (DuckDB)

```toml options
dialect = "duckdb"
```

> Each issue, and whether a comment on it has the word "thanks", targeting DuckDB

```qd
#issues $title $#comments.body|split(" ")|some(@w => @w:="thanks")%any_true -> thanked
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      bool_or(len(list_filter(CASE
        WHEN "comments"."body" = '' THEN []
        ELSE string_split("comments"."body", ' ')
      END, lambda "w": lower(strip_accents("w")) = lower(strip_accents('thanks')))) > 0) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title",
  "cte0"."v1" AS "thanked"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Text `regex_replace`

> Issue titles with a lowercase "fix" at the start capitalized

```qd
#issues $title|regex_replace('^fix' "Fix" "c")
```

```sql
SELECT
  regexp_replace("issues"."title", '(?p)^fix', 'Fix', 'g')
FROM "issues";
```

### Text `regex_replace` (DuckDB)

```toml options
dialect = "duckdb"
```

> Issue titles with a lowercase "fix" at the start capitalized, targeting DuckDB

```qd
#issues $title|regex_replace('^fix' "Fix" "c")
```

```sql
SELECT
  regexp_replace("issues"."title", '^fix', 'Fix', 'g')
FROM "issues";
```

### Text `regex_replace` with a group

> Each user's email with its "@" spelled out

```qd
#users $email|regex_replace('^(.*)@' '\1 at ')
```

```sql
SELECT
  regexp_replace("users"."email", '(?p)^(.*)@', '\1 at ', 'gi')
FROM "users";
```

### Text `regex_replace` with a group (DuckDB)

```toml options
dialect = "duckdb"
```

> Each user's email with its "@" spelled out, targeting DuckDB

```qd
#users $email|regex_replace('^(.*)@' '\1 at ')
```

```sql
SELECT
  regexp_replace("users"."email", '^(.*)@', '\1 at ', 'gi')
FROM "users";
```

### Text `regex_extract`

> The domain of each user's email address

```qd
#users $email|regex_extract('@(.*)$' 1) -> domain
```

```sql
SELECT
  (regexp_match("users"."email", '(?p)(@(.*)$)', 'i'))[2] AS "domain"
FROM "users";
```

### Text `regex_extract` (DuckDB)

```toml options
dialect = "duckdb"
```

> The domain of each user's email address, targeting DuckDB

```qd
#users $email|regex_extract('@(.*)$' 1) -> domain
```

```sql
SELECT
  CASE
    WHEN regexp_matches("users"."email", '@(.*)$', 'i') THEN regexp_extract("users"."email", '@(.*)$', 1, 'i')
    ELSE NULL
  END AS "domain"
FROM "users";
```

### Text `regex_extract` with flags and a pattern from a column

> The first match of each issue's title within its description, across lines

```qd
#issues $description|regex_extract(title 0 "ms")
```

```sql
SELECT
  (regexp_match("issues"."description", ('(?w)' || ('(' || "issues"."title" || ')')), 'i'))[1]
FROM "issues";
```

### Text `regex_extract` with flags and a pattern from a column (DuckDB)

```toml options
dialect = "duckdb"
```

> The first match of each issue's title within its description, across lines, targeting DuckDB

```qd
#issues $description|regex_extract(title 0 "ms")
```

```sql
SELECT
  CASE
    WHEN regexp_matches("issues"."description", ('(?ms)' || "issues"."title"), 'i') THEN regexp_extract("issues"."description", ('(?ms)' || "issues"."title"), 0, 'i')
    ELSE NULL
  END
FROM "issues";
```

### Text `regex_extract` with flags from a constant

> The first number in each issue's title, matched case-sensitively

```qd
@flags = "c"
#issues $title|regex_extract('\d+' 0 @flags)
```

```sql
SELECT
  (regexp_match("issues"."title", '(?p)(\d+)'))[1]
FROM "issues";
```

### Conversion `to_number`

> The first number in each issue's title, as a number

```qd
#issues $title|regex_extract('\d+')|to_number
```

```sql
SELECT
  CAST((regexp_match("issues"."title", '(?p)(\d+)', 'i'))[1] AS DOUBLE PRECISION)
FROM "issues";
```

### Conversion `to_date`

> The number of issues created on each day

```qd
#issues $created_at|to_date \g $%count
```

```sql
SELECT
  CAST("issues"."created_at" AS DATE),
  count(*)
FROM "issues"
GROUP BY CAST("issues"."created_at" AS DATE);
```

### Conversion `to_date` of a zoned timestamp

> The day each user was last seen

```qd
#users $username $last_seen|to_date
```

```sql
SELECT
  "users"."username",
  CAST("users"."last_seen" AS DATE)
FROM "users";
```

### Conversion `to_date` of a zoned timestamp (DuckDB)

```toml options
dialect = "duckdb"
```

> The day each user was last seen, targeting DuckDB

```qd
#users $username $last_seen|to_date
```

```sql
SELECT
  "users"."username",
  CAST(CAST("users"."last_seen" AS TIMESTAMP) AS DATE)
FROM "users";
```

### Conversion `to_timestamp`

> Issues created after 9 o'clock on 1 January 2024

```qd
#issues created_at:>"2024-01-01T09:00:00"|to_timestamp $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."created_at" > CAST('2024-01-01T09:00:00' AS TIMESTAMP);
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

### Aggregate `percentile`

> The median id of the issues in each project

```qd
#projects $name $#issues.id%percentile(0.5) -> median_id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("issues"."id" AS DOUBLE PRECISION)) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1" AS "median_id"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Aggregate `percentile` (DuckDB)

```toml options
dialect = "duckdb"
```

> The median id of the issues in each project, targeting DuckDB

```qd
#projects $name $#issues.id%percentile(0.5) -> median_id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("issues"."id" AS DOUBLE PRECISION)) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1" AS "median_id"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Aggregate `percentile` of computed values

> The length that 90% of each issue's comments are shorter than

```qd
#issues $id $#comments.body|length%percentile(0.9)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      percentile_cont(0.9) WITHIN GROUP (ORDER BY CAST(char_length("comments"."body") AS DOUBLE PRECISION)) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Aggregate `percentile` of computed values (DuckDB)

```toml options
dialect = "duckdb"
```

> The length that 90% of each issue's comments are shorter than, targeting DuckDB

```qd
#issues $id $#comments.body|length%percentile(0.9)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      percentile_cont(0.9) WITHIN GROUP (ORDER BY CAST(char_length("comments"."body") AS DOUBLE PRECISION)) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Aggregate `percentile` in a condition

> Issues whose comments have a median length over 200

```qd
#issues #comments.body|length%percentile(0.5):>200 $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST(char_length("comments"."body") AS DOUBLE PRECISION)) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."v1" > 200;
```

### Aggregate `percentile` in a condition (DuckDB)

```toml options
dialect = "duckdb"
```

> Issues whose comments have a median length over 200, targeting DuckDB

```qd
#issues #comments.body|length%percentile(0.5):>200 $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST(char_length("comments"."body") AS DOUBLE PRECISION)) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."v1" > 200;
```

### Aggregate `percentile` in sorting

> Projects, those with the greatest median issue id first

```qd
#projects \\#issues.id%percentile(0.5) \d $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("issues"."id" AS DOUBLE PRECISION)) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
ORDER BY
  "cte0"."v1" DESC NULLS LAST;
```

### Aggregate `percentile` in sorting (DuckDB)

```toml options
dialect = "duckdb"
```

> Projects, those with the greatest median issue id first, targeting DuckDB

```qd
#projects \\#issues.id%percentile(0.5) \d $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("issues"."id" AS DOUBLE PRECISION)) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
ORDER BY
  "cte0"."v1" DESC NULLS LAST;
```

### Aggregate `percentile` in a grouped query

> The quartiles of the lengths of issues' titles, for each status

```qd
#issues $status \g $title|length%percentile(0.25) -> q1 $title|length%percentile(0.5) -> q2 $title|length%percentile(0.75) -> q3
```

```sql
SELECT
  "issues"."status",
  percentile_cont(0.25) WITHIN GROUP (ORDER BY CAST(char_length("issues"."title") AS DOUBLE PRECISION)) AS "q1",
  percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST(char_length("issues"."title") AS DOUBLE PRECISION)) AS "q2",
  percentile_cont(0.75) WITHIN GROUP (ORDER BY CAST(char_length("issues"."title") AS DOUBLE PRECISION)) AS "q3"
FROM "issues"
GROUP BY "issues"."status";
```

### Aggregate `percentile` in a grouped query (DuckDB)

```toml options
dialect = "duckdb"
```

> The quartiles of the lengths of issues' titles, for each status, targeting DuckDB

```qd
#issues $status \g $title|length%percentile(0.25) -> q1 $title|length%percentile(0.5) -> q2 $title|length%percentile(0.75) -> q3
```

```sql
SELECT
  "issues"."status",
  percentile_cont(0.25) WITHIN GROUP (ORDER BY CAST(char_length("issues"."title") AS DOUBLE PRECISION)) AS "q1",
  percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST(char_length("issues"."title") AS DOUBLE PRECISION)) AS "q2",
  percentile_cont(0.75) WITHIN GROUP (ORDER BY CAST(char_length("issues"."title") AS DOUBLE PRECISION)) AS "q3"
FROM "issues"
GROUP BY "issues"."status";
```

### Aggregate `percentile` in a pipeline's later stage

> The median length of issues' titles for each status, from a first stage

```qd
#issues $status $title|length -> len ~~~ $status \g $len%percentile(0.5)
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."status" AS "status",
      char_length("issues"."title") AS "len"
    FROM "issues"
  )
SELECT
  "pipe0"."status",
  percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("pipe0"."len" AS DOUBLE PRECISION))
FROM "pipe0"
GROUP BY "pipe0"."status";
```

### Aggregate `percentile` in a pipeline's later stage (DuckDB)

```toml options
dialect = "duckdb"
```

> The median length of issues' titles for each status, from a first stage, targeting DuckDB

```qd
#issues $status $title|length -> len ~~~ $status \g $len%percentile(0.5)
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."status" AS "status",
      char_length("issues"."title") AS "len"
    FROM "issues"
  )
SELECT
  "pipe0"."status",
  percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("pipe0"."len" AS DOUBLE PRECISION))
FROM "pipe0"
GROUP BY "pipe0"."status";
```

### Aggregate `percentile` as a sorting entry of `list`

> Each project's statuses, ordered by the median id of the issues with each

```qd
#projects $name $#issues.status%list(\\id%percentile(0.5))
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "issues"."project" AS "pk",
          "issues"."status" AS "value",
          percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("issues"."id" AS DOUBLE PRECISION)) AS "sort1"
        FROM "issues"
        GROUP BY "issues"."project", "issues"."status"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Aggregate `percentile` as a sorting entry of `list` (DuckDB)

```toml options
dialect = "duckdb"
```

> Each project's statuses, ordered by the median id of the issues with each, targeting DuckDB

```qd
#projects $name $#issues.status%list(\\id%percentile(0.5))
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "issues"."project" AS "pk",
          "issues"."status" AS "value",
          percentile_cont(0.5) WITHIN GROUP (ORDER BY CAST("issues"."id" AS DOUBLE PRECISION)) AS "sort1"
        FROM "issues"
        GROUP BY "issues"."project", "issues"."status"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### JSON `json_path`

> Users who chose the dark theme

```qd
#users preferences|json_path("theme"):="dark" $username
```

```sql
SELECT
  "users"."username"
FROM "users"
WHERE
  lower(("users"."preferences" ->> 'theme') COLLATE "C") = lower('dark' COLLATE "C");
```

### JSON `json_path` (DuckDB)

```toml options
dialect = "duckdb"
```

> Users who chose the dark theme, targeting DuckDB

```qd
#users preferences|json_path("theme"):="dark" $username
```

```sql
SELECT
  "users"."username"
FROM "users"
WHERE
  lower(strip_accents(json_extract_string("users"."preferences", '$."theme"'))) = lower(strip_accents('dark'));
```

### JSON `json_path` with positions

> The name of each user's first saved filter, and the label they watch that's listed last

```qd
#users $preferences|json_path("saved_filters" 0 "name") $preferences|json_path("watched_labels" (-1))
```

```sql
SELECT
  (CAST("users"."preferences" AS json) -> 'saved_filters' -> 0 ->> 'name'),
  (CAST("users"."preferences" AS json) -> 'watched_labels' ->> -1)
FROM "users";
```

### JSON `json_path` with positions (DuckDB)

```toml options
dialect = "duckdb"
```

> The name of each user's first saved filter, and the label they watch that's listed last, targeting DuckDB

```qd
#users $preferences|json_path("saved_filters" 0 "name") $preferences|json_path("watched_labels" (-1))
```

```sql
SELECT
  json_extract_string("users"."preferences", '$."saved_filters"[0]."name"'),
  json_extract_string("users"."preferences", '$."watched_labels"[#-1]')
FROM "users";
```

### JSON `json_path` with a key from a column

> Assignments whose user wants to be notified about issues in the issue's status

```qd
#assignments user.preferences|json_path("notify" issue.status):="true" $issue.title
```

```sql
SELECT
  "issues"."title"
FROM "assignments"
LEFT JOIN "users" ON
  "assignments"."user" = "users"."id"
LEFT JOIN "issues" ON
  "assignments"."issue" = "issues"."id"
WHERE
  lower(("users"."preferences" -> 'notify' ->> "issues"."status") COLLATE "C") = lower('true' COLLATE "C");
```

### JSON `json_path` with a key from a column (DuckDB)

```toml options
dialect = "duckdb"
```

> Assignments whose user wants to be notified about issues in the issue's status, targeting DuckDB

```qd
#assignments user.preferences|json_path("notify" issue.status):="true" $issue.title
```

```sql
SELECT
  "issues"."title"
FROM "assignments"
LEFT JOIN "users" ON
  "assignments"."user" = "users"."id"
LEFT JOIN "issues" ON
  "assignments"."issue" = "issues"."id"
WHERE
  lower(strip_accents(json_extract_string("users"."preferences", ('$."notify"."' || replace(replace("issues"."status", '\', '\\'), '"', '\"') || '"')))) = lower(strip_accents('true'));
```

### JSON `json_path` with a computed position

> A label that each user watches, chosen by their id

```qd
#users $preferences|json_path("watched_labels" id|mod(3)|minus(1))
```

```sql
SELECT
  (CAST("users"."preferences" AS json) -> 'watched_labels' ->> CAST("users"."id" % 3 - 1 AS INTEGER))
FROM "users";
```

### JSON `json_path` with a computed position (DuckDB)

```toml options
dialect = "duckdb"
```

> A label that each user watches, chosen by their id, targeting DuckDB

```qd
#users $preferences|json_path("watched_labels" id|mod(3)|minus(1))
```

```sql
SELECT
  json_extract_string("users"."preferences", ('$."watched_labels"[' || CASE
    WHEN CAST("users"."id" % 3 - 1 AS INTEGER) < 0 THEN '#'
    ELSE ''
  END || CAST(CAST("users"."id" % 3 - 1 AS INTEGER) AS VARCHAR) || ']'))
FROM "users";
```

### JSON `json_path` of text

> The first element of the array under the key "a" of the JSON in each issue's description

```qd
#issues $description|json_path("a" 0)
```

```sql
SELECT
  (CAST("issues"."description" AS json) -> 'a' ->> 0)
FROM "issues";
```

### JSON `json_path` of text (DuckDB)

```toml options
dialect = "duckdb"
```

> The first element of the array under the key "a" of the JSON in each issue's description, targeting DuckDB

```qd
#issues $description|json_path("a" 0)
```

```sql
SELECT
  json_extract_string("issues"."description", '$."a"[0]')
FROM "issues";
```

### JSON `json_path` of what `json_path` gives

> Whether each user wants to be notified about open issues

```qd
#users $preferences|json_path("notify")|json_path("open")
```

```sql
SELECT
  (CAST(("users"."preferences" ->> 'notify') AS json) ->> 'open')
FROM "users";
```

### JSON `json_path` of what `json_path` gives (DuckDB)

```toml options
dialect = "duckdb"
```

> Whether each user wants to be notified about open issues, targeting DuckDB

```qd
#users $preferences|json_path("notify")|json_path("open")
```

```sql
SELECT
  json_extract_string(json_extract_string("users"."preferences", '$."notify"'), '$."open"')
FROM "users";
```

### JSON `to_array`

> Users who watch the "bug" label

```qd
#users preferences|json_path("watched_labels")|to_array|some(@l => @l:=="bug") $username
```

```sql
SELECT
  "users"."username"
FROM "users"
WHERE
  (SELECT EXISTS (SELECT 1 FROM unnest("array") AS "l"("l") WHERE "l" = 'bug') FROM (SELECT (SELECT ARRAY(SELECT jsonb_array_elements_text("json")) FROM (SELECT CAST(("users"."preferences" ->> 'watched_labels') AS jsonb)) AS "json"("json") WHERE jsonb_typeof("json") = 'array')) AS "array"("array") WHERE "array" IS NOT NULL);
```

### JSON `to_array` (DuckDB)

```toml options
dialect = "duckdb"
```

> Users who watch the "bug" label, targeting DuckDB

```qd
#users preferences|json_path("watched_labels")|to_array|some(@l => @l:=="bug") $username
```

```sql
SELECT
  "users"."username"
FROM "users"
WHERE
  len(list_filter(from_json(json_extract_string("users"."preferences", '$."watched_labels"'), '["VARCHAR"]'), lambda "l": "l" = 'bug')) > 0;
```

### JSON `to_array` with `filter` and `map`

> The names of each user's saved filters whose query is about issues

```qd
#users $preferences|json_path("saved_filters")|to_array|filter(@f => @f|json_path("query"):~'^#issues')|map(@f => @f|json_path("name"))|join(", ")
```

```sql
SELECT
  array_to_string((SELECT ARRAY(SELECT (CAST("f_1" AS json) ->> 'name') FROM unnest("array_1") WITH ORDINALITY AS "f_1"("f_1", "i_1") ORDER BY "i_1") FROM (SELECT (SELECT ARRAY(SELECT "f" FROM unnest("array") WITH ORDINALITY AS "f"("f", "i") WHERE (CAST("f" AS json) ->> 'query') ~* '(?p)^#issues' ORDER BY "i") FROM (SELECT (SELECT ARRAY(SELECT jsonb_array_elements_text("json")) FROM (SELECT CAST(("users"."preferences" ->> 'saved_filters') AS jsonb)) AS "json"("json") WHERE jsonb_typeof("json") = 'array')) AS "array"("array") WHERE "array" IS NOT NULL)) AS "array_1"("array_1") WHERE "array_1" IS NOT NULL), ', ')
FROM "users";
```

### JSON `to_array` with `filter` and `map` (DuckDB)

```toml options
dialect = "duckdb"
```

> The names of each user's saved filters whose query is about issues, targeting DuckDB

```qd
#users $preferences|json_path("saved_filters")|to_array|filter(@f => @f|json_path("query"):~'^#issues')|map(@f => @f|json_path("name"))|join(", ")
```

```sql
SELECT
  array_to_string(list_filter(list_transform(list_filter(from_json(json_extract_string("users"."preferences", '$."saved_filters"'), '["VARCHAR"]'), lambda "f": regexp_matches(json_extract_string("f", '$."query"'), '^#issues', 'i')), lambda "f_1": json_extract_string("f_1", '$."name"')), lambda e: e IS NOT NULL), ', ')
FROM "users";
```

### JSON `to_json`

> Each user's notification settings, nested as JSON within an object

```qd
#users $@@json_object("user" username "notify" preferences|json_path("notify")|to_json)
```

```sql
SELECT
  jsonb_build_object('user', "users"."username", 'notify', CAST(("users"."preferences" ->> 'notify') AS jsonb))
FROM "users";
```

### JSON `to_json` (DuckDB)

```toml options
dialect = "duckdb"
```

> Each user's notification settings, nested as JSON within an object, targeting DuckDB

```qd
#users $@@json_object("user" username "notify" preferences|json_path("notify")|to_json)
```

```sql
SELECT
  json_object('user', "users"."username", 'notify', CAST(json_extract_string("users"."preferences", '$."notify"') AS JSON))
FROM "users";
```

### JSON `to_json` of JSON

> Each user's preferences, which are JSON already

```qd
#users $preferences|to_json
```

```sql
SELECT
  "users"."preferences"
FROM "users";
```

### JSON `to_json` of JSON (DuckDB)

```toml options
dialect = "duckdb"
```

> Each user's preferences, which are JSON already, targeting DuckDB

```qd
#users $preferences|to_json
```

```sql
SELECT
  "users"."preferences"
FROM "users";
```

### JSON `json_object`

> Each issue as a JSON object of its title, status, and creation time

```qd
#issues $@@json_object("title" title "status" status "created" created_at)
```

```sql
SELECT
  jsonb_build_object('title', "issues"."title", 'status', "issues"."status", 'created', "issues"."created_at")
FROM "issues";
```

### JSON `json_object` (DuckDB)

```toml options
dialect = "duckdb"
```

> Each issue as a JSON object of its title, status, and creation time, targeting DuckDB

```qd
#issues $@@json_object("title" title "status" status "created" created_at)
```

```sql
SELECT
  json_object('title', "issues"."title", 'status', "issues"."status", 'created', replace(CAST("issues"."created_at" AS VARCHAR), ' ', 'T'))
FROM "issues";
```

### JSON `json_object` with a key that isn't text

> Each issue as a JSON object of its title, keyed by its id

```qd
#issues $@@json_object(id title)
```

```sql
SELECT
  jsonb_build_object(CAST("issues"."id" AS TEXT), "issues"."title")
FROM "issues";
```

### JSON `json_object` with a key that isn't text (DuckDB)

```toml options
dialect = "duckdb"
```

> Each issue as a JSON object of its title, keyed by its id, targeting DuckDB

```qd
#issues $@@json_object(id title)
```

```sql
SELECT
  json_object(CAST("issues"."id" AS TEXT), "issues"."title")
FROM "issues";
```

### JSON `json_object` in an aggregated computed column

> Each issue with its comments as JSON objects, oldest first

```qd
#comments.summary = @@json_object("by" user.username "body" body)
#issues $title $#comments.summary%list_all(\\created_at)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg(jsonb_build_object('by', "users"."username", 'body', "comments"."body") ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### JSON `json_object` in an aggregated computed column (DuckDB)

```toml options
dialect = "duckdb"
```

> Each issue with its comments as JSON objects, oldest first, targeting DuckDB

```qd
#comments.summary = @@json_object("by" user.username "body" body)
#issues $title $#comments.summary%list_all(\\created_at)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg(json_object('by', "users"."username", 'body', "comments"."body") ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."title",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### JSON `json_array`

> Each user's username, preferences, birth date, and email addresses, as a JSON array

```qd
#users $@@json_array(username preferences birth_date last_seen @@array(email))
```

```sql
SELECT
  jsonb_build_array("users"."username", "users"."preferences", "users"."birth_date", "users"."last_seen", ARRAY["users"."email"])
FROM "users";
```

### JSON `json_array` (DuckDB)

```toml options
dialect = "duckdb"
```

> Each user's username, preferences, birth date, and email addresses, as a JSON array, targeting DuckDB

```qd
#users $@@json_array(username preferences birth_date last_seen @@array(email))
```

```sql
SELECT
  json_array("users"."username", "users"."preferences", "users"."birth_date", replace(CAST("users"."last_seen" AS VARCHAR), ' ', 'T'), ["users"."email"])
FROM "users";
```

### JSON `json_array` with no values

> An empty JSON array for each user

```qd
#users $@@json_array()
```

```sql
SELECT
  jsonb_build_array()
FROM "users";
```

### JSON `json_array` with no values (DuckDB)

```toml options
dialect = "duckdb"
```

> An empty JSON array for each user, targeting DuckDB

```qd
#users $@@json_array()
```

```sql
SELECT
  json_array()
FROM "users";
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
  "users"."last_seen",
  "users"."preferences"
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
Its `queryAnnotation` is always compared, and is expected to be `null` unless the
block says otherwise.

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

## Query annotations

### Query annotation

> Show all issues, associating `{"foo": 100}` with the query. The annotation doesn't affect the SQL.

```qd
#issues \\\{"foo": 100}
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
  "queryAnnotation": { "foo": 100 }
}
```

### Query annotation alongside column annotations

> The query annotation comes last, after the result columns and their annotations. It is written in
> the same JSON variant as column annotations.

```qd
#issues status:open
\\created_at \d
$title @{width:400}
$created_at
\\\{
  view:table
  pageSize:50
  highlight:{column:title color:'#fbc9ff'}
  groups:[open closed]
  editable:@false
  note:@null
}
```

```sql
SELECT
  "issues"."title",
  "issues"."created_at"
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
ORDER BY
  "issues"."created_at" DESC NULLS LAST;
```

```json
{
  "columnAnnotations": [{ "width": 400 }, null],
  "queryAnnotation": {
    "view": "table",
    "pageSize": 50,
    "highlight": { "column": "title", "color": "#fbc9ff" },
    "groups": ["open", "closed"],
    "editable": false,
    "note": null
  }
}
```

### Query annotation after a pipeline

> The query annotation belongs to the whole query, so it follows the last stage of a pipeline.

```qd
#issues $author \g $%count -> n ~~~ n:>5 $author \\\{chart:bar}
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."author" AS "author",
      count(*) AS "n"
    FROM "issues"
    GROUP BY "issues"."author"
  )
SELECT
  "pipe0"."author"
FROM "pipe0"
WHERE
  "pipe0"."n" > 5;
```

```json
{
  "queryAnnotation": { "chart": "bar" }
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
  "users"."preferences",
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

## Aggregating computed values

### Aggregating a computed value across a to-many path

> The average length of each user's comments. The functions piped between the column and the
> aggregate function are applied to each comment's `body`, and the results are averaged, just as if
> `body|length` were a computed column on `comments`.

```qd
#users $username $#comments.body|length%avg
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk",
      avg(char_length("comments"."body")) AS "v1"
    FROM "comments"
    GROUP BY "comments"."user"
  )
SELECT
  "users"."username",
  "cte0"."v1"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Counting distinct computed values

> The number of distinct years in which each project had issues created

```qd
#projects $name $#issues.created_at|year%distinct
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(DISTINCT CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER)) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Aggregating a computed value through a to-one link

> The longest username among the authors of each project's issues. The value is computed from the
> column of the table that hosts it (`users`), which the CTE joins.

```qd
#projects $name $#issues.author.username|length%max
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      max(char_length("users"."username")) AS "v1"
    FROM "issues"
    JOIN "users" ON
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

### Aggregating a computed value in a condition

> Projects whose issues were created across more than two years

```qd
#projects #issues.created_at|year%distinct:>2 $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(DISTINCT CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER)) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  COALESCE("cte0"."v1", 0) > 2;
```

### Aggregating a computed value over filtered related records

> The average length of each user's comments since 2024

```qd
#users $username $#comments{created_at:>@2024-01-01}.body|length%avg
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk",
      avg(char_length("comments"."body")) AS "v1"
    FROM "comments"
    WHERE
      "comments"."created_at" > DATE '2024-01-01'
    GROUP BY "comments"."user"
  )
SELECT
  "users"."username",
  "cte0"."v1"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Aggregating a computed value whose function takes a column

> The most days that any of each project's issues was given before it's due. A column that a
> function takes as an argument (`created_at`) is a column of the record being aggregated, as in
> a computed column's definition.

```qd
#projects $name $#issues.due_date|minus(created_at)|days%max
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      max(EXTRACT(epoch FROM "issues"."due_date" - "issues"."created_at") / 86400) AS "v1"
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

### Listing computed values sorted by themselves

> The distinct lengths of each user's comments, longest first. `\s` sorts by the computed value.

```qd
#users $username $#comments.body|length%list(\sd)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk",
      array_agg(DISTINCT char_length("comments"."body") ORDER BY char_length("comments"."body") DESC NULLS LAST) AS "v1"
    FROM "comments"
    GROUP BY "comments"."user"
  )
SELECT
  "users"."username",
  "cte0"."v1"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Aggregating a computed value in a grouped query

> The longest issue title in each status

```qd
#issues $status \g $title|length%max
```

```sql
SELECT
  "issues"."status",
  max(char_length("issues"."title"))
FROM "issues"
GROUP BY "issues"."status";
```

### Aggregating a computed value through a to-one link in a grouped query

> The longest username among the authors of the issues in each status

```qd
#issues $status \g $author.username|length%max
```

```sql
SELECT
  "issues"."status",
  max(char_length("users"."username"))
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
GROUP BY "issues"."status";
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
      "comments"."body" ~* '(?p)work[ -]?around'
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

### Custom comparison called with a flagged regex operator

> The flags travel with the operator substituted into the body, and a literal argument still
> absorbs the regex option.

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues comment:\c~"work[ -]?around" $title
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      "comments"."body" ~ '(?p)work[ -]?around'
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

## Unions

Queries separated by `+++` are combined with an SQL `UNION ALL`, which keeps duplicate rows. The union's columns are named by the last query, so the first query (whose columns name an SQL union's) takes those names as aliases. A union is performed before any pipeline stage that follows it.

### Union of two queries

> Issues and projects, listed together by id and name. The second query's aliases name the union's columns.

```qd
#issues $id $title
+++
#projects $id $name -> title
```

```sql
SELECT
  "issues"."id",
  "issues"."title"
FROM "issues"
UNION ALL
SELECT
  "projects"."id",
  "projects"."name" AS "title"
FROM "projects";
```

### Union of three queries followed by a pipeline

> All the issues related to issue 1234, along with the way in which they are related, sorted by creation date.
> The union is performed before the pipeline stage, which reads the union's result.

```qd
#issues duplicate_of:1234
$id
$title
$created_at
$"Duplicate"
+++
#blocks blocker:1234
$blocking.id
$blocking.title
$blocking.created_at
$"Dependent"
+++
#blocks blocking:1234
$blocker.id         -> id
$blocker.title      -> title
$blocker.created_at -> created_at
$"Dependency"       -> relationship
~~~
$*(created_at \s)
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title",
      "issues"."created_at" AS "created_at",
      'Duplicate' AS "relationship"
    FROM "issues"
    WHERE
      "issues"."duplicate_of" = 1234
    UNION ALL
    SELECT
      "blocking"."id",
      "blocking"."title",
      "blocking"."created_at",
      'Dependent'
    FROM "blocks"
    LEFT JOIN "issues" AS "blocking" ON
      "blocks"."blocking" = "blocking"."id"
    WHERE
      "blocks"."blocker" = 1234
    UNION ALL
    SELECT
      "blocker"."id" AS "id",
      "blocker"."title" AS "title",
      "blocker"."created_at" AS "created_at",
      'Dependency' AS "relationship"
    FROM "blocks"
    LEFT JOIN "issues" AS "blocker" ON
      "blocks"."blocker" = "blocker"."id"
    WHERE
      "blocks"."blocking" = 1234
  )
SELECT
  "pipe0"."id",
  "pipe0"."title",
  "pipe0"."created_at",
  "pipe0"."relationship"
FROM "pipe0"
ORDER BY
  "pipe0"."created_at" ASC NULLS LAST;
```

### Column names come from the last query

> Where the last query doesn't name a column (here, the computed second column), the first query's name for it applies.

```qd
#issues $id -> issue $title|length -> label
+++
#projects $id -> project $name|length
```

```sql
SELECT
  "issues"."id" AS "project",
  char_length("issues"."title") AS "label"
FROM "issues"
UNION ALL
SELECT
  "projects"."id" AS "project",
  char_length("projects"."name")
FROM "projects";
```

### Column names come from the last query, before a pipeline

> Projects with more than ten issues, or more than ten comments on their issues. The last query leaves its count unnamed, so the first query's name for it, `n`, is the one the next stage reads.

```qd
#issues $project \g $%count -> n
+++
#comments $issue.project \g $%count
~~~
n:>10 $*
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."project" AS "project",
      count(*) AS "n"
    FROM "issues"
    GROUP BY "issues"."project"
    UNION ALL
    SELECT
      "issues"."project",
      count(*)
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    GROUP BY "issues"."project"
  )
SELECT
  "pipe0"."project",
  "pipe0"."n"
FROM "pipe0"
WHERE
  "pipe0"."n" > 10;
```

### Union of queries with their own CTEs

> Issues with comments and projects without issues. The second query's CTE is scoped to it with parentheses, so it may share a name with the first query's.

```qd
#issues ++#comments $id $title
+++
#projects --#issues $id $name
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
  "issues"."title" AS "name"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL
UNION ALL
(
  WITH
    "cte0" AS (
      SELECT
        "issues"."project" AS "pk"
      FROM "issues"
      GROUP BY "issues"."project"
    )
  SELECT
    "projects"."id",
    "projects"."name"
  FROM "projects"
  LEFT JOIN "cte0" ON
    "projects"."id" = "cte0"."pk"
  WHERE
    "cte0"."pk" IS NULL
);
```

### Union of queries with their own CTEs (DuckDB)

```toml options
dialect = "duckdb"
```

```qd
#issues ++#comments $id $title
+++
#projects --#issues $id $name
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
  "issues"."title" AS "name"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL
UNION ALL
(
  WITH
    "cte0" AS (
      SELECT
        "issues"."project" AS "pk"
      FROM "issues"
      GROUP BY "issues"."project"
    )
  SELECT
    "projects"."id",
    "projects"."name"
  FROM "projects"
  LEFT JOIN "cte0" ON
    "projects"."id" = "cte0"."pk"
  WHERE
    "cte0"."pk" IS NULL
);
```

### Union with every column of the base tables

> With no result columns, each query emits every column of its base table.

```qd
#issues duplicate_of:@null
+++
#issues !duplicate_of:@null
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
  "issues"."duplicate_of" IS NULL
UNION ALL
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
  NOT "issues"."duplicate_of" IS NULL;
```

### Union with grouping

> The number of issues each user has written, alongside the number of comments each has written. Each query of a union may group its own rows.

```qd
#issues $author -> user \g $%count -> n
+++
#comments $user \g $%count
```

```sql
SELECT
  "issues"."author" AS "user",
  count(*) AS "n"
FROM "issues"
GROUP BY "issues"."author"
UNION ALL
SELECT
  "comments"."user",
  count(*)
FROM "comments"
GROUP BY "comments"."user";
```

### Union in a user-defined table

> A user-defined table can hold a union, which is then filtered like any other table.

```qd
#people = #(
  #users $id $username -> name
  +++
  #projects $id $name
)
#people name:~'^a'
```

```sql
WITH
  "people" AS (
    SELECT
      "users"."id" AS "id",
      "users"."username" AS "name"
    FROM "users"
    UNION ALL
    SELECT
      "projects"."id",
      "projects"."name"
    FROM "projects"
  )
SELECT
  "people"."id",
  "people"."name"
FROM "people"
WHERE
  "people"."name" ~* '(?p)^a';
```

### Union with a limit

```toml options
limit = 5
```

> A limit applies to the union as a whole.

```qd
#issues $title
+++
#projects $name
```

```sql
SELECT
  "issues"."title" AS "name"
FROM "issues"
UNION ALL
SELECT
  "projects"."name"
FROM "projects"
LIMIT 5;
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

## Limit and offset

Limits and offsets are not part of the Querydown language. They're supplied as options to the compiler (set here via `limit` and `offset` in a case's options block), which applies them to the outermost query.

### Limit

```toml options
limit = 10
```

> The ten most recently-created issues

```qd
#issues \\created_at \d $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."created_at" DESC NULLS LAST
LIMIT 10;
```

### Offset

```toml options
offset = 20
```

> Every issue after the first twenty

```qd
#issues \\id $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."id" ASC NULLS LAST
OFFSET 20;
```

### Limit and offset

```toml options
limit = 10
offset = 20
```

> The third page of ten issues

```qd
#issues \\id $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."id" ASC NULLS LAST
LIMIT 10
OFFSET 20;
```

### Zero limit

```toml options
limit = 0
```

> A limit of zero returns no rows, while still producing the result's columns.

```qd
#issues $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
LIMIT 0;
```

### Limit and offset (DuckDB)

```toml options
dialect = "duckdb"
limit = 10
offset = 20
```

```qd
#issues \\id $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."id" ASC NULLS LAST
LIMIT 10
OFFSET 20;
```

### Limit and offset with grouping

```toml options
limit = 5
offset = 5
```

> The limit applies to the grouped rows, after the `GROUP BY`.

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
  count(*) DESC NULLS LAST
LIMIT 5
OFFSET 5;
```

### Limit and offset with related records

```toml options
limit = 10
offset = 30
```

> The limit applies to the base table's rows, never to the related records aggregated into them.

```qd
#issues \\id $title $#comments
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
  "issues"."id" = "cte0"."pk"
ORDER BY
  "issues"."id" ASC NULLS LAST
LIMIT 10
OFFSET 30;
```

### Limit and offset on a pipeline

```toml options
limit = 10
offset = 20
```

> Only the final stage is limited; earlier stages are materialized in full.

```qd
#issues $project \g $%count -> issue_count
~~~
issue_count:>=2 \\issue_count \d
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
  "pipe0"."issue_count" >= 2
ORDER BY
  "pipe0"."issue_count" DESC NULLS LAST
LIMIT 10
OFFSET 20;
```

### Limit and offset with a user-defined table and a subquery

```toml options
limit = 10
offset = 20
```

> Neither the user-defined table's CTE nor the scalar subquery is limited.

```qd
@latest = #( #comments $created_at%max )
#recent = #( #issues $id $created_at )
#recent created_at:<=@latest $id
```

```sql
WITH
  "recent" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."created_at" AS "created_at"
    FROM "issues"
  )
SELECT
  "recent"."id"
FROM "recent"
WHERE
  "recent"."created_at" <= (
    SELECT
      max("comments"."created_at")
    FROM "comments"
  )
LIMIT 10
OFFSET 20;
```

## Tables in other schemas

### Base table in another schema, with a link to the current schema

> Unpaid invoices, oldest first, with their clients

```qd
#/billing/invoices is_paid:@false \\issued_on $client.name $amount
```

```sql
SELECT
  "clients"."name",
  "invoices"."amount"
FROM "billing"."invoices" AS "invoices"
LEFT JOIN "clients" ON
  "invoices"."client" = "clients"."id"
WHERE
  "invoices"."is_paid" = FALSE
ORDER BY
  "invoices"."issued_on" ASC NULLS LAST;
```

### Aggregate of related records in another schema

> Each client's total of unpaid invoices

```qd
#clients $name $#/billing/invoices{is_paid:@false}.amount%sum -> unpaid
```

```sql
WITH
  "cte0" AS (
    SELECT
      "invoices"."client" AS "pk",
      sum("invoices"."amount") AS "v1"
    FROM "billing"."invoices" AS "invoices"
    WHERE
      "invoices"."is_paid" = FALSE
    GROUP BY "invoices"."client"
  )
SELECT
  "clients"."name",
  "cte0"."v1" AS "unpaid"
FROM "clients"
LEFT JOIN "cte0" ON
  "clients"."id" = "cte0"."pk";
```

### Related records in another schema, reached through the current schema

> Issues of clients with a large invoice

```qd
#issues ++#/billing/invoices{amount:>1000} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "invoices"."client" AS "pk"
    FROM "billing"."invoices" AS "invoices"
    WHERE
      "invoices"."amount" > 1000
    GROUP BY "invoices"."client"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id"
LEFT JOIN "clients" ON
  "products"."client" = "clients"."id"
LEFT JOIN "cte0" ON
  "clients"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Window condition on a table in another schema (DuckDB)

```toml options
dialect = "duckdb"
```

> Each client's latest invoice

```qd
#/billing/invoices %%(client\p issued_on\sd)%row_number:1 $client.name $amount
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "invoices"."id",
      "invoices"."client",
      "invoices"."issued_on",
      "invoices"."amount",
      "invoices"."is_paid",
      row_number() OVER (PARTITION BY "invoices"."client" ORDER BY "invoices"."issued_on" DESC NULLS LAST) AS "qdwin0"
    FROM "billing"."invoices" AS "invoices"
  )
SELECT
  "clients"."name",
  "invoices"."amount"
FROM "qdwin_src" AS "invoices"
LEFT JOIN "clients" ON
  "invoices"."client" = "clients"."id"
WHERE
  "invoices"."qdwin0" = 1;
```

## Combinations: values and computations

These cases combine literals, arithmetic, functions, and case expressions with one another.

### Fractional and sub-day durations compared against dates

> Issues created within two and a half years, and due more than nine minutes from now

```qd
#issues created_at:<2.5y due_date:>9min $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(years => 2, secs => 15778800)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) > EXTRACT(epoch FROM make_interval(mins => 9));
```

### Duration with every unit as a function argument

> Issues created after a moment computed from a duration that uses every unit

```qd
#issues created_at:>@now|minus(2y6m3d8h9min34.89s) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."created_at" > NOW() - make_interval(years => 2, months => 6, days => 3, hours => 8, mins => 9, secs => 34.89);
```

### Comments between conditions and result columns

> Both comment styles, around a condition

```qd
#issues /* multi
line */ status:open // trailing
$id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE);
```

### Arithmetic precedence and parentheses

> Multiplication and division bind tighter than addition and subtraction, unless parenthesized

```qd
#issues $id + 2 * 3 - id / 4 $(id + 2) * 3 $id * (id + 1) $(id + 1) / 2
```

```sql
SELECT
  "issues"."id" + 2 * 3 - "issues"."id" / 4,
  ("issues"."id" + 2) * 3,
  "issues"."id" * ("issues"."id" + 1),
  ("issues"."id" + 1) / 2
FROM "issues";
```

### Arithmetic functions chained after arithmetic operators

> A piped function applies to the parenthesized expression before it, or to just the last operand

```qd
#issues $(id + 1)|times(2) $id + 1|times(2) $id|minus(1)|times(2) $id|plus(1)|divide(2) $id|times(id + 1)
```

```sql
SELECT
  ("issues"."id" + 1) * 2,
  "issues"."id" + 1 * 2,
  ("issues"."id" - 1) * 2,
  ("issues"."id" + 1) / 2,
  "issues"."id" * ("issues"."id" + 1)
FROM "issues";
```

### Literals of every kind as result columns

> A string, a number, each built-in constant, a duration, and a date

```qd
#issues $"x" -> k $1 -> one $@null -> n $@now -> t $@true -> yes $1w -> dur $@2020-01-01 -> d
```

```sql
SELECT
  'x' AS "k",
  1 AS "one",
  NULL AS "n",
  NOW() AS "t",
  TRUE AS "yes",
  make_interval(weeks => 1) AS "dur",
  DATE '2020-01-01' AS "d"
FROM "issues";
```

### Flagged strings with each remaining quote character

> Custom quotes keep backslashes and both kinds of quote literal

```qd
#issues title:=^/v\d+/ description:=^|say "can't"| status:=^#x# $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  lower("issues"."title" COLLATE "C") = lower('v\d+' COLLATE "C") AND
  lower("issues"."description" COLLATE "C") = lower('say "can''t"' COLLATE "C") AND
  lower("issues"."status" COLLATE "C") = lower('x' COLLATE "C");
```

### Format string with escaped braces, by doubling and by the `e` flag

> Literal braces alongside an interpolation and an escaped newline

```qd
#issues title:=^f"a{{b}}" description:=^ef"a\{b\}\n{id}" $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  lower("issues"."title" COLLATE "C") = lower('a{b}' COLLATE "C") AND
  lower("issues"."description" COLLATE "C") = lower(concat('a{b}
', "issues"."id") COLLATE "C");
```

### Format string interpolating a function pipeline and a count of related records

> Each issue's title in uppercase, with its number of comments

```qd
#issues $^f"{title|uppercase} ({#comments} comments)"
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
  concat(upper("issues"."title"), ' (', COALESCE("cte0"."v1", 0), ' comments)')
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Format string nested within a format string

> The inner string uses the other quote character

```qd
#issues $^f"{^f'{id}!'}?"
```

```sql
SELECT
  concat(concat("issues"."id", '!'), '?')
FROM "issues";
```

### Long function pipeline mixing date, math, and rounding functions

> The number of weeks (rounded down) in twice the number of whole days, plus one, between now and the due date

```qd
#issues $due_date|countdown|days|abs|ceil|plus(1)|times(2)|divide(7)|floor
```

```sql
SELECT
  FLOOR((CEIL(ABS(EXTRACT(epoch FROM "issues"."due_date" - NOW()) / 86400)) + 1) * 2 / 7)
FROM "issues";
```

### Text function pipeline with a piped argument

> The length of the trimmed, lowercased title joined to the uppercased status

```qd
#issues $title|trim|lowercase|concat(" ")|concat(status|uppercase)|length $title|if_null("")|length
```

```sql
SELECT
  char_length(concat(concat(lower(trim("issues"."title")), ' '), upper("issues"."status"))),
  char_length(COALESCE("issues"."title", ''))
FROM "issues";
```

### Clamping functions chained together

> A value kept within bounds, then run through several rounding functions

```qd
#issues $id|keep_above(0)|keep_below(10) $id|max(1)|min(id|plus(5))|abs|floor|ceil
```

```sql
SELECT
  LEAST(GREATEST("issues"."id", 0), 10),
  CEIL(FLOOR(ABS(LEAST(GREATEST("issues"."id", 1), "issues"."id" + 5))))
FROM "issues";
```

### Boolean functions chained with differing precedence

> `and` and `or` are applied left to right, so the SQL needs parentheses where its own precedence differs

```qd
#issues $(id:1)|and(id:2)|or(id:3) $(id:1)|or(id:2)|and(id:3) $(id:1)|and((id:2)|or(id:3)) $!((id:1)|or(id:2)) $(id:1)|or(id:2)|not
```

```sql
SELECT
  "issues"."id" = 1 AND
  "issues"."id" = 2 OR "issues"."id" = 3,
  ("issues"."id" = 1 OR "issues"."id" = 2) AND
  "issues"."id" = 3,
  "issues"."id" = 1 AND
  ("issues"."id" = 2 OR "issues"."id" = 3),
  ("issues"."id" = 1 OR "issues"."id" = 2) IS NOT TRUE,
  ("issues"."id" = 1 OR "issues"."id" = 2) IS NOT TRUE
FROM "issues";
```

### Boolean `xor` chained with `and`

> `xor` compiles to `<>`, whose operands need parentheses

```qd
#issues $(id:1)|xor(id:2)|and(id:3) $(id:1)|and(id:2)|xor(id:3)
```

```sql
SELECT
  ("issues"."id" = 1) <> ("issues"."id" = 2) AND
  "issues"."id" = 3,
  ("issues"."id" = 1 AND
  "issues"."id" = 2) <> ("issues"."id" = 3)
FROM "issues";
```

### Case expression within arithmetic

> A parenthesized case expression is an operand; unparenthesized, the fallback absorbs the arithmetic

```qd
#issues $ (? id:1 ~ 1 ~~ 2) * 3 $ ? id:1 ~ 1 ~~ 2 * 3 $(? id:1 ~ 1 ~~ 2)|plus(1)
```

```sql
SELECT
  CASE
    WHEN "issues"."id" = 1 THEN 1
    ELSE 2
  END * 3,
  CASE
    WHEN "issues"."id" = 1 THEN 1
    ELSE 2 * 3
  END,
  CASE
    WHEN "issues"."id" = 1 THEN 1
    ELSE 2
  END + 1
FROM "issues";
```

### Case expression nested in a case expression

> A case expression as a variant's value and as the fallback

```qd
#issues $ ? id:1 ~ ? status:open ~ "a" ~~ "b" ~~ ? id:2 ~ "c" ~~ "d"
```

```sql
SELECT
  CASE
    WHEN "issues"."id" = 1 THEN CASE
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 'a'
    ELSE 'b'
  END
    ELSE CASE
    WHEN "issues"."id" = 2 THEN 'c'
    ELSE 'd'
  END
  END
FROM "issues";
```

### Case expression testing counts of related records

> Categorize projects by how many issues they have

```qd
#projects $name $ ? #issues:0 ~ "empty" #issues{status:open}:>10 ~ "busy" ~~ "normal" -> load
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  CASE
    WHEN "cte0"."pk" IS NULL THEN 'empty'
    WHEN COALESCE("cte1"."v1", 0) > 10 THEN 'busy'
    ELSE 'normal'
  END AS "load"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Case expression as a condition and within a comparison

> A parenthesized case expression on the left of a comparison, and one standing alone as a condition

```qd
#issues (? status:open ~ 1 ~~ 0):1 [? id:1 ~ @true ~~ @false] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  CASE
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 1
    ELSE 0
  END = 1 AND
  (CASE
    WHEN "issues"."id" = 1 THEN TRUE
    ELSE FALSE
  END);
```

### Case expression whose values differ in text type inference

> Matching against a case expression, a trimmed column, and a concatenation is still a text search

```qd
#issues (? id:1 ~ title ~~ description):foo title|trim:foo title|concat(status):foo $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  COALESCE(strpos(lower(CASE
    WHEN "issues"."id" = 1 THEN "issues"."title"
    ELSE "issues"."description"
  END COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower(trim("issues"."title") COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower(concat("issues"."title", "issues"."status") COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE);
```

### Anonymous function piped into an anonymous function

> Each function binds its own `@x`, including one nested within another's body

```qd
#issues $id|(@x => @x + 1)|(@x => @x * 2) $id|(@x => @x|(@x => @x + 1) * @x)
```

```sql
SELECT
  ("issues"."id" + 1) * 2,
  ("issues"."id" + 1) * "issues"."id"
FROM "issues";
```

### Anonymous function nested in a two-parameter anonymous function

> The inner function's body uses the outer function's second parameter

```qd
#issues $id|(@a @b => @a|(@c => @c * @b))(2)
```

```sql
SELECT
  "issues"."id" * 2
FROM "issues";
```

### Anonymous function within arithmetic

> Pipe binds tighter than arithmetic, so the function applies to the operand next to it

```qd
#issues $id|(@x => @x + 1) * 2 $2 * id|(@x => @x + 1)
```

```sql
SELECT
  ("issues"."id" + 1) * 2,
  2 * ("issues"."id" + 1)
FROM "issues";
```

### User-defined function using a constant, piped into an anonymous function

> The constant is inlined within the function body and at the call site

```qd
@k = 3
@@scale = @x => @x * @k
#issues $id|scale|(@v => @v + @k) $@@scale(@k)
```

```sql
SELECT
  "issues"."id" * 3 + 3,
  3 * 3
FROM "issues";
```

### User-defined functions chained together

> Both functions are inlined, with parentheses where precedence requires them

```qd
@@double = @x => @x * 2
@@inc = @x => @x + 1
#issues $id|inc|double $(id - 1)|inc|double $id|double|inc $@@double(id + 1)
```

```sql
SELECT
  ("issues"."id" + 1) * 2,
  ("issues"."id" - 1 + 1) * 2,
  "issues"."id" * 2 + 1,
  ("issues"."id" + 1) * 2
FROM "issues";
```

### User-defined function whose body is a range comparison

> The bounds are bound to parameters

```qd
@@between = @v @lo @hi => @v:@lo..@hi
#issues id|between(1 5) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."id" >= 1 AND
  "issues"."id" <= 5;
```

### User-defined function whose body is an expanded comparison

> The bare words in the function body are string literals, since they're on the right-hand side

```qd
@@any_of = @v => @v:[open reopened]
#issues status|any_of $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE));
```

### User-defined function whose body is an OR set of null and empty checks

> Applied to two columns as bare conditions

```qd
@@blank = @s => [@s:@null @s:""]
#issues title|blank description|blank $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ("issues"."title" IS NULL OR "issues"."title" = '') AND
  ("issues"."description" IS NULL OR "issues"."description" = '');
```

### User-defined function containing a case expression, used for grouping and sorting

> Count issues by a label derived from their status, in label order

```qd
@@label = @s => ? @s:open ~ "Open" @s:[closed done] ~ "Closed" ~~ @s|uppercase
#issues \\status|label $status|label \g $%count
```

```sql
SELECT
  CASE
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 'Open'
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('done' COLLATE "C")) > 0, FALSE) THEN 'Closed'
    ELSE upper("issues"."status")
  END,
  count(*)
FROM "issues"
GROUP BY CASE
  WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 'Open'
  WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('done' COLLATE "C")) > 0, FALSE) THEN 'Closed'
  ELSE upper("issues"."status")
END
ORDER BY
  CASE
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 'Open'
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('done' COLLATE "C")) > 0, FALSE) THEN 'Closed'
    ELSE upper("issues"."status")
  END ASC NULLS LAST;
```

### Constants defined in terms of other constants within arithmetic

> Each constant is inlined, in parentheses where precedence requires them

```qd
@a = 1 + 2
#issues $id * @a $@a * @a
```

```sql
SELECT
  "issues"."id" * (1 + 2),
  (1 + 2) * (1 + 2)
FROM "issues";
```

### String constant used with several operators and in a format string

> The same constant as a search term, an exact match, a regex, and an interpolated value

```qd
@s = "open"
#issues status:@s status:=@s title:~@s $^f"is {@s}" $id
```

```sql
SELECT
  concat('is ', 'open'),
  "issues"."id"
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) AND
  lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") AND
  "issues"."title" ~* '(?p)open';
```

### Subtraction and division with a parenthesized right operand

> A right operand that binds as tightly as its operator keeps its parentheses: `id - (id + 1)` isn't `id - id + 1`

```qd
#issues $id - (id + 1) $id / (id * 2) $id - (id - 1) $id / (id / 2)
```

```sql
SELECT
  "issues"."id" - ("issues"."id" + 1),
  "issues"."id" / ("issues"."id" * 2),
  "issues"."id" - ("issues"."id" - 1),
  "issues"."id" / ("issues"."id" / 2)
FROM "issues";
```

### Subtraction, division, and modulo functions with a computed argument

> The `minus`, `divide`, and `mod` functions parenthesize a computed second argument, just as the operators do

```qd
#issues $id|minus(id - 1) $id|divide(id|times(2)) $id|mod(id|times(2))
```

```sql
SELECT
  "issues"."id" - ("issues"."id" - 1),
  "issues"."id" / ("issues"."id" * 2),
  "issues"."id" % ("issues"."id" * 2)
FROM "issues";
```

### Addition and multiplication with a parenthesized right operand

> The parentheses are kept here too. They're redundant for numbers, but not always: SQL can add two durations to a date, but not a date to a date

```qd
#issues $id + (id + 1) $id * (id * 2) $created_at + (1w + 1d)
```

```sql
SELECT
  "issues"."id" + ("issues"."id" + 1),
  "issues"."id" * ("issues"."id" * 2),
  "issues"."created_at" + (make_interval(weeks => 1) + make_interval(days => 1))
FROM "issues";
```

### Hash function within arithmetic

> `unit_hash` compiles to a division, so it's parenthesized as the right operand of another

```qd
#users $1 / username|unit_hash $username|unit_hash * 2 $username|unit_hash|times(100)|floor
```

```sql
SELECT
  1 / ((CAST(hashtext("users"."username") AS DOUBLE PRECISION) + 2147483648) / 4294967295.0),
  (CAST(hashtext("users"."username") AS DOUBLE PRECISION) + 2147483648) / 4294967295.0 * 2,
  FLOOR((CAST(hashtext("users"."username") AS DOUBLE PRECISION) + 2147483648) / 4294967295.0 * 100)
FROM "users";
```

### Bare words at depth within a function on the right-hand side

> A pipe, a function call, and an anonymous function all receive string literals

```qd
#issues title:foo|concat(bar) title:@@concat(foo bar) title:foo|(@x => @x|concat(bar)) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower(concat('foo', 'bar') COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower(concat('foo', 'bar') COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower(concat('foo', 'bar') COLLATE "C")) > 0, FALSE);
```

### Bare word in parentheses and in an expansion entry in parentheses

> Parentheses don't change a bare word's meaning

```qd
#issues title:(foo) title:=[foo (bar)] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  (lower("issues"."title" COLLATE "C") = lower('foo' COLLATE "C") OR lower("issues"."title" COLLATE "C") = lower('bar' COLLATE "C"));
```

### Comparison nested on the right-hand side of a comparison

> The inner `name` is a column reference, and the inner `foo` a string literal

```qd
#projects is_active:(name:foo) $id
```

```sql
SELECT
  "projects"."id"
FROM "projects"
WHERE
  "projects"."is_active" = COALESCE(strpos(lower("projects"."name" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE);
```

### Multi-part path on the right-hand side

> Issues whose description mentions their author's username

```qd
#issues description:author.username $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower("users"."username" COLLATE "C")) > 0, FALSE);
```

### Backtick-quoted column as a bare condition

> Active projects

```qd
#projects `is_active` $id
```

```sql
SELECT
  "projects"."id"
FROM "projects"
WHERE
  "projects"."is_active";
```

### Negated right-hand expansion

> Issues whose status is neither open nor reopened

```qd
#issues !status:[open reopened] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  NOT (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE));
```

### Negated dual expansion

> Negation applies to the whole expanded comparison

```qd
#issues !{title description}:~[foo bar] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (("issues"."title" ~* '(?p)foo' OR "issues"."title" ~* '(?p)bar') AND
  ("issues"."description" ~* '(?p)foo' OR "issues"."description" ~* '(?p)bar')) IS NOT TRUE;
```

### Dual expansion with OR on the left and AND on the right

> Issues whose title contains both words, or whose description does

```qd
#issues [title description]:{foo bar} $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE));
```

### Left expansion of a null check

> Issues missing both a title and a description

```qd
#issues {title description}:@null $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."title" IS NULL AND
  "issues"."description" IS NULL;
```

### Left expansion against a range

> Issues whose id or project id is between 1 and 5

```qd
#issues [id project]:1..5 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ("issues"."id" >= 1 AND
  "issues"."id" <= 5 OR "issues"."project" >= 1 AND
  "issues"."project" <= 5);
```

### Negated range

> Issues whose id is outside of a range

```qd
#issues !id:1..5 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ("issues"."id" >= 1 AND
  "issues"."id" <= 5) IS NOT TRUE;
```

### Range with computed bounds and a computed left-hand side

> Arithmetic bounds need parentheses

```qd
#issues id:(1+1)..(2*3) id|mod(2):0..1 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."id" >= 1 + 1 AND
  "issues"."id" <= 2 * 3 AND
  "issues"."id" % 2 >= 0 AND
  "issues"."id" % 2 <= 1;
```

### Range of strings

> Issues whose title sorts between two strings

```qd
#issues title:"a".."m" $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."title" >= 'a' AND
  "issues"."title" <= 'm';
```

### Range against a count of related records

> Issues with between one and five comments

```qd
#issues #comments:1..5 $id
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
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  COALESCE("cte0"."v1", 0) >= 1 AND
  COALESCE("cte0"."v1", 0) <= 5;
```

### Range of durations

> Issues created between six months and one year away from now

```qd
#issues created_at:6m..1y $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) >= EXTRACT(epoch FROM make_interval(months => 6)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) <= EXTRACT(epoch FROM make_interval(years => 1));
```

### Range between a computed date and now

> Issues created during the past year

```qd
#issues created_at:(@now|minus(1y))..@now $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."created_at" >= NOW() - make_interval(years => 1) AND
  "issues"."created_at" <= NOW();
```

### Range as a result column and as a case test

> A range comparison yields a boolean wherever an expression is allowed

```qd
#issues $id:1..5 -> in_range $ ? id:1..5 ~ "low" ~~ "high" -> bucket
```

```sql
SELECT
  "issues"."id" >= 1 AND
  "issues"."id" <= 5 AS "in_range",
  CASE
    WHEN "issues"."id" >= 1 AND
  "issues"."id" <= 5 THEN 'low'
    ELSE 'high'
  END AS "bucket"
FROM "issues";
```

### Date compared with a duration through a left expansion

> Issues created or due within a week of now

```qd
#issues [created_at due_date]:<1w $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) OR ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM make_interval(weeks => 1)));
```

### Date compared with a duration through a right expansion

> Each entry of the expansion is a duration

```qd
#issues created_at:<[1w 1m] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) OR ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(months => 1)));
```

### Date compared with a duration through a left expansion that includes an aggregate

> Issues created, or last commented on, within a week of now

```qd
#issues [created_at #comments.created_at%max]:<1w $id
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
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  (ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) OR ABS(EXTRACT(epoch FROM NOW() - "cte0"."v1")) < EXTRACT(epoch FROM make_interval(weeks => 1)));
```

### Negated date compared with a duration

> Issues not created within a week of now, including those with no creation date

```qd
#issues !created_at:<1w $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1))) IS NOT TRUE;
```

### Date compared with a duration using each operator

> The equality operators also compare the distance from now

```qd
#issues created_at:<=1w created_at:>=1d created_at:1w created_at:=1w $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) <= EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) >= EXTRACT(epoch FROM make_interval(days => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) = EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) = EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Date compared with a duration in parentheses

> A parenthesized duration literal is still a duration literal

```qd
#issues created_at:<(1w) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Date compared with a duration through to-one paths, scopes, and filters

> The date may be reached through a related record

```qd
#comments issue.created_at:<1w issue{due_date:<1d} $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM make_interval(days => 1));
```

### Date compared with a duration as a result column and as a case test

> The comparison yields a boolean wherever an expression is allowed

```qd
#issues $created_at:<1w -> fresh $ ? due_date:<1d ~ "soon" ~~ "later"
```

```sql
SELECT
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AS "fresh",
  CASE
    WHEN ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM make_interval(days => 1)) THEN 'soon'
    ELSE 'later'
  END
FROM "issues";
```

### Aggregated date compared with a duration in a grouped query

> For each status, whether its most recent issue was created within a week of now

```qd
#issues $status \g $created_at%max:<1w
```

```sql
SELECT
  "issues"."status",
  ABS(EXTRACT(epoch FROM NOW() - max("issues"."created_at"))) < EXTRACT(epoch FROM make_interval(weeks => 1))
FROM "issues"
GROUP BY "issues"."status";
```

### Date compared with a duration within function bodies and a computed column

> The date's type is inferred through a function parameter and through a computed column

```qd
@@recent = @d => @d:<1w
#issues.opened = created_at
#issues created_at|recent due_date|(@d => @d:<1w) opened:<1w $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Date compared with a duration through functions of several dates

> The type of `max` over two dates is still a date

```qd
#issues @@max(created_at due_date):<1w $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - GREATEST("issues"."created_at", "issues"."due_date"))) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Date and zoned timestamp columns compared with durations

> A `date` column and a `timestamptz` column each work on the left

```qd
#users last_seen:<1h birth_date:>18y $id
```

```sql
SELECT
  "users"."id"
FROM "users"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "users"."last_seen")) < EXTRACT(epoch FROM make_interval(hours => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "users"."birth_date")) > EXTRACT(epoch FROM make_interval(years => 18));
```

### Durations compared with durations

> An age is a duration, so it's compared with a duration literal directly

```qd
#users birth_date|age:>18y $id
```

```sql
SELECT
  "users"."id"
FROM "users"
WHERE
  NOW() - "users"."birth_date" > make_interval(years => 18);
```

### Date arithmetic compared with a date

> A date plus a duration is a date, compared here with `@now` rather than with a duration

```qd
#issues created_at + 1w:<@now $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."created_at" + make_interval(weeks => 1) < NOW();
```

### Regex flags with a right expansion

> The flags apply to each expanded comparison

```qd
#issues title:\c~[foo bar] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ("issues"."title" ~ '(?p)foo' OR "issues"."title" ~ '(?p)bar');
```

### Regex flags with a left expansion

> The flags apply to each expanded comparison

```qd
#issues [title description]:\ms~'^x' $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ("issues"."title" ~* '(?w)^x' OR "issues"."description" ~* '(?w)^x');
```

### Negated OR set of search terms

> Issues that mention neither term

```qd
#issues ![foo bar] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE));
```

### Search terms in nested condition sets

> A comma inside an AND set, and a negated term inside an AND set inside an OR set

```qd
#issues {foo,bar baz} [foo {bar !baz}] $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)) AND
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE)) AND
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)) AND
  NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE)));
```

### Comma mixing a search term, a comparison, and a negated search term

> Any one of the three operands may hold

```qd
#issues foo,status:open,!bar $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)));
```

### Comma joining quoted and formatted search terms

> Each operand is a default text search

```qd
#issues "two words",'x',^f"{id}" $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('two words' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('two words' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('two words' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower(concat("issues"."id") COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower(concat("issues"."id") COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower(concat("issues"."id") COLLATE "C")) > 0, FALSE));
```

### Comma binding tighter than the implicit AND

> The comma joins only the two comparisons next to it

```qd
#issues id:1,id:2 status:open $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ("issues"."id" = 1 OR "issues"."id" = 2) AND
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE);
```

### Negation binding tighter than the comma

> Only the first operand is negated

```qd
#issues !id:1,id:2 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (("issues"."id" = 1) IS NOT TRUE OR "issues"."id" = 2);
```

### Has-some and has-none joined by a comma

> Issues that have a comment or lack a label

```qd
#issues ++#comments,--#labels $id
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
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  ("cte0"."pk" IS NOT NULL OR "cte1"."pk" IS NULL);
```

### Negated has-some and negated has-none

> Negating `++` is the same as `--`, and vice versa

```qd
#issues !++#comments !--#comments{body:x} $id
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
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  NOT "cte0"."pk" IS NOT NULL AND
  NOT "cte1"."pk" IS NULL;
```

### Has-some within nested condition sets alongside a negated to-one comparison

> Open issues, or commented issues not authored by bob

```qd
#issues [status:open {++#comments !author.username:=bob}] $id
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
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR "cte0"."pk" IS NOT NULL AND
  (lower("users"."username" COLLATE "C") = lower('bob' COLLATE "C")) IS NOT TRUE);
```

### Comparisons and has conditions as result columns

> Each yields a boolean column

```qd
#issues $title:foo -> has_foo $!title:foo -> lacks_foo $++#comments -> commented $--#labels -> unlabeled
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
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    GROUP BY "issue_labels"."issue"
  )
SELECT
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AS "has_foo",
  NOT COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AS "lacks_foo",
  "cte0"."pk" IS NOT NULL AS "commented",
  "cte1"."pk" IS NULL AS "unlabeled"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk";
```

### Expanded comparisons as result columns

> A right expansion and a left expansion, each yielding a boolean column

```qd
#issues $status:[open reopened] -> active $[title description]:~bug -> buggy
```

```sql
SELECT
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE) AS "active",
  "issues"."title" ~* '(?p)bug' OR "issues"."description" ~* '(?p)bug' AS "buggy"
FROM "issues";
```

### Counts of related records compared with each other

> Projects with as many open issues as closed ones

```qd
#projects #issues{status:open}:#issues{status:closed} $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
WHERE
  COALESCE("cte0"."v1", 0) = COALESCE("cte1"."v1", 0);
```

### Aggregates of related records compared with base columns

> Issues with more comments than assignments, whose first comment precedes the due date

```qd
#issues #comments:>#assignments #comments.created_at%min:<`due_date` $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "assignments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "assignments"
    GROUP BY "assignments"."issue"
  ),
  "cte2" AS (
    SELECT
      "comments"."issue" AS "pk",
      min("comments"."created_at") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "issues"."id" = "cte2"."pk"
WHERE
  COALESCE("cte0"."v1", 0) > COALESCE("cte1"."v1", 0) AND
  "cte2"."v1" < "issues"."due_date";
```

### Arithmetic over counts of related records as a condition and a sorted column

> Issues with more than five comments and assignments combined, most active first

```qd
#issues #comments + #assignments:>5 $#comments + #assignments -> activity \sd
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "assignments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "assignments"
    GROUP BY "assignments"."issue"
  )
SELECT
  COALESCE("cte0"."v1", 0) + COALESCE("cte1"."v1", 0) AS "activity"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  COALESCE("cte0"."v1", 0) + COALESCE("cte1"."v1", 0) > 5
ORDER BY
  "activity" DESC NULLS LAST;
```

### Window condition with an expansion, a range, a negation, and a comma

> Each form of comparison works against a window function

```qd
#issues %%(project\p created_at\sd)%row_number:[1 2] %%(project\p id\s)%row_number:1..3 !%%(author\p)%count:1 %%(status\p)%count:1,status:open $id
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
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."id" ASC NULLS LAST) AS "qdwin1",
      count(*) OVER (PARTITION BY "issues"."author") AS "qdwin2",
      count(*) OVER (PARTITION BY "issues"."status") AS "qdwin3"
    FROM "issues"
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  ("issues"."qdwin0" = 1 OR "issues"."qdwin0" = 2) AND
  "issues"."qdwin1" >= 1 AND
  "issues"."qdwin1" <= 3 AND
  ("issues"."qdwin2" = 1) IS NOT TRUE AND
  ("issues"."qdwin3" = 1 OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE));
```

### Date arithmetic compared with a duration

> Issues created about a week ago, give or take a day: the date measured from now is the creation date plus a week, which is parenthesized within the subtraction

```qd
#issues created_at|plus(1w):<1d $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - ("issues"."created_at" + make_interval(weeks => 1)))) < EXTRACT(epoch FROM make_interval(days => 1));
```

### Comparisons as the operands of a comparison

> Projects whose two flags are both true or both not: SQL can't chain comparison operators, so each operand that is itself a comparison is parenthesized

```qd
#projects (is_active:@true):(is_pro_bono:@true) $id
```

```sql
SELECT
  "projects"."id"
FROM "projects"
WHERE
  ("projects"."is_active" = TRUE) = ("projects"."is_pro_bono" = TRUE);
```

### Difference of two dates compared with a duration

> Issues that were given more than a week: the difference of two dates is a duration, so it's compared with the duration directly, rather than being measured from now as a date would be

```qd
#issues due_date - created_at:>1w $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."due_date" - "issues"."created_at" > make_interval(weeks => 1);
```

### Bare words as the bounds of a range

> Issues whose title sorts between "a" and "m": a bare word is a string literal anywhere on the right-hand side, the bounds of a range included

```qd
#issues title:a..m $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."title" >= 'a' AND
  "issues"."title" <= 'm';
```

### Null check of a comparison

> Projects for which it isn't known whether they're active: the comparison is parenthesized within `IS NULL`

```qd
#projects (is_active:@true):@null !(is_pro_bono:@false):@null $id
```

```sql
SELECT
  "projects"."id"
FROM "projects"
WHERE
  ("projects"."is_active" = TRUE) IS NULL AND
  NOT ("projects"."is_pro_bono" = FALSE) IS NULL;
```

### Range of bare words with exclusive bounds, alongside a to-one path

> The `..` of a range doesn't make its bounds into paths, whereas `author.username` is one

```qd
#issues title:a<..<m status:open..reopened description:author.username $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  "issues"."title" > 'a' AND
  "issues"."title" < 'm' AND
  "issues"."status" >= 'open' AND
  "issues"."status" <= 'reopened' AND
  COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower("users"."username" COLLATE "C")) > 0, FALSE);
```

### Date compared with a computed duration

> A duration needn't be a literal: issues created within eight days, or due within twice the time since they were created

```qd
#issues created_at:<1w + 1d due_date:<(@now - `created_at`) * 2 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1) + make_interval(days => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM (NOW() - "issues"."created_at") * 2);
```

### Date compared with the result of a duration function

> Users last seen within as long as they've been alive: `age` is a duration, so the comparison measures `last_seen` from now

```qd
#users last_seen:<`birth_date`|age $id
```

```sql
SELECT
  "users"."id"
FROM "users"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "users"."last_seen")) < EXTRACT(epoch FROM NOW() - "users"."birth_date");
```

### Difference of two dates as a result column, a sort, and a function argument

> The difference is a duration wherever it appears, so `minus` isn't measured from now either

```qd
#issues due_date|minus(created_at):>1w \\due_date - created_at \d $(due_date - created_at)|days|floor
```

```sql
SELECT
  FLOOR(EXTRACT(epoch FROM "issues"."due_date" - "issues"."created_at") / 86400)
FROM "issues"
WHERE
  "issues"."due_date" - "issues"."created_at" > make_interval(weeks => 1)
ORDER BY
  "issues"."due_date" - "issues"."created_at" DESC NULLS LAST;
```

### Difference of two plain dates compared with a number

```toml options
schema = "library"
```

> Subtracting one plain date from another gives a number of days, in both databases

```qd
#checkouts due_date - due_date:<3 $id
```

```sql
SELECT
  "Checkouts"."id"
FROM "Checkouts"
WHERE
  "Checkouts"."Due Date" - "Checkouts"."Due Date" < 3;
```

### Comma after a comparison

> The comma separates whole conditions: issues whose status contains "open", or which mention "closed" anywhere. (To compare one column with several values, use an expansion.)

```qd
#issues status:open,closed $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE));
```

## Combinations: result columns, sorting, and grouping

These cases combine aliases, control flags, globs, nesting, standalone sorting, and grouping with one another and with computed values.

### Sort ordinals across columns

> The ordinals, not the column positions, decide the sort precedence

```qd
#issues $title \s2 $created_at \sd1
```

```sql
SELECT
  "issues"."title",
  "issues"."created_at"
FROM "issues"
ORDER BY
  "issues"."created_at" DESC NULLS LAST,
  "issues"."title" ASC NULLS LAST;
```

### Descending sort with nulls first

> The `d` and `n` flags together

```qd
#issues $title $created_at \sdn
```

```sql
SELECT
  "issues"."title",
  "issues"."created_at"
FROM "issues"
ORDER BY
  "issues"."created_at" DESC NULLS FIRST;
```

### Grouping and sorting flags on the same column

> Count issues by status, in status order

```qd
#issues $status \gs $%count
```

```sql
SELECT
  "issues"."status",
  count(*)
FROM "issues"
GROUP BY "issues"."status"
ORDER BY
  "issues"."status" ASC NULLS LAST;
```

### Grouping ordinals and sorting ordinals on the same columns

> Group by status then project, but sort by project then status

```qd
#issues $status \g1s2 $project \g2s1 $%count
```

```sql
SELECT
  "issues"."status",
  "issues"."project",
  count(*)
FROM "issues"
GROUP BY "issues"."status", "issues"."project"
ORDER BY
  "issues"."project" ASC NULLS LAST,
  "issues"."status" ASC NULLS LAST;
```

### Grouped column sorted descending alongside a sorted aggregate

> The grouped column's sort comes first, since it comes first among the columns

```qd
#issues $status \g2 $project \g1s $%count \sd
```

```sql
SELECT
  "issues"."status",
  "issues"."project",
  count(*)
FROM "issues"
GROUP BY "issues"."project", "issues"."status"
ORDER BY
  "issues"."project" ASC NULLS LAST,
  count(*) DESC NULLS LAST;
```

### Alias, sort flag, and annotation on one column

> The alias comes first, then the flags, then the annotation. The sort refers to the alias.

```qd
#issues $id -> x \s @{w:1}
```

```sql
SELECT
  "issues"."id" AS "x"
FROM "issues"
ORDER BY
  "x" ASC NULLS LAST;
```

```json
{
  "columnAnnotations": [
    {
      "w": 1
    }
  ]
}
```

### Alias that collides with the base table name, sorted

> The `ORDER BY` refers to the alias `issues`, not to the table

```qd
#issues $#comments -> n \g $%count -> issues \sd
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
  COALESCE("cte0"."v1", 0) AS "n",
  count(*) AS "issues"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
GROUP BY COALESCE("cte0"."v1", 0)
ORDER BY
  "issues" DESC NULLS LAST;
```

### Grouping by a function result, a comparison, and a has condition

> Each computed grouping expression is repeated in the `GROUP BY`

```qd
#issues $status|uppercase \g $status:open \g $++#comments -> has_comments \g $%count
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
  upper("issues"."status"),
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE),
  "cte0"."pk" IS NOT NULL AS "has_comments",
  count(*)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
GROUP BY upper("issues"."status"), COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE), "cte0"."pk" IS NOT NULL;
```

### Grouping by a to-one column while listing a column further along the same path

> For each author's username, count their issues and list their teams

```qd
#issues $author.username \g $%count $author.team.name%list
```

```sql
SELECT
  "users"."username",
  count(*),
  array_agg(DISTINCT "teams"."name")
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id"
GROUP BY "users"."username";
```

### Arithmetic and a case expression over aggregates in a grouped query

> Aggregates may be combined within an ungrouped column

```qd
#issues $status \g $%count * 2 + id%max -> v $ ? %count:>10 ~ "many" ~~ "few" -> size
```

```sql
SELECT
  "issues"."status",
  count(*) * 2 + max("issues"."id") AS "v",
  CASE
    WHEN count(*) > 10 THEN 'many'
    ELSE 'few'
  END AS "size"
FROM "issues"
GROUP BY "issues"."status";
```

### Format string over aggregates in a grouped query, sorted

> A summary sentence per status

```qd
#issues $status \g $^f"{%count} issues, latest {created_at%max}" -> summary \s
```

```sql
SELECT
  "issues"."status",
  concat(count(*), ' issues, latest ', max("issues"."created_at")) AS "summary"
FROM "issues"
GROUP BY "issues"."status"
ORDER BY
  "summary" ASC NULLS LAST;
```

### Grouping after has and count conditions on related records

> Count commented issues with more than five assignments, by status

```qd
#issues ++#comments #assignments:>5 $status \g $%count \sd
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
      "assignments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "assignments"
    GROUP BY "assignments"."issue"
  )
SELECT
  "issues"."status",
  count(*)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  COALESCE("cte1"."v1", 0) > 5
GROUP BY "issues"."status"
ORDER BY
  count(*) DESC NULLS LAST;
```

### Standalone sort by an aggregate in a grouped query

> Statuses, most common first, without showing the count

```qd
#issues \\%count \d \\created_at%max $status \g
```

```sql
SELECT
  "issues"."status"
FROM "issues"
GROUP BY "issues"."status"
ORDER BY
  count(*) DESC NULLS LAST,
  max("issues"."created_at") ASC NULLS LAST;
```

### Standalone sorts by a count of related records and a to-one column

> Most-commented issues first, then by author

```qd
#issues \\#comments \d \\author.username $title
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
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
ORDER BY
  COALESCE("cte0"."v1", 0) DESC NULLS LAST,
  "users"."username" ASC NULLS LAST;
```

### Standalone sort by an aggregate of related records with nulls first

> Issues by most recent comment, uncommented issues first

```qd
#issues \\#comments.created_at%max \dn $title
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
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
ORDER BY
  "cte0"."v1" DESC NULLS FIRST;
```

### Standalone sorts by a case expression and a function pipeline

> Open issues first, then by lowercased title

```qd
#issues \\ ? status:open ~ 1 ~~ 2 \\title|lowercase $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  CASE
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 1
    ELSE 2
  END ASC NULLS LAST,
  lower("issues"."title") ASC NULLS LAST;
```

### Standalone sorts by a comparison and a has condition

> Booleans sort like any other value

```qd
#issues \\status:open \d \\++#comments $title
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
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
ORDER BY
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) DESC NULLS LAST,
  "cte0"."pk" IS NOT NULL ASC NULLS LAST;
```

### Standalone sort by a sorted list aggregate

> Issues ordered by their alphabetized list of label names

```qd
#issues \\#labels.name%list(\s) $title
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
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
ORDER BY
  "cte0"."v1" ASC NULLS LAST;
```

### Standalone sort alongside a sort within a glob

> The standalone sort takes precedence

```qd
#issues \\id \d $*(title \s)
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
ORDER BY
  "issues"."id" DESC NULLS LAST,
  "issues"."title" ASC NULLS LAST;
```

### Scoped sorting through a to-one path into a count of related records

> Sort by project name, then by the number of issues in the project

```qd
#issues \\project.( \\name \d \\#issues ) $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
ORDER BY
  "projects"."name" DESC NULLS LAST,
  COALESCE("cte0"."v1", 0) ASC NULLS LAST;
```

### Scoped sorting with functions, negation, and has conditions

> Each entry begins with a column reference behind a different prefix or suffix

```qd
#issues \\author.( \\team.name \\#issues \d \\username|lowercase \\!username:x \\++#comments ) $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."author"
  ),
  "cte1" AS (
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
LEFT JOIN "cte1" ON
  "users"."id" = "cte1"."pk"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id"
ORDER BY
  "teams"."name" ASC NULLS LAST,
  COALESCE("cte0"."v1", 0) DESC NULLS LAST,
  lower("users"."username") ASC NULLS LAST,
  NOT COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE) ASC NULLS LAST,
  "cte1"."pk" IS NOT NULL ASC NULLS LAST;
```

### Scoped sorting through a to-many path with aggregates

> Sort by latest comment, then by the list of commenters

```qd
#issues \\#comments.( \\created_at%max \d \\user.username%list(\s) ) $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg(DISTINCT "users"."username" ORDER BY "users"."username" ASC NULLS LAST) AS "v1"
    FROM "comments"
    JOIN "users" ON
      "comments"."user" = "users"."id"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
ORDER BY
  "cte0"."v1" DESC NULLS LAST,
  "cte1"."v1" ASC NULLS LAST;
```

### Standalone sort by a to-one column that is also the grouping column

> Count issues by project name, in name order

```qd
#issues \\project.name $project.name \g $%count
```

```sql
SELECT
  "projects"."name",
  count(*)
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
GROUP BY "projects"."name"
ORDER BY
  "projects"."name" ASC NULLS LAST;
```

### Glob with a hidden column that is then shown again under an alias

> The column moves to the end

```qd
#issues $*(id \h) $id -> again
```

```sql
SELECT
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date",
  "issues"."id" AS "again"
FROM "issues";
```

### Glob entries with an alias, sorting, an annotation, and hiding together

> One glob entry carries an alias, a sort, and an annotation; another is hidden

```qd
#issues $*(id->identifier \sd @{w:1} title \h)
```

```sql
SELECT
  "issues"."id" AS "identifier",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
ORDER BY
  "issues"."id" DESC NULLS LAST;
```

```json
{
  "columnAnnotations": [
    {
      "w": 1
    },
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

### Glob on a related table with its own entries

> The glob's entries refer to the columns of the related table

```qd
#issues $author.*(username \sd email \h team.name \s)
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen",
  "users"."preferences"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id"
ORDER BY
  "users"."username" DESC NULLS LAST,
  "teams"."name" ASC NULLS LAST;
```

### Glob sorted by a column of a related table and by a count of related records

> Sorting entries within a glob needn't be columns of the globbed table

```qd
#issues $*(author.username \s #comments \sd)
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
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
ORDER BY
  "users"."username" ASC NULLS LAST,
  COALESCE("cte0"."v1", 0) DESC NULLS LAST;
```

### Globs on a chain of related tables, starting with a self-reference

> The self-referenced table is joined under an alias

```qd
#issues $duplicate_of.* $duplicate_of.author.* $duplicate_of.author.team.*
```

```sql
SELECT
  "issues_1"."id",
  "issues_1"."title",
  "issues_1"."description",
  "issues_1"."created_at",
  "issues_1"."author",
  "issues_1"."status",
  "issues_1"."project",
  "issues_1"."duplicate_of",
  "issues_1"."due_date",
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen",
  "users"."preferences",
  "teams"."id",
  "teams"."name"
FROM "issues"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id"
LEFT JOIN "users" ON
  "issues_1"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id";
```

### Globs within nested result columns

> The outer path applies to each glob, and each glob's entries to its own table

```qd
#comments $issue.( $*(description \h title \s) $author.*(email \h) )
```

```sql
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date",
  "users"."id",
  "users"."username",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen",
  "users"."preferences"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
ORDER BY
  "issues"."title" ASC NULLS LAST;
```

### Grouping within nested result columns

> Count comments by the name of their issue's project

```qd
#comments $issue.project.( $name \g ) $%count \sd
```

```sql
SELECT
  "projects"."name",
  count(*)
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
GROUP BY "projects"."name"
ORDER BY
  count(*) DESC NULLS LAST;
```

### Has conditions, negation, and a filtered count within nested result columns

> Each nested column is evaluated relative to the comment's issue

```qd
#comments $issue.( $++#labels -> labeled $!status:open -> not_open $#comments{user:issue.author} )
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
  "cte0"."pk" IS NOT NULL AS "labeled",
  NOT COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) AS "not_open",
  COALESCE("cte1"."v1", 0)
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk";
```

### Nested result columns after a filtered to-many path

> The filter applies to each nested aggregate

```qd
#issues $#comments{body:foo}.( $created_at%max $created_at%min )
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk",
      min("comments"."created_at") AS "v1"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "cte0"."v1",
  "cte1"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk";
```

### Nested result columns within nested result columns, through to-many paths

> The latest comment on any of the project's issues, and the issue titles

```qd
#projects $#issues.( $#comments.( $created_at%max ) $title%list )
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "issues"
    JOIN "comments" ON
      "issues"."id" = "comments"."issue"
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg(DISTINCT "issues"."title") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "cte0"."v1",
  "cte1"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Annotations within nested result columns and nested globs

> Each annotation lands in the slot of its own output column

```qd
#comments $issue.( $id @{a:1} $project.( $name @{b:2} ) $*(title @{c:3}) ) $id
```

```sql
SELECT
  "issues"."id",
  "projects"."name",
  "issues"."id",
  "issues"."title",
  "issues"."description",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date",
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id";
```

```json
{
  "columnAnnotations": [
    {
      "a": 1
    },
    {
      "b": 2
    },
    null,
    {
      "c": 3
    },
    null,
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

### Annotations on grouped, aliased, and sorted columns

> The annotation follows the flags and the alias

```qd
#issues $status \g @{a:1} $%count -> n \sd @{b:2} $created_at%max @{c:3}
```

```sql
SELECT
  "issues"."status",
  count(*) AS "n",
  max("issues"."created_at")
FROM "issues"
GROUP BY "issues"."status"
ORDER BY
  "n" DESC NULLS LAST;
```

```json
{
  "columnAnnotations": [
    {
      "a": 1
    },
    {
      "b": 2
    },
    {
      "c": 3
    }
  ]
}
```

### Annotations on computed columns of several kinds

> A window function, a case expression, a format string, and a comparison

```qd
#issues $%%(project\p)%count @{w:1} $ ? id:1 ~ 2 ~~ 3 @{c:1} $^f"{id}" -> x @{f:1} $id:1 @{cmp:1}
```

```sql
SELECT
  count(*) OVER (PARTITION BY "issues"."project"),
  CASE
    WHEN "issues"."id" = 1 THEN 2
    ELSE 3
  END,
  concat("issues"."id") AS "x",
  "issues"."id" = 1
FROM "issues";
```

```json
{
  "columnAnnotations": [
    {
      "w": 1
    },
    {
      "c": 1
    },
    {
      "f": 1
    },
    {
      "cmp": 1
    }
  ]
}
```

### Annotation values of every kind

> Quoted keys, nested and empty containers, negative and fractional numbers, and the `@` constants

```qd
#issues $id @{} $title @{"quoted key":1 'k2':2 k_3:{} b:[] c:"" d:-1.5 e:'x y' f:@null g:[@true @false]}
```

```sql
SELECT
  "issues"."id",
  "issues"."title"
FROM "issues";
```

```json
{
  "columnAnnotations": [
    {},
    {
      "quoted key": 1,
      "k2": 2,
      "k_3": {},
      "b": [],
      "c": "",
      "d": -1.5,
      "e": "x y",
      "f": null,
      "g": [
        true,
        false
      ]
    }
  ]
}
```

### Aggregate of a foreign key column in a grouped query

> For each status, the number of distinct authors: a foreign key column at the end of an aggregated path is the value to aggregate, rather than a link to follow

```qd
#issues $status \g $author%distinct
```

```sql
SELECT
  "issues"."status",
  count(DISTINCT "issues"."author")
FROM "issues"
GROUP BY "issues"."status";
```

### Count of related records as a grouping column

> The number of issues having each number of comments: a count of related records can be grouped by

```qd
#issues $#comments -> comments \g $%count \sd
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
  COALESCE("cte0"."v1", 0) AS "comments",
  count(*)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
GROUP BY COALESCE("cte0"."v1", 0)
ORDER BY
  count(*) DESC NULLS LAST;
```

### Standalone sort by a grouping expression that is a function result

> A standalone sort that matches a grouping column is valid in a grouped query, even when the column is computed

```qd
#issues \\created_at|year \d $created_at|year \g $%count
```

```sql
SELECT
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER),
  count(*)
FROM "issues"
GROUP BY CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER)
ORDER BY
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) DESC NULLS LAST;
```

### List and maximum of a foreign key column in a grouped query

> For each project, the ids of its issues' authors and the greatest of them

```qd
#issues $project \g $author%list(\s) $author%max
```

```sql
SELECT
  "issues"."project",
  array_agg(DISTINCT "issues"."author" ORDER BY "issues"."author" ASC NULLS LAST),
  max("issues"."author")
FROM "issues"
GROUP BY "issues"."project";
```

## Combinations: related records

These cases combine to-one paths, to-many paths, filters, has conditions, scoped comparisons, aggregate functions, and aggregate sorting with one another.

### Filtered path with an expansion and a negated to-one comparison

> Count each project's open or reopened issues that weren't authored by bob

```qd
#projects $name $#issues{status:[open reopened] !author.username:bob}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    WHERE
      (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE)) AND
      NOT COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('bob' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Filtered path whose filter has its own has conditions, with a sorted list

> List the ids of each project's unlabeled issues that have a comment mentioning x

```qd
#projects $name $#issues{++#comments{body:x} --#labels}.id%list(\sd)
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk"
        FROM "comments"
        WHERE
          COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE)
        GROUP BY "comments"."issue"
      ),
      "cte1" AS (
        SELECT
          "issue_labels"."issue" AS "pk"
        FROM "issue_labels"
        JOIN "labels" ON
          "issue_labels"."label" = "labels"."id"
        GROUP BY "issue_labels"."issue"
      )
    SELECT
      "issues"."project" AS "pk",
      array_agg(DISTINCT "issues"."id" ORDER BY "issues"."id" DESC NULLS LAST) AS "v1"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    LEFT JOIN "cte1" ON
      "issues"."id" = "cte1"."pk"
    WHERE
      "cte0"."pk" IS NOT NULL AND
      "cte1"."pk" IS NULL
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Two filtered tables along one path, with a sorted list of everything

> List the comments, by members of the core team, on each project's recent issues

```qd
#projects $name $#issues{created_at:<1w}.#comments{user.team.name:=core}.body%list_all(\\created_at \d)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg("comments"."body" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "v1"
    FROM "issues"
    JOIN "comments" ON
      "issues"."id" = "comments"."issue"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    LEFT JOIN "teams" ON
      "users"."team" = "teams"."id"
    WHERE
      ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
      lower("teams"."name" COLLATE "C") = lower('core' COLLATE "C")
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Default text search within a filtered path

> The bare terms search the text columns of the filtered table

```qd
#projects $name $#issues{foo,!bar}.title%list
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg(DISTINCT "issues"."title") AS "v1"
    FROM "issues"
    WHERE
      (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)))
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Scoped comparison within a filtered path

> Count each project's issues whose author is on a team named like x

```qd
#projects $name $#issues{author{team.name:x}}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    LEFT JOIN "teams" ON
      "users"."team" = "teams"."id"
    WHERE
      COALESCE(strpos(lower("teams"."name" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Range and date function within a filtered path

> Sum the ids of certain issues created in 2020

```qd
#projects $name $#issues{id:1..5 created_at|year:2020}.id%sum
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      sum("issues"."id") AS "v1"
    FROM "issues"
    WHERE
      "issues"."id" >= 1 AND
      "issues"."id" <= 5 AND
      CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) = 2020
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Has condition whose filter compares a count

> Projects having an issue with more than three comments

```qd
#projects ++#issues{#comments:>3} $name
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk",
          count(*) AS "v1"
        FROM "comments"
        GROUP BY "comments"."issue"
      )
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    WHERE
      COALESCE("cte0"."v1", 0) > 3
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Count and has-none conditions on differently filtered paths to the same table

> Projects with more than five open issues and no closed ones

```qd
#projects #issues{status:open}:>5 #issues{status:closed}:0 $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('closed' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
WHERE
  COALESCE("cte0"."v1", 0) > 5 AND
  "cte1"."pk" IS NULL;
```

### Arithmetic between two counts of related records

> The percentage of each project's issues that are open

```qd
#projects $name $#issues{status:open} * 100 / #issues -> pct_open
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0) * 100 / COALESCE("cte1"."v1", 0) AS "pct_open"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Scalar functions applied to aggregates of related records

> A function piped after a count, and after an aggregate function

```qd
#projects $name $#issues|max(1)|times(2) $#issues.id%max|plus(1)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      max("issues"."id") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  GREATEST(COALESCE("cte0"."v1", 0), 1) * 2,
  "cte1"."v1" + 1
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Arithmetic and date functions over aggregates of related records

> The span between each project's earliest issue and latest due date, and the age of its earliest issue

```qd
#projects $name $#issues.due_date%max - #issues.created_at%min -> span $#issues.created_at%min|age|days|floor
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      max("issues"."due_date") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      min("issues"."created_at") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1" - "cte1"."v1" AS "span",
  FLOOR(EXTRACT(epoch FROM NOW() - "cte1"."v1") / 86400)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Several aggregate functions over differently filtered paths

> `count` of a column is coalesced to zero like a bare count, but other aggregates are not

```qd
#projects $#issues{status:open}.id%count $#issues{status:open}.id%sum $#issues.id%min $#issues.title%max
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count("issues"."id") AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      sum("issues"."id") AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  ),
  "cte2" AS (
    SELECT
      "issues"."project" AS "pk",
      min("issues"."id") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "cte3" AS (
    SELECT
      "issues"."project" AS "pk",
      max("issues"."title") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  COALESCE("cte0"."v1", 0),
  "cte1"."v1",
  "cte2"."v1",
  "cte3"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "projects"."id" = "cte2"."pk"
LEFT JOIN "cte3" ON
  "projects"."id" = "cte3"."pk";
```

### Boolean aggregates through an inferred intermediate table

> Whether all, and whether any, of each client's projects are active

```qd
#clients $name $#projects.is_active%all_true $#projects.is_active%any_true
```

```sql
WITH
  "cte0" AS (
    SELECT
      "products"."client" AS "pk",
      bool_and("projects"."is_active") AS "v1"
    FROM "products"
    JOIN "projects" ON
      "products"."id" = "projects"."product"
    GROUP BY "products"."client"
  ),
  "cte1" AS (
    SELECT
      "products"."client" AS "pk",
      bool_or("projects"."is_active") AS "v1"
    FROM "products"
    JOIN "projects" ON
      "products"."id" = "projects"."product"
    GROUP BY "products"."client"
  )
SELECT
  "clients"."name",
  "cte0"."v1",
  "cte1"."v1"
FROM "clients"
LEFT JOIN "cte0" ON
  "clients"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "clients"."id" = "cte1"."pk";
```

### Transitive path written in shorthand and in full

> Both columns take the same route from clients to issues

```qd
#clients $name $#issues $#products.#projects.#issues
```

```sql
WITH
  "cte0" AS (
    SELECT
      "products"."client" AS "pk",
      count(*) AS "v1"
    FROM "products"
    JOIN "projects" ON
      "products"."id" = "projects"."product"
    JOIN "issues" ON
      "projects"."id" = "issues"."project"
    GROUP BY "products"."client"
  )
SELECT
  "clients"."name",
  COALESCE("cte0"."v1", 0),
  COALESCE("cte0"."v1", 0)
FROM "clients"
LEFT JOIN "cte0" ON
  "clients"."id" = "cte0"."pk";
```

### List through a to-many path and two to-one hops, sorted by its value

> The teams of the authors of each project's issues

```qd
#projects $name $#issues.author.team.name%list(\s)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg(DISTINCT "teams"."name" ORDER BY "teams"."name" ASC NULLS LAST) AS "v1"
    FROM "issues"
    JOIN "users" ON
      "issues"."author" = "users"."id"
    JOIN "teams" ON
      "users"."team" = "teams"."id"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### List through two to-many hops and two to-one hops, sorted by frequency

> The teams commenting on each project's issues, most frequent first

```qd
#projects $name $#issues.#comments.user.team.name%list(\\%count \d)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "issues"."project" AS "pk",
          "teams"."name" AS "value",
          count(*) AS "sort1"
        FROM "issues"
        JOIN "comments" ON
          "issues"."id" = "comments"."issue"
        JOIN "users" ON
          "comments"."user" = "users"."id"
        JOIN "teams" ON
          "users"."team" = "teams"."id"
        GROUP BY "issues"."project", "teams"."name"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" DESC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### List sorted by an aggregate of the same to-one record it lists from

> The titles of each user's assigned issues, most recently created first

```qd
#users $username $#assignments.issue.title%list(\\issue.created_at%max \d)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "assignments"."user" AS "pk",
          "issues"."title" AS "value",
          max("issues_1"."created_at") AS "sort1"
        FROM "assignments"
        JOIN "issues" ON
          "assignments"."issue" = "issues"."id"
        LEFT JOIN "issues" AS "issues_1" ON
          "assignments"."issue" = "issues_1"."id"
        GROUP BY "assignments"."user", "issues"."title"
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" DESC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  )
SELECT
  "users"."username",
  "cte0"."v1"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### To-one chain followed by a transitive to-many path

> For each project, the number of issues across all of its client's products

```qd
#projects $name $product.client.#products.#projects.#issues
```

```sql
WITH
  "cte0" AS (
    SELECT
      "products"."client" AS "pk",
      count(*) AS "v1"
    FROM "products"
    JOIN "projects" ON
      "products"."id" = "projects"."product"
    JOIN "issues" ON
      "projects"."id" = "issues"."project"
    GROUP BY "products"."client"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0)
FROM "projects"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id"
LEFT JOIN "clients" ON
  "products"."client" = "clients"."id"
LEFT JOIN "cte0" ON
  "clients"."id" = "cte0"."pk";
```

### Has condition through a to-one hop with a filter

> Issues in a project that has an open issue

```qd
#issues ++project.#issues{status:open} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Has conditions and an aggregate through a to-many hop, a to-one hop, and a to-many hop

> Issues with a commenter who has authored an issue, with the greatest such issue's id

```qd
#issues ++#comments.user.#issues $#comments.user.#issues.id%max $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    JOIN "users" ON
      "comments"."user" = "users"."id"
    JOIN "issues" ON
      "users"."id" = "issues"."author"
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk",
      max("issues"."id") AS "v1"
    FROM "comments"
    JOIN "users" ON
      "comments"."user" = "users"."id"
    JOIN "issues" ON
      "users"."id" = "issues"."author"
    GROUP BY "comments"."issue"
  )
SELECT
  "cte1"."v1",
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Linking column alongside an inferred path to the same table

> For each user, the issues they authored and the issues assigned to them

```qd
#users $username $#issues(author) $#assignments.#issues
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."author"
  ),
  "cte1" AS (
    SELECT
      "assignments"."user" AS "pk",
      count(*) AS "v1"
    FROM "assignments"
    JOIN "issues" ON
      "assignments"."issue" = "issues"."id"
    GROUP BY "assignments"."user"
  )
SELECT
  "users"."username",
  COALESCE("cte0"."v1", 0),
  COALESCE("cte1"."v1", 0)
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "users"."id" = "cte1"."pk";
```

### Both linking columns of a table, each followed by a to-one hop and a sorted list

> The titles of the issues each issue blocks, and of those it's blocked by

```qd
#issues $#blocks(blocker).blocking.title%list(\s) $#blocks(blocking).blocker.title%list(\s)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "blocks"."blocker" AS "pk",
      array_agg(DISTINCT "issues"."title" ORDER BY "issues"."title" ASC NULLS LAST) AS "v1"
    FROM "blocks"
    JOIN "issues" ON
      "blocks"."blocking" = "issues"."id"
    GROUP BY "blocks"."blocker"
  ),
  "cte1" AS (
    SELECT
      "blocks"."blocking" AS "pk",
      array_agg(DISTINCT "issues"."title" ORDER BY "issues"."title" ASC NULLS LAST) AS "v1"
    FROM "blocks"
    JOIN "issues" ON
      "blocks"."blocker" = "issues"."id"
    GROUP BY "blocks"."blocking"
  )
SELECT
  "cte0"."v1",
  "cte1"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk";
```

### Linking column with a filter through the other linking column

> Issues blocking an open issue

```qd
#issues ++#blocks(blocker){blocking.status:open} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "blocks"."blocker" AS "pk"
    FROM "blocks"
    LEFT JOIN "issues" AS "blocking" ON
      "blocks"."blocking" = "blocking"."id"
    WHERE
      COALESCE(strpos(lower("blocking"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "blocks"."blocker"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Self-referencing linking column with a filter and a list

> The ids of each issue's open duplicates

```qd
#issues $id $#issues(duplicate_of){status:open}.id%list
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."duplicate_of" AS "pk",
      array_agg(DISTINCT "issues"."id") AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."duplicate_of"
  )
SELECT
  "issues"."id",
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Path that revisits its starting table through a join table

> For each label, the labels that share an issue with it

```qd
#labels $name $#issues.#labels.name%list
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."label" AS "pk",
      array_agg(DISTINCT "labels"."name") AS "v1"
    FROM "issue_labels"
    JOIN "issues" ON
      "issue_labels"."issue" = "issues"."id"
    JOIN "issue_labels" AS "issue_labels_1" ON
      "issues"."id" = "issue_labels_1"."issue"
    JOIN "labels" ON
      "issue_labels_1"."label" = "labels"."id"
    GROUP BY "issue_labels"."label"
  )
SELECT
  "labels"."name",
  "cte0"."v1"
FROM "labels"
LEFT JOIN "cte0" ON
  "labels"."id" = "cte0"."pk";
```

### Filter comparing against a function of the parent record's column

> Count each issue's comments made more than a week after the issue was created

```qd
#issues $id $#comments{created_at:>issue.created_at|plus(1w)}
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
      "comments"."created_at" > "issues"."created_at" + make_interval(weeks => 1)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  COALESCE("cte0"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Has conditions with an expansion and a negation over the same join table

> Issues labeled bug or defect, but not wontfix, listing their other labels

```qd
#issues ++#labels{name:[bug defect]} --#labels{name:=wontfix} $#labels{!name:=bug}.name%list(\s)
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
      (COALESCE(strpos(lower("labels"."name" COLLATE "C"), lower('bug' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("labels"."name" COLLATE "C"), lower('defect' COLLATE "C")) > 0, FALSE))
    GROUP BY "issue_labels"."issue"
  ),
  "cte1" AS (
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    WHERE
      lower("labels"."name" COLLATE "C") = lower('wontfix' COLLATE "C")
    GROUP BY "issue_labels"."issue"
  ),
  "cte2" AS (
    SELECT
      "issue_labels"."issue" AS "pk",
      array_agg(DISTINCT "labels"."name" ORDER BY "labels"."name" ASC NULLS LAST) AS "v1"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    WHERE
      (lower("labels"."name" COLLATE "C") = lower('bug' COLLATE "C")) IS NOT TRUE
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "cte2"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "issues"."id" = "cte2"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NULL;
```

### Negated scoped comparison

> Comments whose issue isn't an open issue with foo in its title, including comments with no issue

```qd
#comments !issue{title:foo status:=open} $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C")) IS NOT TRUE;
```

### Scoped comparisons with negation, a comma, a nested set, and a range

> Each entry is evaluated against the comment's issue

```qd
#comments issue{!title:foo,status:=open} issue[!{title:a description:b} id:1..5] $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  (NOT COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C")) AND
  (NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('a' COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('b' COLLATE "C")) > 0, FALSE)) OR "issues"."id" >= 1 AND
  "issues"."id" <= 5);
```

### Scoped comparison with a dual expansion and a function range

> Expansion and ranges work inside a scope as they do at the top level

```qd
#comments issue{[title description]:~[foo bar] created_at|year:2020..2022} $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  ("issues"."title" ~* '(?p)foo' OR "issues"."title" ~* '(?p)bar' OR "issues"."description" ~* '(?p)foo' OR "issues"."description" ~* '(?p)bar') AND
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) >= 2020 AND
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) <= 2022;
```

### Scopes nested three deep

> The paths compose, and each level can hold its own conditions

```qd
#comments issue{author{team{name:=core} username:a} status:=open} $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id"
WHERE
  lower("teams"."name" COLLATE "C") = lower('core' COLLATE "C") AND
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('a' COLLATE "C")) > 0, FALSE) AND
  lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C");
```

### Scopes joined by commas with an ordinary comparison

> A search of the issue, or its project's name, or the comment's body

```qd
#comments issue{foo},issue.project{name:bar},body:baz $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  ((COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)) OR COALESCE(strpos(lower("projects"."name" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE));
```

### Scope containing has conditions with their own filter

> Issues whose author has an assignment and has no comment mentioning x

```qd
#issues author{++#assignments --#comments{body:x}} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "assignments"."user" AS "pk"
    FROM "assignments"
    GROUP BY "assignments"."user"
  ),
  "cte1" AS (
    SELECT
      "comments"."user" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."user"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "users"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NULL;
```

### Scope containing a count comparison and an aggregated date compared with a duration

> Issues whose author has more than five issues and commented within the past week

```qd
#issues author{#issues:>5 #comments.created_at%max:<1w} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."author"
  ),
  "cte1" AS (
    SELECT
      "comments"."user" AS "pk",
      max("comments"."created_at") AS "v1"
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
LEFT JOIN "cte1" ON
  "users"."id" = "cte1"."pk"
WHERE
  COALESCE("cte0"."v1", 0) > 5 AND
  ABS(EXTRACT(epoch FROM NOW() - "cte1"."v1")) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Scopes through a self-reference, nested and repeated

> Each level of nesting joins the issues table again

```qd
#issues duplicate_of{duplicate_of{status:=open}} duplicate_of{title:x} $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "issues" AS "issues_2" ON
  "issues"."duplicate_of" = "issues_2"."id"
LEFT JOIN "issues" AS "issues_1" ON
  "issues_2"."duplicate_of" = "issues_1"."id"
WHERE
  lower("issues_1"."status" COLLATE "C") = lower('open' COLLATE "C") AND
  COALESCE(strpos(lower("issues_2"."title" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE);
```

### Scoped comparison within a filter of a has condition

> Projects having an issue whose author is named like y, on a team named like x

```qd
#projects ++#issues{author{team.name:x username:y}} $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    LEFT JOIN "teams" ON
      "users"."team" = "teams"."id"
    WHERE
      COALESCE(strpos(lower("teams"."name" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE) AND
      COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('y' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Scoped comparisons within two filters along one path

> Each scope is relative to the table its filter belongs to

```qd
#projects $#issues{author{username:a}}.#comments{user{username:b}}.body%list
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg(DISTINCT "comments"."body") AS "v1"
    FROM "issues"
    JOIN "comments" ON
      "issues"."id" = "comments"."issue"
    LEFT JOIN "users" AS "users_1" ON
      "comments"."user" = "users_1"."id"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    WHERE
      COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('a' COLLATE "C")) > 0, FALSE) AND
      COALESCE(strpos(lower("users_1"."username" COLLATE "C"), lower('b' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Has condition through a to-one hop, filtered by a scoped backtick column

> Issues whose author has an issue in an active project

```qd
#issues ++author.#issues{project{`is_active`}} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk"
    FROM "issues"
    LEFT JOIN "projects" ON
      "issues"."project" = "projects"."id"
    WHERE
      "projects"."is_active"
    GROUP BY "issues"."author"
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

### Aggregate of a foreign key column of related records

> For each issue, the number of distinct users who commented on it: the same, at the end of a to-many path

```qd
#issues $id $#comments.user%distinct
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(DISTINCT "comments"."user") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  COALESCE("cte0"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### List and count of a foreign key column of related records

> For each issue, the ids of the users assigned to it, and the number of comments that have a user

```qd
#issues $id $#assignments.user%list(\s) $#comments.user%count
```

```sql
WITH
  "cte0" AS (
    SELECT
      "assignments"."issue" AS "pk",
      array_agg(DISTINCT "assignments"."user" ORDER BY "assignments"."user" ASC NULLS LAST) AS "v1"
    FROM "assignments"
    GROUP BY "assignments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk",
      count("comments"."user") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1",
  COALESCE("cte1"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk";
```

### Foreign key column aggregated after a to-one hop

> For each status, the number of distinct teams among the issues' authors: the path ends with a foreign key column of the linked table

```qd
#issues $status \g $author.team%distinct
```

```sql
SELECT
  "issues"."status",
  count(DISTINCT "users"."team")
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
GROUP BY "issues"."status";
```

## Combinations: definitions

These cases combine constants, computed columns, user-defined functions, custom comparisons, and the special `__querydown_*` comparisons with one another and with the rest of the language.

### Computed column listed across a to-many path, sorted by its value

> The computed expression is repeated in the `ORDER BY`

```qd
#issues.utitle = title|uppercase
#projects $name $#issues.utitle%list(\sd)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg(DISTINCT upper("issues"."title") ORDER BY upper("issues"."title") DESC NULLS LAST) AS "v1"
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

### Computed column listed and topped across a to-many path with aggregated sort keys

> The computed expression becomes the distinct value

```qd
#issues.utitle = title|uppercase
#projects $name $#issues.utitle%list(\\created_at%min) $#issues.utitle%top(\\%count \d)
```

```sql
WITH
  "cte0" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "issues"."project" AS "pk",
          upper("issues"."title") AS "value",
          min("issues"."created_at") AS "sort1"
        FROM "issues"
        GROUP BY "issues"."project", upper("issues"."title")
      )
    SELECT
      "distinct_values"."pk" AS "pk",
      array_agg("distinct_values"."value" ORDER BY "distinct_values"."sort1" ASC NULLS LAST) AS "v1"
    FROM "distinct_values"
    GROUP BY "distinct_values"."pk"
  ),
  "cte1" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "issues"."project" AS "pk",
          upper("issues"."title") AS "value",
          rank() OVER (PARTITION BY "issues"."project" ORDER BY count(*) DESC NULLS LAST) AS "rank"
        FROM "issues"
        GROUP BY "issues"."project", upper("issues"."title")
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
  "cte0"."v1",
  "cte1"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Computed count of related records, summed across a to-many path

> The total number of comments on each project's issues

```qd
#issues.n_comments = #comments
#projects $name $#issues.n_comments%sum
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk",
          count(*) AS "v1"
        FROM "comments"
        GROUP BY "comments"."issue"
      )
    SELECT
      "issues"."project" AS "pk",
      sum(COALESCE("cte0"."v1", 0)) AS "v1"
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

### Computed count of related records, summed in a grouped query

> The total number of comments on the issues of each status

```qd
#issues.n_comments = #comments
#issues $status \g $n_comments%sum
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
  "issues"."status",
  sum(COALESCE("cte0"."v1", 0))
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
GROUP BY "issues"."status";
```

### Computed count of related records used as a condition, a sort, and a result column

> Issues with more than two comments, most commented first, showing how many each has

```qd
#issues.n_comments = #comments
#issues n_comments:>2 \\n_comments \d $id $n_comments
```

Each use reads the same CTE, rather than each having one of its own.

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
  "issues"."id",
  COALESCE("cte0"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  COALESCE("cte0"."v1", 0) > 2
ORDER BY
  COALESCE("cte0"."v1", 0) DESC NULLS LAST;
```

### Range against a count of related records, within a filter of related records

> Count each project's issues that have between one and three comments

```qd
#projects $name $#issues{#comments:1..3}
```

Both bounds read the same nested CTE.

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk",
          count(*) AS "v1"
        FROM "comments"
        GROUP BY "comments"."issue"
      )
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    WHERE
      COALESCE("cte0"."v1", 0) >= 1 AND
      COALESCE("cte0"."v1", 0) <= 3
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Computed boolean column as a filter of related records, plain and negated

> A name containing an underscore isn't a search term, so it needs no backticks

```qd
#issues.is_open = status:[open reopened]
#projects $name $#issues{is_open} $#issues{!is_open}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE))
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      NOT (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE))
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0),
  COALESCE("cte1"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Computed boolean column as a bare condition, negated in a comma, and aggregated

> Backticks are optional here, since the name contains an underscore

```qd
#issues.is_open = status:[open reopened]
#projects ++#issues{`is_open`} $#issues.is_open%any_true $name
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    WHERE
      (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE))
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      bool_or(COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE)) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "cte1"."v1",
  "projects"."name"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Computed boolean column within a scope and within nested result columns

> The computed column is resolved against the related table in both places

```qd
#issues.is_open = status:[open reopened]
#comments issue{`is_open`} issue.is_open:@true $issue.( $is_open $title )
```

```sql
SELECT
  COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE),
  "issues"."title"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
WHERE
  (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE)) AND
  (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE)) = TRUE;
```

### Computed column reached through a to-one path, compared with a range

> Issues whose author is of working age

```qd
#users.age = birth_date|age|years|floor
#issues author.age:18..65 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) >= 18 AND
  FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) <= 65;
```

### Computed column through to-one hops in an aggregate and in a filter

> The average age of each issue's assignees, and the number of comments by people over 30

```qd
#users.age = birth_date|age|years|floor
#issues $id $#assignments.user.age%avg $#comments{user.age:>30}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "assignments"."issue" AS "pk",
      avg(FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600)) AS "v1"
    FROM "assignments"
    JOIN "users" ON
      "assignments"."user" = "users"."id"
    GROUP BY "assignments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    WHERE
      FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) > 30
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1",
  COALESCE("cte1"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk";
```

### Computed column as a window partition

> Number each issue among those whose authors are the same age

```qd
#users.age = birth_date|age|years|floor
#issues $id $%%(author.age\p created_at\s)%row_number
```

```sql
SELECT
  "issues"."id",
  row_number() OVER (PARTITION BY FLOOR(EXTRACT(epoch FROM NOW() - "users"."birth_date") / 31557600) ORDER BY "issues"."created_at" ASC NULLS LAST)
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id";
```

### Computed format string through a to-one path and listed across a to-many path

> The author's contact details, and those of the commenters

```qd
#users.display = ^f"{username} <{email}>"
#issues $author.display $#comments.user.display%list
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg(DISTINCT concat("users"."username", ' <', "users"."email", '>')) AS "v1"
    FROM "comments"
    JOIN "users" ON
      "comments"."user" = "users"."id"
    GROUP BY "comments"."issue"
  )
SELECT
  concat("users"."username", ' <', "users"."email", '>'),
  "cte0"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id";
```

### Computed case expression as a grouping column

> The whole case expression is repeated in the `GROUP BY`

```qd
#issues.label = ? status:open ~ "Open" ~~ "Other"
#issues $label \g $%count
```

```sql
SELECT
  CASE
    WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 'Open'
    ELSE 'Other'
  END,
  count(*)
FROM "issues"
GROUP BY CASE
  WHEN COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) THEN 'Open'
  ELSE 'Other'
END;
```

### Computed columns chained through a self-reference

> Each hop through `duplicate_of` evaluates the computed column against its own alias

```qd
#issues.next = id + 1
#issues.after = next * 2
#issues $after $duplicate_of.next $duplicate_of.duplicate_of.after
```

```sql
SELECT
  ("issues"."id" + 1) * 2,
  "issues_1"."id" + 1,
  ("issues_2"."id" + 1) * 2
FROM "issues"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id"
LEFT JOIN "issues" AS "issues_2" ON
  "issues_1"."duplicate_of" = "issues_2"."id";
```

### Computed column defined before the computed column it uses

> Definitions may appear in any order

```qd
#issues.dup_next = duplicate_of.next
#issues.next = id + 1
#issues $dup_next
```

```sql
SELECT
  "issues_1"."id" + 1
FROM "issues"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id";
```

### Computed columns hosted on different tables, chained through to-one and to-many paths

> The number of issues by the author of the issue that each issue duplicates

```qd
#users.n_authored = #issues
#issues.author_load = author.n_authored
#issues.dup_author_load = duplicate_of.author_load
#issues $dup_author_load
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."author" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."author"
  )
SELECT
  COALESCE("cte0"."v1", 0)
FROM "issues"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id"
LEFT JOIN "users" ON
  "issues_1"."author" = "users"."id"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### Computed column defined with a user-defined function over two counts, sorted

> Projects by their percentage of open issues

```qd
@@pct = @a @b => @a * 100 / @b
#projects.open_pct = @@pct(#issues{status:open} #issues)
#projects $name $open_pct \sd
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0) * 100 / COALESCE("cte1"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
ORDER BY
  COALESCE("cte0"."v1", 0) * 100 / COALESCE("cte1"."v1", 0) DESC NULLS LAST;
```

### Computed column defined with a custom comparison

> A custom comparison may be used within a computed column's definition

```qd
#issues.tagged:@x = ++#labels{name:=@x}
#issues.triaged = tagged:triaged
#issues `triaged` $triaged $id
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
      lower("labels"."name" COLLATE "C") = lower('triaged' COLLATE "C")
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "cte0"."pk" IS NOT NULL,
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Computed column defined with a constant

> The constant is inlined into the computed column

```qd
@threshold = 5
#issues.is_busy = #comments:>@threshold
#issues is_busy $id $is_busy
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
  "issues"."id",
  COALESCE("cte0"."v1", 0) > 5
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  COALESCE("cte0"."v1", 0) > 5;
```

### Date constant and a constant computed from `@now`

> A constant may hold any expression, which is inlined wherever it's used, including within filters of related records

```qd
@since = @2020-01-01
@cutoff = @now|minus(1y)
#projects ++#issues{created_at:>@cutoff} $name $#issues{created_at:>@since}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    WHERE
      "issues"."created_at" > NOW() - make_interval(years => 1)
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      "issues"."created_at" > DATE '2020-01-01'
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte1"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Subquery constant within arithmetic, as a condition and as a result column

> Projects holding more than a tenth of all issues, with their share

```qd
@n = #( #issues $%count )
#projects #issues:>@n / 10 $name $#issues / @n
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0) / (SELECT
    count(*)
  FROM "issues")
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  COALESCE("cte0"."v1", 0) > (SELECT
    count(*)
  FROM "issues") / 10;
```

### Subquery constant within a filter of related records

> For each issue, the number of its comments made at the time of the latest comment overall

```qd
@latest = #( #comments $created_at%max )
#issues $id $#comments{created_at:@latest}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    WHERE
      "comments"."created_at" = (SELECT
        max("comments"."created_at")
      FROM "comments")
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  COALESCE("cte0"."v1", 0)
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Subquery constant whose query has a condition on related records

> Issues older than the oldest commented issue

```qd
@oldest_commented = #( #issues ++#comments $created_at%min )
#issues created_at:<@oldest_commented $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."created_at" < (WITH
    "cte0" AS (
      SELECT
        "comments"."issue" AS "pk"
      FROM "comments"
      GROUP BY "comments"."issue"
    )
  SELECT
    min("issues"."created_at")
  FROM "issues"
  LEFT JOIN "cte0" ON
    "issues"."id" = "cte0"."pk"
  WHERE
    "cte0"."pk" IS NOT NULL);
```

### Custom comparison negated and with a right expansion

> Each use compiles its own existence check

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues !comment:foo comment:[foo bar] $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  NOT "cte0"."pk" IS NOT NULL AND
  ("cte0"."pk" IS NOT NULL OR "cte1"."pk" IS NOT NULL);
```

### Custom comparison with a flagged regex expansion, joined by a comma to a negated switched operator

> The switched operator applies within the custom comparison's body

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues comment:\cm~[foo bar],!comment:=baz $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      "comments"."body" ~ '(?n)foo'
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      "comments"."body" ~ '(?n)bar'
    GROUP BY "comments"."issue"
  ),
  "cte2" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      lower("comments"."body" COLLATE "C") = lower('baz' COLLATE "C")
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "issues"."id" = "cte2"."pk"
WHERE
  ("cte0"."pk" IS NOT NULL OR "cte1"."pk" IS NOT NULL OR NOT "cte2"."pk" IS NOT NULL);
```

### Custom comparison within filters of related records

> Projects having an issue with no comment mentioning foo, listing the titles of those that do have one

```qd
#issues.comment:@x = ++#comments{body:@x}
#projects ++#issues{!comment:foo} $name $#issues{comment:foo}.title%list(\s)
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk"
        FROM "comments"
        WHERE
          COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
        GROUP BY "comments"."issue"
      )
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    WHERE
      NOT "cte0"."pk" IS NOT NULL
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk"
        FROM "comments"
        WHERE
          COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
        GROUP BY "comments"."issue"
      )
    SELECT
      "issues"."project" AS "pk",
      array_agg(DISTINCT "issues"."title" ORDER BY "issues"."title" ASC NULLS LAST) AS "v1"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    WHERE
      "cte0"."pk" IS NOT NULL
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte1"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Custom comparison within a scope and as a nested result column

> The custom comparison is resolved against the related table in both places

```qd
#issues.comment:@x = ++#comments{body:@x}
#comments issue{comment:foo} $id $issue.( $comment:foo -> c )
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "comments"."id",
  "cte0"."pk" IS NOT NULL AS "c"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Custom comparison as a grouping column, sorted by count

> Count the issues that have a comment mentioning foo, and those that don't

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues \\%count \d $comment:foo -> mentions_foo \g $ ? %count:>1 ~ "many" ~~ "few"
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "cte0"."pk" IS NOT NULL AS "mentions_foo",
  CASE
    WHEN count(*) > 1 THEN 'many'
    ELSE 'few'
  END
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
GROUP BY "cte0"."pk" IS NOT NULL
ORDER BY
  count(*) DESC NULLS LAST;
```

### Custom comparison as a standalone sort and a case test

> A custom comparison yields a boolean wherever an expression is allowed

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues \\comment:urgent \d $ ? comment:foo ~ "!" ~~ "" -> flag $title
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('urgent' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  CASE
    WHEN "cte1"."pk" IS NOT NULL THEN '!'
    ELSE ''
  END AS "flag",
  "issues"."title"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
ORDER BY
  "cte0"."pk" IS NOT NULL DESC NULLS LAST;
```

### Custom comparison called with a constant

> The constant is inlined into the custom comparison's body

```qd
#issues.comment:@x = ++#comments{body:@x}
@term = "foo"
#issues comment:@term $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Custom comparison used within another custom comparison

> The outer comparison's argument is passed along to the inner one

```qd
#issues.c:@x = ++#comments{body:@x}
#issues.cc:@x = [c:@x title:@x]
#issues cc:foo c:bar $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  ("cte0"."pk" IS NOT NULL OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)) AND
  "cte1"."pk" IS NOT NULL;
```

### Custom comparison used within another through a self-reference

> The title of the issue that an issue's duplicate target itself duplicates

```qd
#issues.dup:@x = duplicate_of.title:@x
#issues.dupdup:@x = duplicate_of.dup:@x
#issues dupdup:foo $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "issues" AS "issues_2" ON
  "issues"."duplicate_of" = "issues_2"."id"
LEFT JOIN "issues" AS "issues_1" ON
  "issues_2"."duplicate_of" = "issues_1"."id"
WHERE
  COALESCE(strpos(lower("issues_1"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE);
```

### Custom comparison defined with `:=`, in a comma and negated

> The comparison is called exactly as it was defined

```qd
#issues.is:@x = status:=@x
#issues is:open,is:closed !is:done $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C") OR lower("issues"."status" COLLATE "C") = lower('closed' COLLATE "C")) AND
  (lower("issues"."status" COLLATE "C") = lower('done' COLLATE "C")) IS NOT TRUE;
```

### Custom comparison with a has condition, in AND and OR sets

> Issues with both of two labels, and either of two others

```qd
#issues.tagged:@x = ++#labels{name:=@x}
#issues {tagged:bug tagged:urgent} [tagged:a tagged:b] $id
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
  ),
  "cte1" AS (
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    WHERE
      lower("labels"."name" COLLATE "C") = lower('urgent' COLLATE "C")
    GROUP BY "issue_labels"."issue"
  ),
  "cte2" AS (
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    WHERE
      lower("labels"."name" COLLATE "C") = lower('a' COLLATE "C")
    GROUP BY "issue_labels"."issue"
  ),
  "cte3" AS (
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    WHERE
      lower("labels"."name" COLLATE "C") = lower('b' COLLATE "C")
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "issues"."id" = "cte2"."pk"
LEFT JOIN "cte3" ON
  "issues"."id" = "cte3"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NOT NULL AND
  ("cte2"."pk" IS NOT NULL OR "cte3"."pk" IS NOT NULL);
```

### Default text search configured with a custom comparison, negated and in a comma

> Each search term expands through both definitions

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues.__querydown_default_text_search:@x = [title:@x comment:@x]
#issues foo !bar,baz $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte2" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "issues"."id" = "cte2"."pk"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR "cte0"."pk" IS NOT NULL) AND
  (NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR "cte1"."pk" IS NOT NULL) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE) OR "cte2"."pk" IS NOT NULL);
```

### Configured default text search within a filter of related records and within scopes

> The configuration belongs to the table, wherever that table is searched

```qd
#issues.__querydown_default_text_search:@x = [title:@x description:@x]
#comments issue{foo} issue[bar baz] ++issue.project.#issues{qux} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    WHERE
      (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('qux' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('qux' COLLATE "C")) > 0, FALSE))
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
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)) AND
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('baz' COLLATE "C")) > 0, FALSE)) AND
  "cte0"."pk" IS NOT NULL;
```

### Configured default text search with quoted, formatted, and negated terms

> Every kind of search term goes through the configuration

```qd
#issues.__querydown_default_text_search:@x = title:@x
#issues foo "two words" ^f"{id}" !x $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('two words' COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower(concat("issues"."id") COLLATE "C")) > 0, FALSE) AND
  NOT COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('x' COLLATE "C")) > 0, FALSE);
```

### Configured default text search using a linked record comparison

> Searching issues also searches the author's username

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#issues.__querydown_default_text_search:@x = [title:@x author:@x]
#issues alice $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE));
```

### Linked record comparison with an expansion, a negation, and a switched operator

> Each form of comparison works against a linked record

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#issues author:[alice bob] !author:carol author:=dave $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  (COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('bob' COLLATE "C")) > 0, FALSE)) AND
  NOT COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('carol' COLLATE "C")) > 0, FALSE) AND
  lower("users"."username" COLLATE "C") = lower('dave' COLLATE "C");
```

### Linked record comparison within filters of related records

> The link is resolved from the filtered table

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#issues ++#comments{user:alice} --#assignments{user:=bob} $id
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
      COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "assignments"."issue" AS "pk"
    FROM "assignments"
    LEFT JOIN "users" ON
      "assignments"."user" = "users"."id"
    WHERE
      lower("users"."username" COLLATE "C") = lower('bob' COLLATE "C")
    GROUP BY "assignments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NULL;
```

### Linked record comparison within a scope and through a filtered to-many path

> Comments on alice's issues which also have a comment by carol

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#comments issue{author:alice} ++issue.#comments{user:~'^carol'} $id
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
      "users"."username" ~* '(?p)^carol'
    GROUP BY "comments"."issue"
  )
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE) AND
  "cte0"."pk" IS NOT NULL;
```

### Linked record comparisons on two tables at once

> Each link uses the comparison of the table it leads to

```qd
#projects.__querydown_linked_record_comparison:@x = name:@x
#products.__querydown_linked_record_comparison:@x = name:@x
#issues project:website project.product:=acme $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id"
WHERE
  COALESCE(strpos(lower("projects"."name" COLLATE "C"), lower('website' COLLATE "C")) > 0, FALSE) AND
  lower("products"."name" COLLATE "C") = lower('acme' COLLATE "C");
```

### Constant and computed column holding arithmetic, as a right operand

> An inlined constant or computed column that holds arithmetic is parenthesized where it's the right operand of an operator that binds as tightly

```qd
@a = 1 + 2
#issues.next = id + 1
#issues $id - @a $id - next
```

```sql
SELECT
  "issues"."id" - (1 + 2),
  "issues"."id" - ("issues"."id" + 1)
FROM "issues";
```

### Function bodies holding arithmetic, as a right operand

> The inlined body of a user-defined function or of an anonymous function is parenthesized likewise

```qd
@@inc = @x => @x + 1
#issues $1 - @@inc(id) $10 - id|(@x => @x - 1)
```

```sql
SELECT
  1 - ("issues"."id" + 1),
  10 - ("issues"."id" - 1)
FROM "issues";
```

### Duration constant compared with a date

> Issues created within the past week: a duration held by a constant is still a duration, so comparing a date with it measures the date from now

```qd
@recent = 1w
#issues created_at:<@recent $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Duration passed to a user-defined function and to a custom comparison

> A duration bound to a function's parameter or to a custom comparison's parameter is still a duration too

```qd
@@within = @d @span => @d:<@span
#issues.older_than:@x = created_at:>@x
#issues due_date|within(1w) older_than:6m $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) > EXTRACT(epoch FROM make_interval(months => 6));
```

### Null constant in a match comparison

> Issues with no title but with a status: `@null` held by a constant still compiles to `IS NULL`

```qd
@none = @null
#issues title:@none !status:@none $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."title" IS NULL AND
  NOT "issues"."status" IS NULL;
```

### Null passed to a linked record comparison and to a custom comparison

> `@null` passed to a custom comparison reaches the comparisons in its body as `@null`, so `author:@null` finds issues whose author has no username (among them, issues with no author)

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#issues.comment:@x = ++#comments{body:@x}
#issues author:@null comment:@null $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      "comments"."body" IS NULL
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  "users"."username" IS NULL AND
  "cte0"."pk" IS NOT NULL;
```

### Empty string constant in a match comparison

> Issues with an empty description: matching against an empty string held by a constant is strict equality, as it is for a literal `""`

```qd
@empty = ""
#issues description:@empty $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."description" = '';
```

### Left expansion over a custom comparison

> Issues with "foo" in a comment or in the title: an entry of a left-hand expansion can name a custom comparison

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues [comment title]:foo $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  ("cte0"."pk" IS NOT NULL OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE));
```

### Left expansion over linked records

> Issues with "alice" as the author's username or in the title: an entry of a left-hand expansion that names a linked record uses the linked record comparison

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#issues [author title]:alice $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  (COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE));
```

### User-defined function applied to its own result

> A user-defined function can be applied to a value that itself contains a call to the same function, which isn't recursion

```qd
@@inc = @x => @x + 1
#issues $id|inc|inc $@@inc(@@inc(id))
```

```sql
SELECT
  "issues"."id" + 1 + 1,
  "issues"."id" + 1 + 1
FROM "issues";
```

### Constant used by another constant, shadowed by a function parameter

> A constant means what it meant where it was defined: `@b` is `@a * 2` with `@a = 1`, even within a function whose parameter is also named `@a`

```qd
@a = 1
@b = @a * 2
#issues $id|(@a => @a + @b)
```

```sql
SELECT
  "issues"."id" + 1 * 2
FROM "issues";
```

### Function argument that references a column, used within a filter of related records

> Issues that their own author has commented on: the argument `author` is a column of the issue, even though the function uses it within a filter on `#comments`, so the issue is joined into the CTE that finds the comments

```qd
@@has_comment_by = @u => ++#comments{user:@u}
#issues author|has_comment_by $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    WHERE
      "comments"."user" = "issues"."author"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Custom comparison argument that references a column

> Issues having a comment that mentions the issue's own title: the same, with a custom comparison

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues comment:`title` $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower("issues"."title" COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Constant holding a search term

> A constant bound to a string is a search term when it stands alone as a condition, negated or not

```qd
@term = "foo"
@other = @term
#issues @term !@other $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)) AND
  NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE));
```

### Zero constant compared with a count of related records

> Issues with no comments: zero held by a constant still makes the comparison a has-none condition

```qd
@none = 0
#issues #comments:@none $id
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
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NULL;
```

### Duration constant scaled and compared with a date

> Issues due within twice the grace period

```qd
@grace = 1w
#issues due_date:<@grace * 2 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "issues"."due_date")) < EXTRACT(epoch FROM make_interval(weeks => 1) * 2);
```

### Regex pattern passed through a function parameter and a constant

> A pattern that arrives through variables is still merged with the flags at compile time

```qd
@pattern = '^fix'
@@titled = @t @p => @t:\m~@p
#issues title|titled(@pattern) $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."title" ~* '(?n)^fix';
```

### Argument that is a constant of the same name as the parameter

> The argument `@x` is the constant, and the `@x` within the function's body is the parameter

```qd
@x = 5
#issues $@x|(@x => @x + 1) $id|(@x => @x * 2)
```

```sql
SELECT
  5 + 1,
  "issues"."id" * 2
FROM "issues";
```

### Named function called within an anonymous function

> The body of a named function sees the constants, the anonymous function's body sees its own parameter and the constants

```qd
@step = 10
@@bump = @n => @n + @step
#issues $id|(@n => @n|bump * @step)
```

```sql
SELECT
  ("issues"."id" + 10) * 10
FROM "issues";
```

### Argument that references a column, used within a filter through a join table

> Issues having a label whose name appears in the issue's own title: the issue is joined to the row of the join table

```qd
#issues.labelled:@x = ++#labels{name:@x}
#issues !labelled:`title` $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issue_labels"."issue" AS "pk"
    FROM "issue_labels"
    JOIN "labels" ON
      "issue_labels"."label" = "labels"."id"
    LEFT JOIN "issues" ON
      "issue_labels"."issue" = "issues"."id"
    WHERE
      COALESCE(strpos(lower("labels"."name" COLLATE "C"), lower("issues"."title" COLLATE "C")) > 0, FALSE)
    GROUP BY "issue_labels"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  NOT "cte0"."pk" IS NOT NULL;
```

### Argument that references a column of a self-referencing table

> Issues having a duplicate with a different status: within the filter, `status` is the duplicate's, and the argument is the original's, which is joined under an alias of its own

```qd
@@has_duplicate_unlike = @s => ++#issues{!status:=@s}
#issues status|has_duplicate_unlike $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."duplicate_of" AS "pk"
    FROM "issues"
    LEFT JOIN "issues" AS "issues_1" ON
      "issues"."duplicate_of" = "issues_1"."id"
    WHERE
      (lower("issues"."status" COLLATE "C") = lower("issues_1"."status" COLLATE "C")) IS NOT TRUE
    GROUP BY "issues"."duplicate_of"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Argument computed from the record, used within a filter of related records

> Issues having a comment longer than the issue's own description

```qd
@@has_comment_longer_than = @n => ++#comments{body|length:>@n}
#issues description|length|has_comment_longer_than $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    WHERE
      char_length("comments"."body") > char_length("issues"."description")
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Argument that counts related records, used within a filter of related records

> Projects having an issue with more comments than the project has issues: the count of the project's issues is computed within the CTE that the project is joined into

```qd
#projects #issues|(@n => ++#issues{#comments:>@n}) $id
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk",
          count(*) AS "v1"
        FROM "comments"
        GROUP BY "comments"."issue"
      ),
      "cte1" AS (
        SELECT
          "issues"."project" AS "pk",
          count(*) AS "v1"
        FROM "issues"
        GROUP BY "issues"."project"
      )
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    LEFT JOIN "projects" ON
      "issues"."project" = "projects"."id"
    LEFT JOIN "cte1" ON
      "projects"."id" = "cte1"."pk"
    WHERE
      COALESCE("cte0"."v1", 0) > COALESCE("cte1"."v1", 0)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."id"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Argument that follows a to-one path, used within a filter of related records

> Issues having a comment by a teammate of the issue's author

```qd
@@has_comment_from_team = @t => ++#comments{user.team:@t}
#issues author.team|has_comment_from_team $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    LEFT JOIN "users" AS "users_1" ON
      "issues"."author" = "users_1"."id"
    WHERE
      "users"."team" = "users_1"."team"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Assignment evaluated where the function is called, used within a filter

> Issues with a comment made in the week before they were due: the local `@cutoff` is computed from the issue, not from the comment

```qd
@@commented_shortly_before = @d =>
  @cutoff = @d - 1w
  ++#comments{created_at:@cutoff..@d}
#issues due_date|commented_shortly_before $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    WHERE
      "comments"."created_at" >= "issues"."due_date" - make_interval(weeks => 1) AND
      "comments"."created_at" <= "issues"."due_date"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Literal arguments used within nested filters of related records

> A value that doesn't refer to its record needs no join, however deep it's used

```qd
@@has_issue_with_comment = @text @since => ++#issues{++#comments{body:@text created_at:>@since}}
#projects "urgent"|has_issue_with_comment(@2024-01-01) $id
```

```sql
WITH
  "cte0" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk"
        FROM "comments"
        WHERE
          COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('urgent' COLLATE "C")) > 0, FALSE) AND
          "comments"."created_at" > DATE '2024-01-01'
        GROUP BY "comments"."issue"
      )
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    WHERE
      "cte0"."pk" IS NOT NULL
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."id"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
WHERE
  "cte0"."pk" IS NOT NULL;
```

### Linked record comparison called with a column

> Issues whose author's username appears in the title: the argument is a column of the issue, not of the author that the comparison's body is evaluated against

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#issues `title`|(@t => author:@t) author:`description` $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower("issues"."title" COLLATE "C")) > 0, FALSE) AND
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower("issues"."description" COLLATE "C")) > 0, FALSE);
```

### Custom comparison within a scope, called with a column of the scoped record

> Issues whose author has written a comment containing their own username: within the scope, the argument is a column of the author, which is the record the comments are related to

```qd
#users.wrote:@x = ++#comments{body:@x}
#issues author{wrote:`username`} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk"
    FROM "comments"
    LEFT JOIN "users" ON
      "comments"."user" = "users"."id"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower("users"."username" COLLATE "C")) > 0, FALSE)
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

### Parameter used within a scope

> Comments made by the author of their issue: the argument is a column of the comment's user, even where the function's body uses it within the scope of the issue

```qd
#comments `user`.username|(@u => issue{author.username:=@u}) $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "users" AS "users_1" ON
  "comments"."user" = "users_1"."id"
WHERE
  lower("users"."username" COLLATE "C") = lower("users_1"."username" COLLATE "C");
```

### Left expansion over a custom comparison with a right expansion, negated

> Issues that don't have "foo" or "bar" in a comment or in the title: the left expansion is the outer one

```qd
#issues.comment:@x = ++#comments{body:@x}
#issues ![comment title]:[foo bar] $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      COALESCE(strpos(lower("comments"."body" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  NOT ("cte0"."pk" IS NOT NULL OR "cte1"."pk" IS NOT NULL OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE));
```

### Left expansion over a custom comparison and a linked record, with a switched operator

> Issues for which a comment, the author, or the title is exactly "alice"

```qd
#issues.comment:@x = ++#comments{body:@x}
#users.__querydown_linked_record_comparison:@x = username:@x
#issues [comment author title]:=alice $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    WHERE
      lower("comments"."body" COLLATE "C") = lower('alice' COLLATE "C")
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  ("cte0"."pk" IS NOT NULL OR lower("users"."username" COLLATE "C") = lower('alice' COLLATE "C") OR lower("issues"."title" COLLATE "C") = lower('alice' COLLATE "C"));
```

### Computed column unaffected by the parameters around it

> A computed column's definition sees the constants, not the parameters of a function that happens to use it

```qd
@k = 2
#issues.scaled = id * @k
@@plus_scaled = @k => @k + scaled
#issues $id|plus_scaled
```

```sql
SELECT
  "issues"."id" + "issues"."id" * 2
FROM "issues";
```

## Combinations: pipelines, unions, and user-defined tables

These cases combine `~~~`, `+++`, and `#name = #( )` with one another and with the rest of the language.

### Later stage with a comma, a range, a negation, a function, and a sort

> The match operator is exact equality after the first stage, since stage columns are untyped

```qd
#issues $id $title $status ~~~ title:foo status:=open,id:1..5 !title:bar $title|uppercase \s
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
  upper("pipe0"."title")
FROM "pipe0"
WHERE
  "pipe0"."title" = 'foo' AND
  ("pipe0"."status" = 'open' OR "pipe0"."id" >= 1 AND
  "pipe0"."id" <= 5) AND
  ("pipe0"."title" = 'bar') IS NOT TRUE
ORDER BY
  upper("pipe0"."title") ASC NULLS LAST;
```

### Later stage with a glob plus a computed, aliased, sorted column

> The glob expands to the previous stage's columns

```qd
#issues $author \g $%count -> n ~~~ $* $n * 2 -> doubled \sd
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."author" AS "author",
      count(*) AS "n"
    FROM "issues"
    GROUP BY "issues"."author"
  )
SELECT
  "pipe0"."author",
  "pipe0"."n",
  "pipe0"."n" * 2 AS "doubled"
FROM "pipe0"
ORDER BY
  "doubled" DESC NULLS LAST;
```

### Glob sort in a middle stage of three

> A sort in a stage that isn't the last is rendered within that stage's CTE

```qd
#issues $author \g $%count -> n ~~~ $*(n \sd) ~~~ n:>1
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."author" AS "author",
      count(*) AS "n"
    FROM "issues"
    GROUP BY "issues"."author"
  ),
  "pipe1" AS (
    SELECT
      "pipe0"."author" AS "author",
      "pipe0"."n" AS "n"
    FROM "pipe0"
    ORDER BY
      "pipe0"."n" DESC NULLS LAST
  )
SELECT
  "pipe1"."author",
  "pipe1"."n"
FROM "pipe1"
WHERE
  "pipe1"."n" > 1;
```

### Aggregates of related records, then aggregated again in the next stage

> The comments of each author's issues, summarized per author, keeping the busiest

```qd
#issues $author.username -> name $#comments -> c
~~~
$name \g $c%sum -> total $c%avg $c%max $%count
~~~
total:>10 \\total \d
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
      "users"."username" AS "name",
      COALESCE("cte0"."v1", 0) AS "c"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
  ),
  "pipe1" AS (
    SELECT
      "pipe0"."name" AS "name",
      sum("pipe0"."c") AS "total",
      avg("pipe0"."c") AS "column_3",
      max("pipe0"."c") AS "column_4",
      count(*) AS "column_5"
    FROM "pipe0"
    GROUP BY "pipe0"."name"
  )
SELECT
  "pipe1"."name",
  "pipe1"."total",
  "pipe1"."column_3",
  "pipe1"."column_4",
  "pipe1"."column_5"
FROM "pipe1"
WHERE
  "pipe1"."total" > 10
ORDER BY
  "pipe1"."total" DESC NULLS LAST;
```

### Unaliased computed columns and duplicate column names flowing between stages

> Unnamed columns are named by position, and duplicate names get a numeric suffix

```qd
#issues $id $project.id $created_at|year $1 ~~~ $*
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "projects"."id" AS "id_2",
      CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) AS "column_3",
      1 AS "column_4"
    FROM "issues"
    LEFT JOIN "projects" ON
      "issues"."project" = "projects"."id"
  )
SELECT
  "pipe0"."id",
  "pipe0"."id_2",
  "pipe0"."column_3",
  "pipe0"."column_4"
FROM "pipe0";
```

### Alias containing a space, referenced flexibly in the next stage

> Flexible identifier resolution applies to stage columns too

```qd
#issues $id -> `Issue ID` $title ~~~ `Issue ID`:5 $issue_id $`Issue ID` -> x
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "Issue ID",
      "issues"."title" AS "title"
    FROM "issues"
  )
SELECT
  "pipe0"."Issue ID",
  "pipe0"."Issue ID" AS "x"
FROM "pipe0"
WHERE
  "pipe0"."Issue ID" = 5;
```

### Hidden glob column is absent from the next stage

> A column hidden from a stage's output can't be referenced by later stages

```qd
#issues $*(description \h) ~~~ $*
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title",
      "issues"."created_at" AS "created_at",
      "issues"."author" AS "author",
      "issues"."status" AS "status",
      "issues"."project" AS "project",
      "issues"."duplicate_of" AS "duplicate_of",
      "issues"."due_date" AS "due_date"
    FROM "issues"
  )
SELECT
  "pipe0"."id",
  "pipe0"."title",
  "pipe0"."created_at",
  "pipe0"."author",
  "pipe0"."status",
  "pipe0"."project",
  "pipe0"."duplicate_of",
  "pipe0"."due_date"
FROM "pipe0";
```

### List aggregates over a previous stage's rows

> Distinct and non-distinct lists, each with its own sorting

```qd
#issues $id $status ~~~ $status \g $id%list(\sd) $id%list_all(\\id)
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."status" AS "status"
    FROM "issues"
  )
SELECT
  "pipe0"."status",
  array_agg(DISTINCT "pipe0"."id" ORDER BY "pipe0"."id" DESC NULLS LAST),
  array_agg("pipe0"."id" ORDER BY "pipe0"."id" ASC NULLS LAST)
FROM "pipe0"
GROUP BY "pipe0"."status";
```

### Window function over a previous stage, filtered in a third stage

> The three newest issues of each project

```qd
#issues $id $status $project ~~~ $%%(project\p id\sd)%row_number -> rn $id ~~~ rn:<=3
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."status" AS "status",
      "issues"."project" AS "project"
    FROM "issues"
  ),
  "pipe1" AS (
    SELECT
      row_number() OVER (PARTITION BY "pipe0"."project" ORDER BY "pipe0"."id" DESC NULLS LAST) AS "rn",
      "pipe0"."id" AS "id"
    FROM "pipe0"
  )
SELECT
  "pipe1"."rn",
  "pipe1"."id"
FROM "pipe1"
WHERE
  "pipe1"."rn" <= 3;
```

### Case expression and format string over a previous stage's columns

> Text comparisons against stage columns are exact

```qd
#issues $id $status ~~~ $ ? status:=open ~ 1 ~~ 0 -> is_open $^f"#{id}" -> tag
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."status" AS "status"
    FROM "issues"
  )
SELECT
  CASE
    WHEN "pipe0"."status" = 'open' THEN 1
    ELSE 0
  END AS "is_open",
  concat('#', "pipe0"."id") AS "tag"
FROM "pipe0";
```

### Constant and user-defined function used in a later stage

> Definitions are available to every stage

```qd
@min = 5
@@double = @x => @x * 2
#issues $author \g $%count -> n ~~~ n:>@min $n|double
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."author" AS "author",
      count(*) AS "n"
    FROM "issues"
    GROUP BY "issues"."author"
  )
SELECT
  "pipe0"."n" * 2
FROM "pipe0"
WHERE
  "pipe0"."n" > 5;
```

### Computed boolean column carried into the next stage as a plain column

> The next stage sees the computed column's value, referenced with backticks as a bare condition

```qd
#issues.big = id:>100
#issues $id $big ~~~ `big` $id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."id" > 100 AS "big"
    FROM "issues"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  "pipe0"."big";
```

### Window condition in a middle stage

> The window function is computed over the previous stage's rows

```qd
#issues status:open ~~~ %%(project\p created_at\sd)%row_number:1 $id ~~~ $%count
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title",
      "issues"."description" AS "description",
      "issues"."created_at" AS "created_at",
      "issues"."author" AS "author",
      "issues"."status" AS "status",
      "issues"."project" AS "project",
      "issues"."duplicate_of" AS "duplicate_of",
      "issues"."due_date" AS "due_date"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
  ),
  "pipe1" AS (
    WITH
      "qdwin_src" AS (
        SELECT
          "pipe0"."id",
          "pipe0"."title",
          "pipe0"."description",
          "pipe0"."created_at",
          "pipe0"."author",
          "pipe0"."status",
          "pipe0"."project",
          "pipe0"."duplicate_of",
          "pipe0"."due_date",
          row_number() OVER (PARTITION BY "pipe0"."project" ORDER BY "pipe0"."created_at" DESC NULLS LAST) AS "qdwin0"
        FROM "pipe0"
      )
    SELECT
      "pipe0"."id" AS "id"
    FROM "qdwin_src" AS "pipe0"
    WHERE
      "pipe0"."qdwin0" = 1
  )
SELECT
  count(*)
FROM "pipe1";
```

### Annotations from the last stage only

> Column annotations written in an earlier stage don't carry over to the output

```qd
#issues $id @{a:1} ~~~ $id @{b:2} $id + 1 @{c:[1 2 {d:e}]}
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id"
    FROM "issues"
  )
SELECT
  "pipe0"."id",
  "pipe0"."id" + 1
FROM "pipe0";
```

```json
{
  "columnAnnotations": [
    {
      "b": 2
    },
    {
      "c": [
        1,
        2,
        {
          "d": "e"
        }
      ]
    }
  ]
}
```

### Annotation on a glob entry in a later stage

> The annotation lands in the slot of the globbed column

```qd
#issues $id $title @{width:1} ~~~ $*(title @{width:2})
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title"
    FROM "issues"
  )
SELECT
  "pipe0"."id",
  "pipe0"."title"
FROM "pipe0";
```

```json
{
  "columnAnnotations": [
    null,
    {
      "width": 2
    }
  ]
}
```

### Union members using shared constants and a shared function with a format string

> Definitions written before the first query are available to every member

```qd
@since = @2024-01-01
@@tag = @kind @id => ^f"{@kind}-{@id}"
#issues created_at:>@since $@@tag("issue" id) -> ref $title
+++
#comments created_at:>@since $@@tag("comment" id) $body
```

```sql
SELECT
  concat('issue', '-', "issues"."id") AS "ref",
  "issues"."title" AS "body"
FROM "issues"
WHERE
  "issues"."created_at" > DATE '2024-01-01'
UNION ALL
SELECT
  concat('comment', '-', "comments"."id"),
  "comments"."body"
FROM "comments"
WHERE
  "comments"."created_at" > DATE '2024-01-01';
```

### Union members using a shared computed column and linked record comparison

> Each member resolves the definitions against its own base table

```qd
#issues.is_open = status:[open reopened]
#users.__querydown_linked_record_comparison:@x = username:@x
#issues `is_open` author:alice $id $title
+++
#comments issue{`is_open` author:alice} $issue.id $body
```

```sql
SELECT
  "issues"."id",
  "issues"."title" AS "body"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE)) AND
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE)
UNION ALL
SELECT
  "issues"."id",
  "comments"."body"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  (COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('reopened' COLLATE "C")) > 0, FALSE)) AND
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE);
```

### Union with column annotations from several members and a query annotation

> The last annotation of each column wins, and the query annotation follows the last member

```qd
#issues $id $title @{width:1}
+++
#projects $id @{hide:yes} $name @{width:2}
\\\{view:list}
```

```sql
SELECT
  "issues"."id",
  "issues"."title" AS "name"
FROM "issues"
UNION ALL
SELECT
  "projects"."id",
  "projects"."name"
FROM "projects";
```

```json
{
  "columnAnnotations": [
    {
      "hide": "yes"
    },
    {
      "width": 2
    }
  ],
  "queryAnnotation": {
    "view": "list"
  }
}
```

### Union of window columns, filtered in a pipeline stage

> The first row of each partition, from either member

```qd
#issues $id $%%(project\p created_at\s)%row_number -> rn
+++
#comments $id $%%(issue\p created_at\s)%row_number
~~~
rn:1 $id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST) AS "rn"
    FROM "issues"
    UNION ALL
    SELECT
      "comments"."id",
      row_number() OVER (PARTITION BY "comments"."issue" ORDER BY "comments"."created_at" ASC NULLS LAST)
    FROM "comments"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  "pipe0"."rn" = 1;
```

### Window condition in the first member of a union

> The first member's CTE precedes the whole union

```qd
#issues %%(project\p created_at\sd)%row_number:1 $id $title
+++
#projects $id $name
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
  "issues"."title" AS "name"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1
UNION ALL
SELECT
  "projects"."id",
  "projects"."name"
FROM "projects";
```

### Window condition in a later member of a union

> A later member's CTE is scoped to it with parentheses

```qd
#issues $id $title
+++
#projects %%(product\p id\s)%row_number:1 $id $name
```

```sql
SELECT
  "issues"."id",
  "issues"."title" AS "name"
FROM "issues"
UNION ALL
(
  WITH
    "qdwin_src" AS (
      SELECT
        "projects"."id",
        "projects"."name",
        "projects"."product",
        "projects"."is_active",
        "projects"."is_pro_bono",
        row_number() OVER (PARTITION BY "projects"."product" ORDER BY "projects"."id" ASC NULLS LAST) AS "qdwin0"
      FROM "projects"
    )
  SELECT
    "projects"."id",
    "projects"."name"
  FROM "qdwin_src" AS "projects"
  WHERE
    "projects"."qdwin0" = 1
);
```

### Union of globs with hidden columns

> Each member hides the same column, so the column counts agree

```qd
#issues $*(description \h)
+++
#issues $*(description \h)
```

```sql
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues"
UNION ALL
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."created_at",
  "issues"."author",
  "issues"."status",
  "issues"."project",
  "issues"."duplicate_of",
  "issues"."due_date"
FROM "issues";
```

### Union of counts of related records, then two pipeline stages

> Each member's CTE is nested within the first stage

```qd
#issues $id $#comments -> n
+++
#projects $id $#issues
~~~
n:>0 \\n \d
~~~
$%count
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
      COALESCE("cte0"."v1", 0) AS "n"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    UNION ALL
    (
      WITH
        "cte0" AS (
          SELECT
            "issues"."project" AS "pk",
            count(*) AS "v1"
          FROM "issues"
          GROUP BY "issues"."project"
        )
      SELECT
        "projects"."id",
        COALESCE("cte0"."v1", 0)
      FROM "projects"
      LEFT JOIN "cte0" ON
        "projects"."id" = "cte0"."pk"
    )
  ),
  "pipe1" AS (
    SELECT
      "pipe0"."id" AS "id",
      "pipe0"."n" AS "n"
    FROM "pipe0"
    WHERE
      "pipe0"."n" > 0
    ORDER BY
      "pipe0"."n" DESC NULLS LAST
  )
SELECT
  count(*)
FROM "pipe1";
```

### Union of user-defined tables, one of them twice

> A user-defined table may be the base table of several members

```qd
#a = #( #issues $id $title )
#b = #( #projects $id $name -> title )
#a $*
+++
#b $*
+++
#a title:foo $id $title
```

```sql
WITH
  "a" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title"
    FROM "issues"
  ),
  "b" AS (
    SELECT
      "projects"."id" AS "id",
      "projects"."name" AS "title"
    FROM "projects"
  )
SELECT
  "a"."id",
  "a"."title"
FROM "a"
UNION ALL
SELECT
  "b"."id",
  "b"."title"
FROM "b"
UNION ALL
SELECT
  "a"."id",
  "a"."title"
FROM "a"
WHERE
  "a"."title" = 'foo';
```

### Union of a grouped query and its grand total, over a user-defined table

> A subtotal per author, plus a total row

```qd
#recent = #( #issues created_at:<1w $id $title $author )
#recent $author \g $%count
+++
#recent $@null $%count
```

```sql
WITH
  "recent" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title",
      "issues"."author" AS "author"
    FROM "issues"
    WHERE
      ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1))
  )
SELECT
  "recent"."author",
  count(*)
FROM "recent"
GROUP BY "recent"."author"
UNION ALL
SELECT
  NULL,
  count(*)
FROM "recent";
```

### Union and pipeline within a subquery constant

> The constant is the number of rows in the union

```qd
@n = #( #issues $id +++ #projects $id ~~~ $%count )
#issues id:<@n $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  "issues"."id" < (WITH
    "pipe0" AS (
      SELECT
        "issues"."id" AS "id"
      FROM "issues"
      UNION ALL
      SELECT
        "projects"."id"
      FROM "projects"
    )
  SELECT
    count(*)
  FROM "pipe0");
```

### Union and pipeline within a user-defined table

> Ids shared between issues and projects

```qd
#u = #( #issues $id $title +++ #projects $id $name ~~~ $id \g $%count -> n )
#u n:>1
```

```sql
WITH
  "u" AS (
    WITH
      "pipe0" AS (
        SELECT
          "issues"."id" AS "id",
          "issues"."title" AS "name"
        FROM "issues"
        UNION ALL
        SELECT
          "projects"."id",
          "projects"."name"
        FROM "projects"
      )
    SELECT
      "pipe0"."id" AS "id",
      count(*) AS "n"
    FROM "pipe0"
    GROUP BY "pipe0"."id"
  )
SELECT
  "u"."id",
  "u"."n"
FROM "u"
WHERE
  "u"."n" > 1;
```

### Union deduplicated by grouping in a pipeline stage

> Grouping by every column removes the duplicates that `UNION ALL` keeps

```qd
#issues $id $title
+++
#issues $id $title
~~~
$id \g $title \g
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "title"
    FROM "issues"
    UNION ALL
    SELECT
      "issues"."id",
      "issues"."title"
    FROM "issues"
  )
SELECT
  "pipe0"."id",
  "pipe0"."title"
FROM "pipe0"
GROUP BY "pipe0"."id", "pipe0"."title";
```

### Union of a grouped member and an ungrouped member

> Each member groups independently

```qd
#issues $id \g $%count
+++
#issues $id $1
```

```sql
SELECT
  "issues"."id",
  count(*)
FROM "issues"
GROUP BY "issues"."id"
UNION ALL
SELECT
  "issues"."id",
  1
FROM "issues";
```

### Union members reaching the same columns through different to-one paths

> Each member has its own joins

```qd
#issues $id $title
+++
#issues $author.id $author.username
+++
#issues $project.id $project.name
```

```sql
SELECT
  "issues"."id",
  "issues"."title" AS "name"
FROM "issues"
UNION ALL
SELECT
  "users"."id",
  "users"."username"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
UNION ALL
SELECT
  "projects"."id",
  "projects"."name"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id";
```

### Union with a has condition, sorted in a pipeline stage, with a limit and offset

```toml options
limit = 5
offset = 2
```

> The limit and offset apply to the last stage only

```qd
#issues $id $title
+++
#projects ++#issues $id $name
~~~
\\id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."title" AS "name"
    FROM "issues"
    UNION ALL
    (
      WITH
        "cte0" AS (
          SELECT
            "issues"."project" AS "pk"
          FROM "issues"
          GROUP BY "issues"."project"
        )
      SELECT
        "projects"."id",
        "projects"."name"
      FROM "projects"
      LEFT JOIN "cte0" ON
        "projects"."id" = "cte0"."pk"
      WHERE
        "cte0"."pk" IS NOT NULL
    )
  )
SELECT
  "pipe0"."id",
  "pipe0"."name"
FROM "pipe0"
ORDER BY
  "pipe0"."id" ASC NULLS LAST
LIMIT 5
OFFSET 2;
```

### User-defined table with a date condition, used with a window condition

> The window subquery reads from the user-defined table's CTE

```qd
#t = #( #issues created_at:<1w $id $project $created_at )
#t %%(project\p created_at\sd)%row_number:1 $id
```

```sql
WITH
  "t" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."project" AS "project",
      "issues"."created_at" AS "created_at"
    FROM "issues"
    WHERE
      ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1))
  ),
  "qdwin_src" AS (
    SELECT
      "t"."id",
      "t"."project",
      "t"."created_at",
      row_number() OVER (PARTITION BY "t"."project" ORDER BY "t"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "t"
  )
SELECT
  "t"."id"
FROM "qdwin_src" AS "t"
WHERE
  "t"."qdwin0" = 1;
```

### User-defined table with a sort, alongside a subquery constant, with a limit

```toml options
limit = 3
```

> Only the outermost query is limited

```qd
@n = #( #issues $%count )
#t = #( #issues \\id $id )
#t id:<@n
```

```sql
WITH
  "t" AS (
    SELECT
      "issues"."id" AS "id"
    FROM "issues"
    ORDER BY
      "issues"."id" ASC NULLS LAST
  )
SELECT
  "t"."id"
FROM "t"
WHERE
  "t"."id" < (SELECT
    count(*)
  FROM "issues")
LIMIT 3;
```

### Date column of an earlier stage compared with a duration

> A stage's date columns keep their types, so the comparison still measures the date from now

```qd
#issues $id $created_at $due_date|plus(1d) -> deadline
~~~
created_at:<1w deadline:>1m $id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."created_at" AS "created_at",
      "issues"."due_date" + make_interval(days => 1) AS "deadline"
    FROM "issues"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "pipe0"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  ABS(EXTRACT(epoch FROM NOW() - "pipe0"."deadline")) > EXTRACT(epoch FROM make_interval(months => 1));
```

### Duration column of an earlier stage compared with a duration

> A stage's duration columns keep their types too, so this is a plain comparison of two durations

```qd
#issues $id $due_date|countdown -> remaining $due_date - created_at -> allowed
~~~
remaining:<1w allowed:>1m $id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."due_date" - NOW() AS "remaining",
      "issues"."due_date" - "issues"."created_at" AS "allowed"
    FROM "issues"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  "pipe0"."remaining" < make_interval(weeks => 1) AND
  "pipe0"."allowed" > make_interval(months => 1);
```

### Date column of a user-defined table compared with a duration

> The columns of a user-defined table are typed in the same way

```qd
#recent = #( #issues created_at:<1y $id $created_at $status )
#recent created_at:<1w status:open $id
```

```sql
WITH
  "recent" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."created_at" AS "created_at",
      "issues"."status" AS "status"
    FROM "issues"
    WHERE
      ABS(EXTRACT(epoch FROM NOW() - "issues"."created_at")) < EXTRACT(epoch FROM make_interval(years => 1))
  )
SELECT
  "recent"."id"
FROM "recent"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "recent"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  "recent"."status" = 'open';
```

### Date column of a union compared with a duration

> A union's column keeps its type when every member's column has that type. As in any later stage, a text column is untyped, and so compared exactly

```qd
#issues $id $created_at $title
+++
#comments $id $created_at $body -> other
~~~
created_at:<1w other:x $id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."created_at" AS "created_at",
      "issues"."title" AS "other"
    FROM "issues"
    UNION ALL
    SELECT
      "comments"."id",
      "comments"."created_at",
      "comments"."body" AS "other"
    FROM "comments"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "pipe0"."created_at")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  "pipe0"."other" = 'x';
```

### Union of a naive and a zoned timestamp compared with a duration

> A union of timestamps is a timestamp, even when only some of them are zoned

```qd
#issues $id $created_at
+++
#users $id $last_seen -> seen
~~~
seen:<1w $id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      "issues"."created_at" AS "seen"
    FROM "issues"
    UNION ALL
    SELECT
      "users"."id",
      "users"."last_seen" AS "seen"
    FROM "users"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "pipe0"."seen")) < EXTRACT(epoch FROM make_interval(weeks => 1));
```

### Aggregated date of an earlier stage compared with a duration

> Statuses whose most recent issue was created within the past week. (A condition can't use the aggregate in the stage that computes it.)

```qd
#issues $status \g $created_at%max -> latest $%count -> n
~~~
latest:<1w n:>5 $status
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."status" AS "status",
      max("issues"."created_at") AS "latest",
      count(*) AS "n"
    FROM "issues"
    GROUP BY "issues"."status"
  )
SELECT
  "pipe0"."status"
FROM "pipe0"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "pipe0"."latest")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  "pipe0"."n" > 5;
```

## Combinations: window functions

These cases combine window definitions, window function arguments, and window conditions with the rest of the language.

### Window partitioned by a to-one column and a function result

> Rank issues within each team and year

```qd
#issues $id $%%(author.team.name\p created_at|year\p id\sd)%rank
```

```sql
SELECT
  "issues"."id",
  rank() OVER (PARTITION BY "teams"."name", CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) ORDER BY "issues"."id" DESC NULLS LAST)
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id";
```

### Window ordered by a count of related records

> Rank each project's issues by their number of comments

```qd
#issues $id $%%(project\p #comments\sd)%row_number -> rank_by_comments
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
  "issues"."id",
  row_number() OVER (PARTITION BY "issues"."project" ORDER BY COALESCE("cte0"."v1", 0) DESC NULLS LAST) AS "rank_by_comments"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
```

### Window function arguments that are aggregates of related records

> Each project's total number of comments, and its latest comment

```qd
#issues $id $%%(project\p)%sum(#comments) $%%(project\p)%max(#comments.created_at%max)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  sum(COALESCE("cte0"."v1", 0)) OVER (PARTITION BY "issues"."project"),
  max("cte1"."v1") OVER (PARTITION BY "issues"."project")
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk";
```

### Window function arguments with a function pipeline, an offset, and a default

> The uppercased title from two issues earlier

```qd
#issues $id $%%(status\p created_at\s)%lag(title|uppercase 2 "n/a") $%%(status\p created_at\s)%lead(due_date)
```

```sql
SELECT
  "issues"."id",
  lag(upper("issues"."title"), 2, 'n/a') OVER (PARTITION BY "issues"."status" ORDER BY "issues"."created_at" ASC NULLS LAST),
  lead("issues"."due_date") OVER (PARTITION BY "issues"."status" ORDER BY "issues"."created_at" ASC NULLS LAST)
FROM "issues";
```

### Window partitioned by a comparison and ordered by a case expression

> Any expression may define a window

```qd
#issues $id $%%(status:open\p ? id:<10 ~ 1 ~~ 2 \s)%dense_rank
```

```sql
SELECT
  "issues"."id",
  dense_rank() OVER (PARTITION BY COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE) ORDER BY CASE
    WHEN "issues"."id" < 10 THEN 1
    ELSE 2
  END ASC NULLS LAST)
FROM "issues";
```

### Windows without partitions, with sort flags and ordinals

> An empty window, and windows with only ordering

```qd
#issues $id $%%()%count $%%(id\s)%row_number $%%(created_at\sdn)%percent_rank $%%(id\s1 status\s2)%cume_dist
```

```sql
SELECT
  "issues"."id",
  count(*) OVER (),
  row_number() OVER (ORDER BY "issues"."id" ASC NULLS LAST),
  percent_rank() OVER (ORDER BY "issues"."created_at" DESC NULLS FIRST),
  cume_dist() OVER (ORDER BY "issues"."id" ASC NULLS LAST, "issues"."status" ASC NULLS LAST)
FROM "issues";
```

### Value window functions over the same window

> The first, last, and second titles in each project

```qd
#issues $id $%%(project\p id\s)%first_value(title) $%%(project\p id\s)%last_value(title) $%%(project\p id\s)%nth_value(title 2)
```

```sql
SELECT
  "issues"."id",
  first_value("issues"."title") OVER (PARTITION BY "issues"."project" ORDER BY "issues"."id" ASC NULLS LAST),
  last_value("issues"."title") OVER (PARTITION BY "issues"."project" ORDER BY "issues"."id" ASC NULLS LAST),
  nth_value("issues"."title", 2) OVER (PARTITION BY "issues"."project" ORDER BY "issues"."id" ASC NULLS LAST)
FROM "issues";
```

### Arithmetic between window functions, and a function piped after one

> Each project's share of all issues

```qd
#issues $id $%%(project\p)%count * 100 / %%()%count -> pct $%%(project\p created_at\s)%row_number|plus(1)
```

```sql
SELECT
  "issues"."id",
  count(*) OVER (PARTITION BY "issues"."project") * 100 / count(*) OVER () AS "pct",
  row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST) + 1
FROM "issues";
```

### Window function compared within result columns, a case expression, and a negation

> In a result column, a window comparison needs no subquery

```qd
#issues $id $ ? %%(project\p created_at\s)%row_number:1 ~ "first" ~~ "later" -> pos $!%%(project\p)%count:1 -> has_siblings
```

```sql
SELECT
  "issues"."id",
  CASE
    WHEN row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST) = 1 THEN 'first'
    ELSE 'later'
  END AS "pos",
  (count(*) OVER (PARTITION BY "issues"."project") = 1) IS NOT TRUE AS "has_siblings"
FROM "issues";
```

### Window functions within a format string

> Each issue's position out of the total

```qd
#issues $id $^f"#{%%(id\s)%row_number} of {%%()%count}"
```

```sql
SELECT
  "issues"."id",
  concat('#', row_number() OVER (ORDER BY "issues"."id" ASC NULLS LAST), ' of ', count(*) OVER ())
FROM "issues";
```

### Window functions as a standalone sort and as a sorted, aliased column

> Issues from the biggest projects first

```qd
#issues \\%%(project\p)%count \d $id $%%(author\p)%count -> n \sd
```

```sql
SELECT
  "issues"."id",
  count(*) OVER (PARTITION BY "issues"."author") AS "n"
FROM "issues"
ORDER BY
  count(*) OVER (PARTITION BY "issues"."project") DESC NULLS LAST,
  "n" DESC NULLS LAST;
```

### Difference between a column and its lagged value

> The time elapsed since the previous issue in the project

```qd
#issues $id $created_at - %%(project\p created_at\s)%lag(created_at) -> gap
```

```sql
SELECT
  "issues"."id",
  "issues"."created_at" - lag("issues"."created_at") OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST) AS "gap"
FROM "issues";
```

### Two window conditions

> Both window functions are computed in the same subquery

```qd
#issues %%(project\p created_at\sd)%row_number:<=3 %%(author\p)%count:>1 $id
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
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0",
      count(*) OVER (PARTITION BY "issues"."author") AS "qdwin1"
    FROM "issues"
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" <= 3 AND
  "issues"."qdwin1" > 1;
```

### Window condition in an OR set with a has condition

> The has condition's CTE follows the window subquery

```qd
#issues [%%(project\p created_at\sd)%row_number:1 ++#comments] $id
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
  ),
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
WHERE
  ("issues"."qdwin0" = 1 OR "cte0"."pk" IS NOT NULL);
```

### Window condition after a has condition and a to-one condition, with related result columns

> The ordinary conditions apply within the window subquery, and the result columns join onto it

```qd
#issues ++#comments author.username:alice %%(project\p created_at\sd)%row_number:1 $id $#comments $author.username
```

```sql
WITH
  "qdwin_src" AS (
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
      "issues"."due_date",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    WHERE
      "cte0"."pk" IS NOT NULL AND
      COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE)
  ),
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  COALESCE("cte0"."v1", 0),
  "users"."username"
FROM "qdwin_src" AS "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  "issues"."qdwin0" = 1;
```

### Window condition ordered by a count of related records

> The issue with the most comments in each project

```qd
#issues %%(project\p #comments\sd)%row_number:1 $id
```

```sql
WITH
  "qdwin_src" AS (
    WITH
      "cte0" AS (
        SELECT
          "comments"."issue" AS "pk",
          count(*) AS "v1"
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
      "issues"."due_date",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY COALESCE("cte0"."v1", 0) DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
    LEFT JOIN "cte0" ON
      "issues"."id" = "cte0"."pk"
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Window condition partitioned through a to-one path that the result columns also use

> The newest issue per author's team, with the team's name

```qd
#issues %%(author.team\p created_at\sd)%row_number:1 $id $author.team.name
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
      row_number() OVER (PARTITION BY "users"."team" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
  )
SELECT
  "issues"."id",
  "teams"."name"
FROM "qdwin_src" AS "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id"
WHERE
  "issues"."qdwin0" = 1;
```

### Window condition with a standalone sort and a window result column

> The result column's window is computed over the rows that pass the window condition

```qd
#issues %%(project\p created_at\sd)%row_number:1 \\project.name $id $%%(status\p)%count
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
  count(*) OVER (PARTITION BY "issues"."status")
FROM "qdwin_src" AS "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  "issues"."qdwin0" = 1
ORDER BY
  "projects"."name" ASC NULLS LAST;
```

### Window condition with globs on the base table and a related table

> The base table's glob reads from the window subquery

```qd
#issues %%(project\p created_at\sd)%row_number:1 $*(title \s) $author.*
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
  "issues"."due_date",
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen",
  "users"."preferences"
FROM "qdwin_src" AS "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  "issues"."qdwin0" = 1
ORDER BY
  "issues"."title" ASC NULLS LAST;
```

### Window condition compared with a constant

> The three newest issues in each project

```qd
@limit = 3
#issues %%(project\p created_at\sd)%row_number:<=@limit $id
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
  "issues"."qdwin0" <= 3;
```

### Window condition before a search term

> The search applies within the window subquery

```qd
#issues %%(project\p created_at\sd)%row_number:1 foo $id
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
      (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE))
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Window condition with flagged regex conditions

> The ordinary conditions apply within the window subquery

```qd
#issues %%(project\p created_at\sd)%row_number:1 title:\cm~'^Fix' !description:\s~'a.b' $id
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
      "issues"."title" ~ '(?n)^Fix' AND
      ("issues"."description" ~* 'a.b') IS NOT TRUE
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Window condition with a limit and offset

```toml options
limit = 3
offset = 6
```

> The limit applies to the outer query

```qd
#issues %%(project\p created_at\sd)%row_number:1 \\id $id
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
  "issues"."qdwin0" = 1
ORDER BY
  "issues"."id" ASC NULLS LAST
LIMIT 3
OFFSET 6;
```

### Computed column holding a window function, as a sort and a compared result column

> A computed window column may be used anywhere but in a condition

```qd
#issues.rn = %%(project\p created_at\sd)%row_number
#issues \\rn $id $rn:1 -> newest
```

```sql
SELECT
  "issues"."id",
  row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) = 1 AS "newest"
FROM "issues"
ORDER BY
  row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) ASC NULLS LAST;
```

### Computed column holding a window function, filtered in the next stage

> The window value becomes a plain column of the next stage

```qd
#issues.rn = %%(project\p created_at\sd)%row_number
#issues $id $rn ~~~ rn:1
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id",
      row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "rn"
    FROM "issues"
  )
SELECT
  "pipe0"."id",
  "pipe0"."rn"
FROM "pipe0"
WHERE
  "pipe0"."rn" = 1;
```

### User-defined function whose body compares a window function

> Whether each issue is the newest in its project

```qd
@@nth = @n => %%(project\p created_at\sd)%row_number:@n
#issues $id $@@nth(1)
```

```sql
SELECT
  "issues"."id",
  row_number() OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" DESC NULLS LAST) = 1
FROM "issues";
```

### Search term before a window condition

> The most recent issue in each project, among those matching "foo": a search term may be followed by a window condition, since `%%` begins a new condition rather than piping the term to an aggregate

```qd
#issues foo %%(project\p created_at\sd)%row_number:1 $id
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
      (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE))
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Negated and quoted search terms before a window condition

> Each search term is its own condition, and so is the window condition that follows

```qd
#issues !foo "bar" %%(project\p)%count:>1 $id
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
      count(*) OVER (PARTITION BY "issues"."project") AS "qdwin0"
    FROM "issues"
    WHERE
      NOT (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)) AND
      (COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."description" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE))
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" > 1;
```

## Combinations: several to-one paths from one table

These cases follow two or more to-one links out of the same table, in combination with the rest of the language. The joins to the linked tables are rendered in the order in which the query first refers to them: conditions, then standalone sorting, then result columns.

### Result columns through two links

> Each issue's author and project

```qd
#issues $id $author.username $project.name
```

```sql
SELECT
  "issues"."id",
  "users"."username",
  "projects"."name"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id";
```

### Conditions through two links and a result column through a third

> The original of each duplicate issue that alice opened in the website project

```qd
#issues author.username:=alice project.name:=website $duplicate_of.title
```

```sql
SELECT
  "issues_1"."title"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id"
WHERE
  lower("users"."username" COLLATE "C") = lower('alice' COLLATE "C") AND
  lower("projects"."name" COLLATE "C") = lower('website' COLLATE "C");
```

### Joins ordered by first use

> The project is joined before the author, because a condition refers to it before any result column is compiled

```qd
#issues project.name:website $author.username $project.name
```

```sql
SELECT
  "users"."username",
  "projects"."name"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  COALESCE(strpos(lower("projects"."name" COLLATE "C"), lower('website' COLLATE "C")) > 0, FALSE);
```

### Scoped comparisons on two links

> Comments by bar on issues about foo

```qd
#comments issue{title:foo} user{username:=bar} $id
```

```sql
SELECT
  "comments"."id"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "users" ON
  "comments"."user" = "users"."id"
WHERE
  COALESCE(strpos(lower("issues"."title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  lower("users"."username" COLLATE "C") = lower('bar' COLLATE "C");
```

### Globs on two links, one of them a self-reference

> Every column of each issue's author and of the issue it duplicates

```qd
#issues $author.* $duplicate_of.*
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen",
  "users"."preferences",
  "issues_1"."id",
  "issues_1"."title",
  "issues_1"."description",
  "issues_1"."created_at",
  "issues_1"."author",
  "issues_1"."status",
  "issues_1"."project",
  "issues_1"."duplicate_of",
  "issues_1"."due_date"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "issues" AS "issues_1" ON
  "issues"."duplicate_of" = "issues_1"."id";
```

### Two links that each lead further

> The joins form a tree: the author's team, and the project's product and its client

```qd
#issues $author.team.name $project.product.name $author.username $project.product.client.name
```

```sql
SELECT
  "teams"."name",
  "products"."name",
  "users"."username",
  "clients"."name"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id"
LEFT JOIN "clients" ON
  "products"."client" = "clients"."id";
```

### Two links to the same table

> Each link is aliased by its column, since the table's name wouldn't tell them apart

```qd
#blocks $blocker.title $blocking.title
```

```sql
SELECT
  "blocker"."title",
  "blocking"."title"
FROM "blocks"
LEFT JOIN "issues" AS "blocker" ON
  "blocks"."blocker" = "blocker"."id"
LEFT JOIN "issues" AS "blocking" ON
  "blocks"."blocking" = "blocking"."id";
```

### Two links to the same table, each leading further

> Blocks in which an issue opened by alice blocks an issue of the website project

```qd
#blocks blocker.author.username:=alice blocking.project.name:=website $id $blocker.project.name
```

```sql
SELECT
  "blocks"."id",
  "projects_1"."name"
FROM "blocks"
LEFT JOIN "issues" AS "blocker" ON
  "blocks"."blocker" = "blocker"."id"
LEFT JOIN "users" ON
  "blocker"."author" = "users"."id"
LEFT JOIN "projects" AS "projects_1" ON
  "blocker"."project" = "projects_1"."id"
LEFT JOIN "issues" AS "blocking" ON
  "blocks"."blocking" = "blocking"."id"
LEFT JOIN "projects" ON
  "blocking"."project" = "projects"."id"
WHERE
  lower("users"."username" COLLATE "C") = lower('alice' COLLATE "C") AND
  lower("projects"."name" COLLATE "C") = lower('website' COLLATE "C");
```

### Two links within a filter of related records

> For each project, the number of duplicate issues that alice opened whose original is closed

```qd
#projects $name $#issues{author.username:=alice duplicate_of.status:=closed}
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    LEFT JOIN "issues" AS "issues_1" ON
      "issues"."duplicate_of" = "issues_1"."id"
    WHERE
      lower("users"."username" COLLATE "C") = lower('alice' COLLATE "C") AND
      lower("issues_1"."status" COLLATE "C") = lower('closed' COLLATE "C")
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0)
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Standalone sorts through two links

> Issues by project and then by author, showing neither

```qd
#issues \\project.name \\author.username \d $title
```

```sql
SELECT
  "issues"."title"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
ORDER BY
  "projects"."name" ASC NULLS LAST,
  "users"."username" DESC NULLS LAST;
```

### Standalone sort through one link, with a result column through another

> The sorting is compiled first, so its join comes first

```qd
#issues \\author.username $project.name $title
```

```sql
SELECT
  "projects"."name",
  "issues"."title"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
ORDER BY
  "users"."username" ASC NULLS LAST;
```

### Grouping by columns of two linked tables

> The number of issues that each author has opened in each project

```qd
#issues $project.name \g $author.username \g $%count \sd
```

```sql
SELECT
  "projects"."name",
  "users"."username",
  count(*)
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
GROUP BY "projects"."name", "users"."username"
ORDER BY
  count(*) DESC NULLS LAST;
```

### Aggregates of two linked tables in a grouped query

> For each status, the latest author to have been seen, and the number of distinct products

```qd
#issues $status \g $author.last_seen%max $project.product%distinct
```

```sql
SELECT
  "issues"."status",
  max("users"."last_seen"),
  count(DISTINCT "projects"."product")
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
GROUP BY "issues"."status";
```

### Counts of related records through two links

> For each issue, the number of comments its author has made and the number of issues in its project: each CTE is joined to the table its link leads to

```qd
#issues $id $author.#comments $project.#issues
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk",
      count(*) AS "v1"
    FROM "comments"
    GROUP BY "comments"."user"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "issues"."id",
  COALESCE("cte0"."v1", 0),
  COALESCE("cte1"."v1", 0)
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

### Has conditions through two links

> Issues whose author has commented somewhere and whose project has an open issue

```qd
#issues ++author.#comments ++project.#issues{status:=open} $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk"
    FROM "comments"
    GROUP BY "comments"."user"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    WHERE
      lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C")
    GROUP BY "issues"."project"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NOT NULL;
```

### List sorted through two links of the related table

> For each project, its issue titles ordered by author and then by the creation date of the issue each duplicates

```qd
#projects $name $#issues.title%list_all(\\author.username \\duplicate_of.created_at \d)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      array_agg("issues"."title" ORDER BY "users"."username" ASC NULLS LAST, "issues_1"."created_at" DESC NULLS LAST) AS "v1"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    LEFT JOIN "issues" AS "issues_1" ON
      "issues"."duplicate_of" = "issues_1"."id"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte0"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk";
```

### Nested result columns through two links

> For each comment, its issue's author and project, and its own author

```qd
#comments $id $issue.( $author.username $project.name ) $user.username
```

```sql
SELECT
  "comments"."id",
  "users"."username",
  "projects"."name",
  "users_1"."username"
FROM "comments"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "users" AS "users_1" ON
  "comments"."user" = "users_1"."id";
```

### Format string and case expression over two links

> The two linked tables are joined once each, however many times they're referred to

```qd
#issues
$^f"{author.username} / {project.name}"
$? project.is_active ~ author.username ~~ project.name
```

```sql
SELECT
  concat("users"."username", ' / ', "projects"."name"),
  CASE
    WHEN "projects"."is_active" THEN "users"."username"
    ELSE "projects"."name"
  END
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id";
```

### Computed columns hosted on two linked tables

> Each computed column is evaluated against the table its link leads to

```qd
#users.handle = ^f"@{username}"
#projects.label = ^f"{name} ({product.name})"
#issues $author.handle $project.label
```

```sql
SELECT
  concat('@', "users"."username"),
  concat("projects"."name", ' (', "products"."name", ')')
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id";
```

### Linked record comparisons through two links

> Each link uses the comparison of the table it leads to

```qd
#users.__querydown_linked_record_comparison:@x = username:@x
#projects.__querydown_linked_record_comparison:@x = name:@x
#issues author:alice project:=website $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  COALESCE(strpos(lower("users"."username" COLLATE "C"), lower('alice' COLLATE "C")) > 0, FALSE) AND
  lower("projects"."name" COLLATE "C") = lower('website' COLLATE "C");
```

### Custom comparison whose body follows two links

> Issues involving "acme", by its author's team or its project's product

```qd
#issues.involves:@x = [author.team.name:@x project.product.name:@x]
#issues involves:acme !involves:=internal $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "teams" ON
  "users"."team" = "teams"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "products" ON
  "projects"."product" = "products"."id"
WHERE
  (COALESCE(strpos(lower("teams"."name" COLLATE "C"), lower('acme' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("products"."name" COLLATE "C"), lower('acme' COLLATE "C")) > 0, FALSE)) AND
  (lower("teams"."name" COLLATE "C") = lower('internal' COLLATE "C") OR lower("products"."name" COLLATE "C") = lower('internal' COLLATE "C")) IS NOT TRUE;
```

### Window partitioned by one link and ordered by another

> Number each project's issues by the name of their author

```qd
#issues $id $%%(project.name\p author.username\s)%row_number
```

```sql
SELECT
  "issues"."id",
  row_number() OVER (PARTITION BY "projects"."name" ORDER BY "users"."username" ASC NULLS LAST)
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id";
```

### Window condition with result columns through two links

> The most recent issue of each author's team, with its project and author: the window is computed over the join to the author, and the result columns join again outside it

```qd
#issues %%(author.team\p created_at\sd)%row_number:1 $project.name $author.username
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
      row_number() OVER (PARTITION BY "users"."team" ORDER BY "issues"."created_at" DESC NULLS LAST) AS "qdwin0"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
  )
SELECT
  "projects"."name",
  "users"."username"
FROM "qdwin_src" AS "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  "issues"."qdwin0" = 1;
```

### Union whose members each follow two links

> Each member has joins of its own

```qd
#issues $author.username $project.name +++ #comments $user.username $issue.title
```

```sql
SELECT
  "users"."username",
  "projects"."name" AS "title"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
UNION ALL
SELECT
  "users"."username",
  "issues"."title"
FROM "comments"
LEFT JOIN "users" ON
  "comments"."user" = "users"."id"
LEFT JOIN "issues" ON
  "comments"."issue" = "issues"."id";
```

### Two links in the first stage of a pipeline

> The number of issues that each author has opened in each project, for the pairs with more than one

```qd
#issues $author.username -> author \g $project.name -> project \g $%count -> n
~~~
n:>1 $author $project
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "users"."username" AS "author",
      "projects"."name" AS "project",
      count(*) AS "n"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    LEFT JOIN "projects" ON
      "issues"."project" = "projects"."id"
    GROUP BY "users"."username", "projects"."name"
  )
SELECT
  "pipe0"."author",
  "pipe0"."project"
FROM "pipe0"
WHERE
  "pipe0"."n" > 1;
```

### Two links through flexible identifiers (library)

```toml options
schema = "library"
```

> Checkouts of books by a given patron, with the item's barcode and the patron's email

```qd
#checkouts patron.last_name:=smith $item.barcode $patron.email $item.book.title
```

```sql
SELECT
  "Items"."Barcode",
  "Patrons"."Email",
  "Books"."Title"
FROM "Checkouts"
LEFT JOIN "Patrons" ON
  "Checkouts"."Patron" = "Patrons"."id"
LEFT JOIN "Items" ON
  "Checkouts"."Item" = "Items"."id"
LEFT JOIN "Books" ON
  "Items"."Book" = "Books"."id"
WHERE
  lower("Patrons"."Last Name" COLLATE "C") = lower('smith' COLLATE "C");
```

### Scopes, a glob, and a sort through two links (DuckDB)

```toml options
dialect = "duckdb"
```

> The same join order under the other dialect

```qd
#issues author{username:alice} project{is_active:@true} \\project.name $author.* $title
```

```sql
SELECT
  "users"."id",
  "users"."username",
  "users"."email",
  "users"."team",
  "users"."birth_date",
  "users"."last_seen",
  "users"."preferences",
  "issues"."title"
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  COALESCE(contains(lower(strip_accents("users"."username")), lower(strip_accents('alice'))), FALSE) AND
  "projects"."is_active" = TRUE
ORDER BY
  "projects"."name" ASC NULLS LAST;
```

## Combinations: dialects, schemas, and options

These cases run combinations of features under the DuckDB dialect, against the library schema (whose identifiers need quoting), and with the `limit` and `offset` options.

### Search terms, expansion, equality operators, and durations (DuckDB)

```toml options
dialect = "duckdb"
```

> Text matching is accent-insensitive in DuckDB

```qd
#issues foo,!bar status:[open reopened] !title:=x description:==y created_at:<1w due_date:1d..1w $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  (COALESCE(contains(lower(strip_accents("issues"."title")), lower(strip_accents('foo'))), FALSE) OR COALESCE(contains(lower(strip_accents("issues"."description")), lower(strip_accents('foo'))), FALSE) OR COALESCE(contains(lower(strip_accents("issues"."status")), lower(strip_accents('foo'))), FALSE) OR NOT (COALESCE(contains(lower(strip_accents("issues"."title")), lower(strip_accents('bar'))), FALSE) OR COALESCE(contains(lower(strip_accents("issues"."description")), lower(strip_accents('bar'))), FALSE) OR COALESCE(contains(lower(strip_accents("issues"."status")), lower(strip_accents('bar'))), FALSE))) AND
  (COALESCE(contains(lower(strip_accents("issues"."status")), lower(strip_accents('open'))), FALSE) OR COALESCE(contains(lower(strip_accents("issues"."status")), lower(strip_accents('reopened'))), FALSE)) AND
  (lower(strip_accents("issues"."title")) = lower(strip_accents('x'))) IS NOT TRUE AND
  "issues"."description" = 'y' AND
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at")) < EXTRACT(epoch FROM to_weeks(1)) AND
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."due_date")) >= EXTRACT(epoch FROM to_days(1)) AND
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."due_date")) <= EXTRACT(epoch FROM to_weeks(1));
```

### Zoned, naive, and date columns compared with durations, dates, and now (DuckDB)

```toml options
dialect = "duckdb"
```

> The zoned column is cast so that it can be compared with naive values

```qd
#users last_seen:<1h last_seen:>@2024-01-01 last_seen:<@now|minus(1d) birth_date:>18y $id $last_seen|age|days $birth_date|age|years|floor
```

```sql
SELECT
  "users"."id",
  EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - CAST("users"."last_seen" AS TIMESTAMP)) / 86400,
  FLOOR(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "users"."birth_date") / 31557600)
FROM "users"
WHERE
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - CAST("users"."last_seen" AS TIMESTAMP))) < EXTRACT(epoch FROM to_hours(1)) AND
  CAST("users"."last_seen" AS TIMESTAMP) > DATE '2024-01-01' AND
  CAST("users"."last_seen" AS TIMESTAMP) < CAST(NOW() AS TIMESTAMP) - to_days(1) AND
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "users"."birth_date")) > EXTRACT(epoch FROM to_years(18));
```

### Aggregate of related records compared with a zoned column (DuckDB)

```toml options
dialect = "duckdb"
```

> Users whose latest comment precedes their last visit

```qd
#users #comments.created_at%max:<`last_seen` $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."user" AS "pk",
      max("comments"."created_at") AS "v1"
    FROM "comments"
    GROUP BY "comments"."user"
  )
SELECT
  "users"."id"
FROM "users"
LEFT JOIN "cte0" ON
  "users"."id" = "cte0"."pk"
WHERE
  "cte0"."v1" < CAST("users"."last_seen" AS TIMESTAMP);
```

### Format string with date functions, and hashes (DuckDB)

```toml options
dialect = "duckdb"
```

> Several date functions interpolated into one string

```qd
#users $^f"{username} seen {last_seen|age|days|floor} days ago; born {birth_date|year}-{birth_date|month}-{birth_date|day}" $email|md5 $email|unit_hash
```

```sql
SELECT
  concat("users"."username", ' seen ', FLOOR(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - CAST("users"."last_seen" AS TIMESTAMP)) / 86400), ' days ago; born ', CAST(EXTRACT(year FROM "users"."birth_date") AS INTEGER), '-', CAST(EXTRACT(month FROM "users"."birth_date") AS INTEGER), '-', CAST(EXTRACT(day FROM "users"."birth_date") AS INTEGER)),
  md5("users"."email"),
  CAST(hash("users"."email") AS DOUBLE) / 18446744073709551615.0
FROM "users";
```

### Sorted list, top, and list-all aggregates (DuckDB)

```toml options
dialect = "duckdb"
```

> Each aggregate has its own kind of sorting

```qd
#issues $id $#labels.name%list(\sd) $#comments.user.username%top(\\%count \d) $#comments.body%list_all(\\created_at)
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
  ),
  "cte1" AS (
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
  ),
  "cte2" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg("comments"."body" ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id",
  "cte0"."v1",
  "cte1"."v1",
  "cte2"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "issues"."id" = "cte2"."pk";
```

### Window condition with flagged regex conditions (DuckDB)

```toml options
dialect = "duckdb"
```

> Regex flags are rendered as RE2 options

```qd
#issues %%(project\p created_at\sd)%row_number:1 title:\cm~'^Fix' !description:\s~'a.b' $id
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
      regexp_matches("issues"."title", '(?m)^Fix') AND
      regexp_matches("issues"."description", '(?s)a.b', 'i') IS NOT TRUE
  )
SELECT
  "issues"."id"
FROM "qdwin_src" AS "issues"
WHERE
  "issues"."qdwin0" = 1;
```

### Grouping by a date function with list aggregates (DuckDB)

```toml options
dialect = "duckdb"
```

> The grouping expression is repeated in the `GROUP BY`

```qd
#issues $status \g $created_at|year_month \g $%count $id%list(\sd) $id%list_all(\\created_at \d)
```

```sql
SELECT
  "issues"."status",
  strftime("issues"."created_at", '%Y-%m'),
  count(*),
  array_agg(DISTINCT "issues"."id" ORDER BY "issues"."id" DESC NULLS LAST),
  array_agg("issues"."id" ORDER BY "issues"."created_at" DESC NULLS LAST)
FROM "issues"
GROUP BY "issues"."status", strftime("issues"."created_at", '%Y-%m');
```

### Union with a filtered has condition, sorted in a pipeline stage (DuckDB)

```toml options
dialect = "duckdb"
```

> The later member's CTE is scoped to it with parentheses

```qd
#issues $id +++ #projects ++#issues{created_at:<1w} $id ~~~ \\id \d
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."id" AS "id"
    FROM "issues"
    UNION ALL
    (
      WITH
        "cte0" AS (
          SELECT
            "issues"."project" AS "pk"
          FROM "issues"
          WHERE
            ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at")) < EXTRACT(epoch FROM to_weeks(1))
          GROUP BY "issues"."project"
        )
      SELECT
        "projects"."id"
      FROM "projects"
      LEFT JOIN "cte0" ON
        "projects"."id" = "cte0"."pk"
      WHERE
        "cte0"."pk" IS NOT NULL
    )
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
ORDER BY
  "pipe0"."id" DESC NULLS LAST;
```

### Computed columns with a duration comparison and a format string (DuckDB)

```toml options
dialect = "duckdb"
```

> The computed duration comparison is used as a condition and as a case test

```qd
#issues.stale = created_at:>6m
#users.display = ^f"{username} <{email}>"
#issues `stale` $author.display $ ? `stale` ~ "stale" ~~ "fresh"
```

```sql
SELECT
  concat("users"."username", ' <', "users"."email", '>'),
  CASE
    WHEN ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at")) > EXTRACT(epoch FROM to_months(6)) THEN 'stale'
    ELSE 'fresh'
  END
FROM "issues"
LEFT JOIN "users" ON
  "issues"."author" = "users"."id"
WHERE
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at")) > EXTRACT(epoch FROM to_months(6));
```

### Flagged strings in comparisons (DuckDB)

```toml options
dialect = "duckdb"
```

> Escapes and interpolation within text comparisons

```qd
#issues title:=^e'it\'s\n' description:^f"{title}{{}}" $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  lower(strip_accents("issues"."title")) = lower(strip_accents('it''s
')) AND
  COALESCE(contains(lower(strip_accents("issues"."description")), lower(strip_accents(concat("issues"."title", '{}')))), FALSE);
```

### Boolean expansion, negation, `xor`, and numeric aggregates (DuckDB)

```toml options
dialect = "duckdb"
```

> Boolean logic mixed with aggregates of related records

```qd
#projects {is_pro_bono is_active}:@false !`is_active` is_active|xor(#issues:0) $name $#issues.id%product $#issues.id%avg
```

```sql
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      product("issues"."id") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "cte2" AS (
    SELECT
      "issues"."project" AS "pk",
      avg("issues"."id") AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  "cte1"."v1",
  "cte2"."v1"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk"
LEFT JOIN "cte2" ON
  "projects"."id" = "cte2"."pk"
WHERE
  "projects"."is_pro_bono" = FALSE AND
  "projects"."is_active" = FALSE AND
  "projects"."is_active" IS NOT TRUE AND
  "projects"."is_active" <> ("cte0"."pk" IS NULL);
```

### Date functions in ranges and comparisons (DuckDB)

```toml options
dialect = "duckdb"
```

> A year range, a month range with an exclusive end, and a day

```qd
#issues created_at|year:2010..2019 created_at|month:1..<4 created_at|day:15 $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
WHERE
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) >= 2010 AND
  CAST(EXTRACT(year FROM "issues"."created_at") AS INTEGER) <= 2019 AND
  CAST(EXTRACT(month FROM "issues"."created_at") AS INTEGER) >= 1 AND
  CAST(EXTRACT(month FROM "issues"."created_at") AS INTEGER) < 4 AND
  CAST(EXTRACT(day FROM "issues"."created_at") AS INTEGER) = 15;
```

### Durations in each unit of measure (DuckDB)

```toml options
dialect = "duckdb"
```

> The time until the due date and since creation

```qd
#issues $due_date|countdown|hours|ceil $created_at|age|minutes $created_at|age|seconds $due_date|countdown|years
```

```sql
SELECT
  CEIL(EXTRACT(epoch FROM "issues"."due_date" - CAST(NOW() AS TIMESTAMP)) / 3600),
  EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at") / 60,
  EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at"),
  EXTRACT(epoch FROM "issues"."due_date" - CAST(NOW() AS TIMESTAMP)) / 31557600
FROM "issues";
```

### Window functions with a lagged difference (DuckDB)

```toml options
dialect = "duckdb"
```

> The time elapsed since the previous issue in the project

```qd
#issues $%%(project\p created_at\s)%lag(created_at) $created_at - %%(project\p created_at\s)%lag(created_at) -> gap
```

```sql
SELECT
  lag("issues"."created_at") OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST),
  "issues"."created_at" - lag("issues"."created_at") OVER (PARTITION BY "issues"."project" ORDER BY "issues"."created_at" ASC NULLS LAST) AS "gap"
FROM "issues";
```

### Quoted identifiers in a condition, a sorted alias, and a count (library)

```toml options
schema = "library"
```

> Backtick-quoted names with spaces

```qd
#Patrons `First Name`:foo $`Last Name` -> surname \s $#Checkouts
```

```sql
WITH
  "cte0" AS (
    SELECT
      "Checkouts"."Patron" AS "pk",
      count(*) AS "v1"
    FROM "Checkouts"
    GROUP BY "Checkouts"."Patron"
  )
SELECT
  "Patrons"."Last Name" AS "surname",
  COALESCE("cte0"."v1", 0)
FROM "Patrons"
LEFT JOIN "cte0" ON
  "Patrons"."id" = "cte0"."pk"
WHERE
  COALESCE(strpos(lower("Patrons"."First Name" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
ORDER BY
  "surname" ASC NULLS LAST;
```

### Flexible identifiers with an expansion, a negation, and a function pipeline (library)

```toml options
schema = "library"
```

> Snake-case names resolve to the names with spaces

```qd
#patrons first_name:[foo bar] !last_name:=baz $first_name|concat(" ")|concat(last_name) -> full_name \s
```

```sql
SELECT
  concat(concat("Patrons"."First Name", ' '), "Patrons"."Last Name") AS "full_name"
FROM "Patrons"
WHERE
  (COALESCE(strpos(lower("Patrons"."First Name" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("Patrons"."First Name" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE)) AND
  (lower("Patrons"."Last Name" COLLATE "C") = lower('baz' COLLATE "C")) IS NOT TRUE
ORDER BY
  "full_name" ASC NULLS LAST;
```

### Flexible and quoted identifiers within a format string, and a filtered list through two to-one hops (library)

```toml options
schema = "library"
```

> The titles of the books each patron has checked out and not returned

```qd
#patrons $^f"{first_name} {`Last Name`}" -> name $#checkouts{check_in_time:@null}.item.book.title%list(\s)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "Checkouts"."Patron" AS "pk",
      array_agg(DISTINCT "Books"."Title" ORDER BY "Books"."Title" ASC NULLS LAST) AS "v1"
    FROM "Checkouts"
    JOIN "Items" ON
      "Checkouts"."Item" = "Items"."id"
    JOIN "Books" ON
      "Items"."Book" = "Books"."id"
    WHERE
      "Checkouts"."Check In Time" IS NULL
    GROUP BY "Checkouts"."Patron"
  )
SELECT
  concat("Patrons"."First Name", ' ', "Patrons"."Last Name") AS "name",
  "cte0"."v1"
FROM "Patrons"
LEFT JOIN "cte0" ON
  "Patrons"."id" = "cte0"."pk";
```

### Scoped comparisons with flexible identifiers (library)

```toml options
schema = "library"
```

> A scope nested within a scope

```qd
#checkouts item{barcode:foo book{page_count:>200 title:bar}} $id
```

```sql
SELECT
  "Checkouts"."id"
FROM "Checkouts"
LEFT JOIN "Items" ON
  "Checkouts"."Item" = "Items"."id"
LEFT JOIN "Books" ON
  "Items"."Book" = "Books"."id"
WHERE
  COALESCE(strpos(lower("Items"."Barcode" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE) AND
  "Books"."Page Count" > 200 AND
  COALESCE(strpos(lower("Books"."Title" COLLATE "C"), lower('bar' COLLATE "C")) > 0, FALSE);
```

### Date column compared with a duration, grouped by a to-one column (library)

```toml options
schema = "library"
```

> A `date` column on the left of a duration comparison

```qd
#checkouts due_date:<1w `Checkout Time`:>@now|minus(1m) $patron.email \g $%count \sd
```

```sql
SELECT
  "Patrons"."Email",
  count(*)
FROM "Checkouts"
LEFT JOIN "Patrons" ON
  "Checkouts"."Patron" = "Patrons"."id"
WHERE
  ABS(EXTRACT(epoch FROM NOW() - "Checkouts"."Due Date")) < EXTRACT(epoch FROM make_interval(weeks => 1)) AND
  "Checkouts"."Checkout Time" > NOW() - make_interval(months => 1)
GROUP BY "Patrons"."Email"
ORDER BY
  count(*) DESC NULLS LAST;
```

### Computed column and configured default text search through a scope (library)

```toml options
schema = "library"
```

> Searching a checkout's patron searches the patron's full name and email

```qd
#patrons.full_name = ^f"{first_name} {last_name}"
#patrons.__querydown_default_text_search:@x = [full_name:@x email:@x]
#checkouts patron{smith} $patron.full_name
```

```sql
SELECT
  concat("Patrons"."First Name", ' ', "Patrons"."Last Name")
FROM "Checkouts"
LEFT JOIN "Patrons" ON
  "Checkouts"."Patron" = "Patrons"."id"
WHERE
  (COALESCE(strpos(lower(concat("Patrons"."First Name", ' ', "Patrons"."Last Name") COLLATE "C"), lower('smith' COLLATE "C")) > 0, FALSE) OR COALESCE(strpos(lower("Patrons"."Email" COLLATE "C"), lower('smith' COLLATE "C")) > 0, FALSE));
```

### Standalone sorts with a hidden glob column and a related glob (library)

```toml options
schema = "library"
```

> Every quoted column of the related table is listed

```qd
#items \\book.title \\#checkouts \d $*(book \h) $book.*
```

```sql
WITH
  "cte0" AS (
    SELECT
      "Checkouts"."Item" AS "pk",
      count(*) AS "v1"
    FROM "Checkouts"
    GROUP BY "Checkouts"."Item"
  )
SELECT
  "Items"."id",
  "Items"."Barcode",
  "Items"."Acquisition Date",
  "Items"."Acquisition Price",
  "Books"."id",
  "Books"."Title",
  "Books"."Publication Year",
  "Books"."Media",
  "Books"."Page Count",
  "Books"."LC Classification",
  "Books"."ISBN",
  "Books"."Dewey Decimal",
  "Books"."Dewey Wording",
  "Books"."Author",
  "Books"."Publisher"
FROM "Items"
LEFT JOIN "cte0" ON
  "Items"."id" = "cte0"."pk"
LEFT JOIN "Books" ON
  "Items"."Book" = "Books"."id"
ORDER BY
  "Books"."Title" ASC NULLS LAST,
  COALESCE("cte0"."v1", 0) DESC NULLS LAST;
```

### Aliases with spaces flowing through a pipeline (library)

```toml options
schema = "library"
```

> The next stage refers to the aliases flexibly

```qd
#books title:foo $title -> `Book Title` $page_count -> `Page Count` ~~~ page_count:>100 $book_title
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "Books"."Title" AS "Book Title",
      "Books"."Page Count" AS "Page Count"
    FROM "Books"
    WHERE
      COALESCE(strpos(lower("Books"."Title" COLLATE "C"), lower('foo' COLLATE "C")) > 0, FALSE)
  )
SELECT
  "pipe0"."Book Title"
FROM "pipe0"
WHERE
  "pipe0"."Page Count" > 100;
```

### Window condition with a null check and a to-one result column (library)

```toml options
schema = "library"
```

> The latest unreturned checkout of each item

```qd
#checkouts %%(item\p checkout_time\sd)%row_number:1 check_in_time:@null $item.book.title
```

```sql
WITH
  "qdwin_src" AS (
    SELECT
      "Checkouts"."id",
      "Checkouts"."Item",
      "Checkouts"."Patron",
      "Checkouts"."Checkout Time",
      "Checkouts"."Due Date",
      "Checkouts"."Check In Time",
      row_number() OVER (PARTITION BY "Checkouts"."Item" ORDER BY "Checkouts"."Checkout Time" DESC NULLS LAST) AS "qdwin0"
    FROM "Checkouts"
    WHERE
      "Checkouts"."Check In Time" IS NULL
  )
SELECT
  "Books"."Title"
FROM "qdwin_src" AS "Checkouts"
LEFT JOIN "Items" ON
  "Checkouts"."Item" = "Items"."id"
LEFT JOIN "Books" ON
  "Items"."Book" = "Books"."id"
WHERE
  "Checkouts"."qdwin0" = 1;
```

### Limit with a sorted, annotated column and a query annotation

```toml options
limit = 3
```

> Annotations are unaffected by the limit

```qd
#issues $id \s @{a:1} \\\{page:1}
```

```sql
SELECT
  "issues"."id"
FROM "issues"
ORDER BY
  "issues"."id" ASC NULLS LAST
LIMIT 3;
```

```json
{
  "columnAnnotations": [
    {
      "a": 1
    }
  ],
  "queryAnnotation": {
    "page": 1
  }
}
```

### Limit on the last stage of a grouped pipeline

```toml options
limit = 3
```

> The grouped stage isn't limited

```qd
#issues $status \g $%count ~~~ \\status
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "issues"."status" AS "status",
      count(*) AS "column_2"
    FROM "issues"
    GROUP BY "issues"."status"
  )
SELECT
  "pipe0"."status",
  "pipe0"."column_2"
FROM "pipe0"
ORDER BY
  "pipe0"."status" ASC NULLS LAST
LIMIT 3;
```

### Limit with list aggregates of related records

```toml options
limit = 3
```

> The limit counts issues, not the related records within the CTEs

```qd
#issues $id $#comments.body%list_all(\\created_at) $#labels.name%top(\\%count \d)
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk",
      array_agg("comments"."body" ORDER BY "comments"."created_at" ASC NULLS LAST) AS "v1"
    FROM "comments"
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    WITH
      "distinct_values" AS (
        SELECT
          "issue_labels"."issue" AS "pk",
          "labels"."name" AS "value",
          rank() OVER (PARTITION BY "issue_labels"."issue" ORDER BY count(*) DESC NULLS LAST) AS "rank"
        FROM "issue_labels"
        JOIN "labels" ON
          "issue_labels"."label" = "labels"."id"
        GROUP BY "issue_labels"."issue", "labels"."name"
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
  "cte0"."v1",
  "cte1"."v1"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
LIMIT 3;
```

### Offset on a union without a pipeline

```toml options
offset = 5
```

> The offset follows the last member and applies to the whole union

```qd
#issues $id +++ #projects $id
```

```sql
SELECT
  "issues"."id"
FROM "issues"
UNION ALL
SELECT
  "projects"."id"
FROM "projects"
OFFSET 5;
```

### Date column of an earlier stage compared with a duration (DuckDB)

```toml options
dialect = "duckdb"
```

> The zoned column is still known to be zoned in the later stage, where it's coerced as it would be in the first

```qd
#users $id $last_seen $birth_date|age -> age
~~~
last_seen:<1w age:>18y $id
```

```sql
WITH
  "pipe0" AS (
    SELECT
      "users"."id" AS "id",
      "users"."last_seen" AS "last_seen",
      CAST(NOW() AS TIMESTAMP) - "users"."birth_date" AS "age"
    FROM "users"
  )
SELECT
  "pipe0"."id"
FROM "pipe0"
WHERE
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - CAST("pipe0"."last_seen" AS TIMESTAMP))) < EXTRACT(epoch FROM to_weeks(1)) AND
  "pipe0"."age" > to_years(18);
```

### Arguments that reference columns, used within filters of related records (DuckDB)

```toml options
dialect = "duckdb"
```

> The record is joined into the CTE in the same way under the other dialect

```qd
#issues.comment:@x = ++#comments{body:@x}
@@has_comment_by = @u => ++#comments{user:@u}
#issues comment:`title` author|has_comment_by $id
```

```sql
WITH
  "cte0" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    WHERE
      COALESCE(contains(lower(strip_accents("comments"."body")), lower(strip_accents("issues"."title"))), FALSE)
    GROUP BY "comments"."issue"
  ),
  "cte1" AS (
    SELECT
      "comments"."issue" AS "pk"
    FROM "comments"
    LEFT JOIN "issues" ON
      "comments"."issue" = "issues"."id"
    WHERE
      "comments"."user" = "issues"."author"
    GROUP BY "comments"."issue"
  )
SELECT
  "issues"."id"
FROM "issues"
LEFT JOIN "cte0" ON
  "issues"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "issues"."id" = "cte1"."pk"
WHERE
  "cte0"."pk" IS NOT NULL AND
  "cte1"."pk" IS NOT NULL;
```

### Parenthesized right operands, foreign key aggregates, and a computed duration (DuckDB)

```toml options
dialect = "duckdb"
```

> Fixes to the generated SQL that don't depend on the dialect

```qd
#issues created_at:<1w + 1d $status \g $author%distinct $(id%max - (id%min + 1))
```

```sql
SELECT
  "issues"."status",
  count(DISTINCT "issues"."author"),
  max("issues"."id") - (min("issues"."id") + 1)
FROM "issues"
WHERE
  ABS(EXTRACT(epoch FROM CAST(NOW() AS TIMESTAMP) - "issues"."created_at")) < EXTRACT(epoch FROM to_weeks(1) + to_days(1))
GROUP BY "issues"."status";
```
