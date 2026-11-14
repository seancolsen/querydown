# Sample schema

Every example in these docs is written against the same fictitious database: an issue tracker, like the one a software team might use. This page describes it, so that you can make sense of the examples and write queries of your own in the [playground](/playground), which starts out with this schema.

<!-- er-diagram -->

## Tables

| Table | Each record is… |
| -- | -- |
| `issues` | a task or a bug report, which the rest of the schema revolves around |
| `users` | a person who writes, comments on, and is assigned to issues |
| `teams` | a group of users. Each user belongs to one team. |
| `comments` | a comment that a user made on an issue |
| `assignments` | the assignment of an issue to a user. An issue can be assigned to many users. |
| `labels` | a label, such as "bug", that issues can be tagged with |
| `issue_labels` | the tagging of an issue with a label |
| `blocks` | a dependency between two issues: the `blocker` issue must be resolved before the `blocking` issue can be |
| `projects` | a project, which groups issues |
| `products` | a product, which groups projects |
| `clients` | a client, which products are built for |

## Relationships

Querydown learns how tables are related from the links in the schema: its foreign keys. The sample schema has a few of each kind of relationship that the [language](./language.md) has syntax for.

- **One related record.** Each issue has one `author` and one `project`. You follow such a link by [naming the column](./language.md#single-related-records-via-column-name-chains), as in `author.username`. Links chain: an issue's client is `project.product.client`.

    ```qd
    #issues $title $author.username $project.product.client.name
    ```

- **Many related records.** Each issue has many comments, and each project has many issues. You reach them by [naming the table](./language.md#referencing-multiple-related-records), as in `#comments`.

    ```qd
    #projects $name $#issues $#issues.created_at%max
    ```

- **Many-to-many.** Issues and labels are related through the `issue_labels` table, and issues and users through `assignments`. Querydown [finds its way](./language.md#transitive-relationships) through a table in between, so from an issue, `#labels` is enough.

    ```qd
    #issues $title $#labels.name%list
    ```

- **More than one way to relate two tables.** An issue is related to users through its `author`, through `assignments`, and through `comments`. And the `blocks` table refers to `issues` twice, as `blocker` and as `blocking`. These are what make some paths [ambiguous](./language.md#ambiguous-paths), and why you sometimes need to [name the linking column](./language.md#specifying-the-linking-column).

    ```qd
    #issues $title $#blocks(blocking) -> blocked_by $#assignments.#users -> assignees
    ```

- **A table related to itself.** An issue can be marked as a duplicate of another issue, through its `duplicate_of` column. See [intersecting paths](./language.md#intersecting-paths).

    ```qd
    #issues $title $duplicate_of.title $#issues -> duplicates
    ```

## Columns

Most columns are what their names suggest. A few are worth knowing about when you read the examples:

- Every table has an integer `id`, which the foreign keys of other tables refer to.
- A foreign key column is named after what it refers to, without an `_id` suffix: `issues.author`, not `issues.author_id`. That makes paths read naturally, as in `author.team.name`.
- `issues.status` is free text, such as "open" or "closed".
- `issues.created_at`, `issues.due_date`, and `comments.created_at` are timestamps, while `users.birth_date` is a date and `users.last_seen` is a timestamp with a time zone.
- `projects.is_active` and `projects.is_pro_bono` are booleans.

## The schema as files

The schema lives in the Querydown repository, where the compiler's tests use it too:

- [`issue_schema.json`](../compiler/resources/test/issue_schema.json) is the schema as the compiler takes it. The [introspection](./setup/introspection.md#the-schema-json) page explains the format.
- [`issue_schema.sql`](../compiler/resources/test/issue_schema.sql) creates the tables, for PostgreSQL or DuckDB. They're empty, since there's no sample data yet.
