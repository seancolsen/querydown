# Querydown

Querydown is a modern [domain-specific programing language](https://en.wikipedia.org/wiki/Domain-specific_language) designed for expressively writing relational databases queries that [transpile](https://en.wikipedia.org/wiki/Source-to-source_compiler) to [SQL](https://en.wikipedia.org/wiki/SQL). The code is succinct and safe for end-users to write, making it like **"markdown for SQL"**.

## Status

⚠️ Querydown is currently in **concept phase**. ⚠️


## Design goals

- Expressive and succinct
- Safety from arbitrary joins which may cause unwanted compute if missing the proper `ON` clause
- No keywords
- All queries begin with one base table
- Results never have more rows than are present in the base table

## Language design and examples

**[See examples of querydown code](./docs/design.md)**


## How it works

The querydown processor does the following

1. Takes input
    - Querydown code
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
