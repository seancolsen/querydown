-- Typed, empty table definitions for the "library" schema, mirroring library_schema.json (column
-- names). Column types are inferred from the column names. Used by the `db-tests` feature to run
-- compiled corpus SQL against real Postgres and DuckDB. Written in SQL that both engines accept;
-- keep this in sync with library_schema.json when columns change.

CREATE TABLE "Authors" (
  "id" integer,
  "First Name" text,
  "Last Name" text,
  "Website" text
);

CREATE TABLE "Books" (
  "id" integer,
  "Title" text,
  "Publication Year" integer,
  "Media" integer,
  "Page Count" integer,
  "LC Classification" text,
  "ISBN" text,
  "Dewey Decimal" numeric,
  "Dewey Wording" text,
  "Author" integer,
  "Publisher" integer
);

CREATE TABLE "Checkouts" (
  "id" integer,
  "Item" integer,
  "Patron" integer,
  "Checkout Time" timestamp,
  "Due Date" date,
  "Check In Time" timestamp
);

CREATE TABLE "Items" (
  "id" integer,
  "Barcode" text,
  "Acquisition Date" date,
  "Acquisition Price" numeric,
  "Book" integer
);

CREATE TABLE "Media" (
  "id" integer,
  "Type" text
);

CREATE TABLE "Patrons" (
  "id" integer,
  "First Name" text,
  "Last Name" text,
  "Email" text
);

CREATE TABLE "Publishers" (
  "id" integer,
  "Name" text
);
