-- Typed, empty table definitions for the "issues" schema, mirroring issue_schema.json (column
-- names) and issue_schema.dbml (column types). Used by the `db-tests` feature to run compiled
-- corpus SQL against real Postgres and DuckDB. Written in SQL that both engines accept; keep this
-- in sync with issue_schema.json when columns change.

CREATE TABLE "users" (
  "id" integer,
  "username" text,
  "email" text,
  "team" integer,
  "birth_date" date,
  "last_seen" timestamptz
);

CREATE TABLE "issues" (
  "id" integer,
  "title" text,
  "description" text,
  "created_at" timestamp,
  "author" integer,
  "status" text,
  "project" integer,
  "duplicate_of" integer,
  "due_date" timestamp
);

CREATE TABLE "assignments" (
  "id" integer,
  "issue" integer,
  "user" integer
);

CREATE TABLE "blocks" (
  "id" integer,
  "blocker" integer,
  "blocking" integer
);

CREATE TABLE "projects" (
  "id" integer,
  "name" text,
  "product" integer,
  "is_active" boolean,
  "is_pro_bono" boolean
);

CREATE TABLE "labels" (
  "id" integer,
  "name" text
);

CREATE TABLE "issue_labels" (
  "id" integer,
  "issue" integer,
  "label" integer
);

CREATE TABLE "comments" (
  "id" integer,
  "issue" integer,
  "user" integer,
  "body" text,
  "created_at" timestamp
);

CREATE TABLE "teams" (
  "id" integer,
  "name" text
);

CREATE TABLE "products" (
  "id" integer,
  "name" text,
  "client" integer
);

CREATE TABLE "clients" (
  "id" integer,
  "name" text
);
