# E2E test corpus

⛔ = skip
✅ = solo

## Paths

### Path to many with column at end

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
