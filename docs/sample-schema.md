# Sample schema

Every example in these docs is written against the same fictitious database: an issue tracker, like the one a software team might use. This page describes it, so that you can make sense of the examples and write queries of your own in the [playground](/playground), which starts out with this schema.

<!-- er-diagram -->

One column holds JSON: `users.preferences`, each user's settings, which the [JSON functions](./functions.md#json-functions) read. It's an object such as `{"theme": "dark", "page_size": 50, "notify": {"open": true, "closed": false}, "watched_labels": ["bug", "ui"], "saved_filters": [{"name": "Mine", "query": "#issues author.username:sam"}]}`, where `notify` says whether the user wants to hear about issues of each status.

Every table is in the database's current schema except `invoices`, which is in the schema `billing`, and so is written [`#/billing/invoices`](./language.md#tables-in-other-schemas).

The schema lives in the Querydown repository, where the compiler's tests use it too:

- [`issue_schema.json`](../compiler/resources/test/issue_schema.json) is the schema as the compiler takes it. The [introspection](./setup/introspection.md#the-schema-json) page explains the format.
- [`issue_schema.sql`](../compiler/resources/test/issue_schema.sql) creates the tables, for PostgreSQL or DuckDB. They're empty, since there's no sample data yet.
