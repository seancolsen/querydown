# E2E test corpus

⛔ = skip
🔦 = solo

## Simple

> Show all issue ids

```qd
issues $id->id
```

```sql
SELECT "issues"."id" AS "id" FROM "issues";
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

## Paths to one

### ⛔ Joined column in related table

> Issues under project named "foo".

```qd
issues project.title:"foo" $id->id
```

```sql
SELECT
  "issues"."id" AS "id"
FROM "issues"
LEFT JOIN "projects" ON
  "issues"."project" = "projects"."id"
WHERE
  "project"."title" = 'foo';
```

### Referenced column in related table not joined

> Issues under project with id 1.

This test case ensures that we don't have an unnecessary join on `projects` when the `projects.id` value can already be found within `issues.project`.

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
  *
FROM "issues"
JOIN "cte0" ON
  "issues"."id" = "cte0"."pk";
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
  *
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
  *
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
  *
FROM "users"
JOIN "cte0" ON
  "users"."id" = "cte0"."pk";
```

### ⛔ Has through inferred intermediate

FIXME there is a bug here


```qd
issues ++#labels
```

```sql
TODO
```


## ⛔ Filtered paths

Not yet implemented

### A filter that aligns with the join

> Issues, showing the total number of comments made by the issue's author

```qd
issues $#comments{user:issue.author}
```

```sql
TODO
```

## ⛔ Variables

None of this is implemented yet

### Search points

```qd
@@search_points = field_value search_value; ?
  field_value:search_value => 2
  field_value:~search_value => 1
  *=> 0
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
  birth_year:>=2010 => "Alpha"
  birth_year:>=1997 => "Z"
  birth_year:>=1981 => "Millennial"
  birth_year:>=1965 => "X"
  birth_year:>=1946 => "Boomer"
  birth_year:>=1928 => "Silent"
  birth_year:>=1901 => "Greatest"
  birth_year:>=1883 => "Lost"
  * => @null)
people.generation = birth_date|generation
```

### Completion ratio by client

```qd
clients.open_count = #issues{status:"open"}
clients.closed_count = #issues{status:"closed"}
clients.completion = closed_count/(closed_issues+open_count)
clients $[] $completion \s
```
