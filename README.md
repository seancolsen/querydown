# Lower Query Language

Lower Query Language, or LQL, is a simple language designed to allow less-technical end-users to query a relational database.

It's like "markdown for SQL". 

## Design goals

- Expressive and succinct
- Safety from arbitrary joins which may cause unwanted compute if missing the proper `ON` clause
- No keywords
- All queries begin with one base table
- Results never have more rows than are present in the base table

## Language design and examples

**[See examples of LQL code](./docs/lql-design.md)**

## Status

LQL is currently in **concept phase**.

## How it works

An LQL processor does the following

1. Takes input
    - LQL code
    - Database schema (so that it knows about foreign keys)
    - *(optionally) Additional schema hints (like pseudo foreign keys)*
    - *(optionally) Other global settings*
1. Produces output
    - SQL
    - Information about the origin of each column in the results (so that cells may be updated)

## Intentional limitations

- It only produces SELECT queries.
- Requires knowledge of schema
- Joinable tables are specified by configuration (which will typically be limited to foreign keys). You can't join arbitrary tables.
- With some exceptions for relative dates, it does not allow expressions to be used as values.
