# E2E test corpus

⛔ = skip
✅ = solo

## Paths

### Path to many with column at end

```qd
issues $id $#comments.created_at%max->most_recent_comment
```

```sql
WITH "cte1" AS (
  SELECT
    "comments"."issue" AS "pk",
    MAX("comments"."created_at") AS "v1",
  FROM "comments"
  GROUP BY "comments"."issue"
)
SELECT
  "issues"."id",
  "cte1"."v1" AS "most_recent_comment"
FROM "issues"
LEFT JOIN "cte1" ON "cte1"."pk" = "issues"."id";
```

### Path through one, many

```qd
issues $id->id $author.#comments->total_comments_by_author
```

```sql
WITH "cte1" AS (
  SELECT
    "comments"."issue" AS "pk",
    count(*) AS "v1"
  FROM "comments"
  GROUP BY "comments"."author"
)
SELECT
  "issues"."id",
  COALESCE("cte1"."v1", 0) AS "total_comments_by_author"
FROM "issues"
LEFT JOIN "users" ON "users"."id" = "issues"."author"
LEFT JOIN "cte1" ON "cte1"."pk" = "users"."id";
```

### Path through many, many

```qd
users $id $#issues.#comments.created_at%max
```

```sql
WITH "cte1" AS (
  SELECT
    "issues"."author" AS "pk",
    MAX("comments"."created_at") AS "v1"
  FROM "issues"
  JOIN "comments" ON "issues"."id" = "comments"."issue"
  GROUP BY "issues"."author"
)
SELECT
  "users"."id",
  "cte1"."v1"
FROM "users"
LEFT JOIN "cte1" ON "cte1"."pk" = "users"."id"
```

### Path through many, one, many

```qd
projects $id->id $#issues.author.#comments.created_at%max->v
```

```sql
WITH "cte1" AS (
  SELECT
    "issues"."project" AS "pk",
    MAX("comments"."created_at") AS "v1"
  FROM "issues"
  JOIN "users" ON "users"."id" = "issues"."author"
  JOIN "comments" ON "users"."id" = "comments"."user"
  GROUP BY "issues"."project"
)
SELECT
  "projects"."id" AS "id",
  "cte1"."v1" AS "v"
FROM "projects"
LEFT JOIN "cte1" ON "cte1"."pk" = "projects"."id";
```


## ⛔ Variables

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

