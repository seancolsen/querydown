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
issues $id->id
```

```sql
SELECT "issues"."id" AS "id" FROM "issues";
```

## Simple library schema

```toml options
schema = "library"
```

> Show all patrons

```qd
Patrons
```

```sql
SELECT "Patrons".* FROM "Patrons";
```

## Large examples

### ⛔ Main README

Not yet working

```qd
issues
created_at:>@6M|ago
--#assignments
++#labels{name:["Regression" "Bug"]}
#comments{user.team.name!"Backend"}:~10..20
$[]
$author.username
$#comments.created_at%min \sd
```

## Flexible identifiers

```toml options
schema = "library"
```

### Simplest flexible identifier

> All checkouts

```qd
checkouts
```

```sql
SELECT
  "Checkouts".*
FROM "Checkouts";
```

### snake_case

> Checkouts from over one month ago and not yet returned

```qd
checkouts check_in_time:@null check_out_time:<@1M|ago
```

```sql
SELECT
  "Checkouts".*
FROM "Checkouts"
WHERE
  "Checkouts"."Check In Time" = NULL AND
  "Checkouts"."Checkout Time" < (NOW() - INTERVAL '1M');
```

### camelCase

> Checkouts from over one month ago and not yet returned

```qd
checkouts checkInTime:@null checkOutTime:<@1M|ago
```

```sql
SELECT
  "Checkouts".*
FROM "Checkouts"
WHERE
  "Checkouts"."Check In Time" = NULL AND
  "Checkouts"."Checkout Time" < (NOW() - INTERVAL '1M');
```

### Complex flexible identifiers

```qd
items
++#checkouts{check_in_time:@null patron.first_name:"Foo"}
book.page_count:>200
```

```sql
WITH "cte0" AS (
  SELECT
    "Checkouts"."Item" AS "pk"
  FROM "Checkouts"
  WHERE
    "Checkouts"."Check In Time" = NULL AND
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
issues created_at:>=@2023-01-01
```

```sql
SELECT
  "issues".*
FROM "issues"
WHERE
  "issues"."created_at" >= DATE '2023-01-01';
```

## Condition sets

### "Has some" with "OR"

This test is part of a bug fix. Previously, we were using `JOIN` instead of `LEFT JOIN` when joining "has some" related tables because that produced simpler SQL. But that didn't work when the condition was nested inside an `OR` condition set. We use `LEFT JOIN` plus a `WHERE` condition because it seems less prone to bugs.

> Issues that have labels or comments

```qd
issues [++#labels ++#comments]
```

```sql
WITH "cte0" AS (
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
  "cte0"."pk" IS NOT NULL OR
  "cte1"."pk" IS NOT NULL;
```

## Paths to one

### Joined column in related table

> Issues under project named "foo".

```qd
issues project.name:"foo" $id->id
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

### ⛔ Referenced column in related table not joined

This test case ensures that we don't have an unnecessary join on `projects` when the `projects.id` value can already be found within `issues.project`.

**TODO** This is not yet implemented. We need to make some changes within `build_linked_path` to optimize for this case. The SQL we're producing still works even though this test case is not satisfied. We're just producing SQL that has a superfluous join.

> Issues under project with id 1.


```qd
issues project.id:1 $id->id
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
issues $id $#comments.created_at%max->most_recent_comment
```

```sql
WITH "cte0" AS (
  SELECT
    "comments"."issue" AS "pk",
    MAX("comments"."created_at") AS "v1"
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
issues $id->id $author.#comments->total_comments_by_author
```

```sql
WITH "cte0" AS (
  SELECT
    "comments"."user" AS "pk",
    COUNT(*) AS "v1"
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
users $id->id $#issues.#comments.created_at%max->v
```

```sql
WITH "cte0" AS (
  SELECT
    "issues"."author" AS "pk",
    MAX("comments"."created_at") AS "v1"
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
projects $id->id $#issues.author.#comments.created_at%max->v
```

```sql
WITH "cte0" AS (
  SELECT
    "issues"."project" AS "pk",
    MAX("comments"."created_at") AS "v1"
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

### Multiple CTEs

> Issues that have comments and assignments

```qd
issues ++#comments ++#assignments
```

```sql
WITH "cte0" AS (
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
issues ++#comments
```

```sql
WITH "cte0" AS (
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
users --#issues
```

```sql
WITH "cte0" AS (
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
users --#issues.#comments
```

```sql
WITH "cte0" AS (
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
users ++#issues.#comments
```

```sql
WITH "cte0" AS (
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
issues ++#labels
```

```sql
TODO
```


## Filtered paths

### Simple filtered path in has none

> Users who have not created any issues in the past year

```qd
users --#issues{created_at:>@1Y|ago}
```

```sql
WITH "cte0" AS (
  SELECT
    "issues"."author" AS "pk"
  FROM "issues"
  WHERE
    "issues"."created_at" > (NOW() - INTERVAL '1Y')
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
users $#issues{created_at:>@1Y|ago}
```

```sql
WITH "cte0" AS (
  SELECT
    "issues"."author" AS "pk",
    COUNT(*) AS "v1"
  FROM "issues"
  WHERE
    "issues"."created_at" > (NOW() - INTERVAL '1Y')
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
issues --#labels{name:"bug"} $id
```

```sql
WITH "cte0" AS (
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
issues $#comments{user:issue.author}
```

```sql
TODO
```

### ⛔Nested filter

> Clients that don't have any issues without comments

```qd
clients --#issues{--#comments}
```

```sql
TODO
```

## Column control flags

### Basic sort

> Issues, showing the most recent ones first

```qd
issues $id $title $created_at \sd
```

```sql
SELECT
  "issues"."id",
  "issues"."title",
  "issues"."created_at"
FROM "issues"
ORDER BY "issues"."created_at" DESC NULLS LAST;
```

## ⛔ Variables

None of this is implemented yet

### Search points

```qd
@@search_points = field_value search_value; ?
  field_value:search_value ~ 2
  field_value:~search_value ~ 1
  ~~ 0
@search = "foo"
@@points = field; field|search_points(@search)
people.points = @@max(first_name|points last_name|points)
people $[] $points \sd :::limit(10)
```

### Drinking age

```qd
@drinking_age = 21
users.age = birth_date|age|years
users.can_purchase_alcohol = age:>=@drinking_age
users $can_purchase_alcohol \g $%count
```

### Generation

```qd
@@generation = birth_date; birth_date|year|(birth_year; ?
  birth_year:>=2010 ~ "Alpha"
  birth_year:>=1997 ~ "Z"
  birth_year:>=1981 ~ "Millennial"
  birth_year:>=1965 ~ "X"
  birth_year:>=1946 ~ "Boomer"
  birth_year:>=1928 ~ "Silent"
  birth_year:>=1901 ~ "Greatest"
  birth_year:>=1883 ~ "Lost"
  ~~ @null)
people.generation = birth_date|generation
```

### Completion ratio by client

```qd
clients.open_count = #issues{status:"open"}
clients.closed_count = #issues{status:"closed"}
clients.completion = closed_count/(closed_issues+open_count)
clients $[] $completion \s
```

### Table-level functions

```qd
issues.@@involves = u; [
  ++#assignments{user.username:u}
  ++#comments{author.username:u}
  author.username:u
]
issues.@@labeled = l; ++labels{name:l}
---
issues @@involves("dave") @@labeled("bug")
```
