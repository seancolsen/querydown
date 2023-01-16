# Lower Query Language design

## Example schemas

### Library

```yml
- patron
  - id
  - first_name
  - last_name

- email
  - id
  - patron -> patron.id
  - email

- patron_tag
  - id
  - patron
  - tag

- tag
  - id
  - name

- checkout
  - id
  - item -> item.id
  - patron -> patron.id
  - out_date
  - due_date
  - in_date

- item
  - id
  - publication -> publication.id

- publication
  - id
  - title
  - year
  - format
  - author -> author.id
  - publisher -> publisher.id

- author
  - id
  - name
  - birth_date
  - death_date

- publisher
  - id
  - name
```

### Logistics

```yml
- location
  - id
  - addressee
  - street
  - apt
  - city
  - state
  - zip

- shipment
  - id
  - tracking_number
  - origin -> location
  - destination -> location
  - departure_datetime
  - arrival_datetime
```


## Basics

- Write the name of a table to select from it. when no columns are specified, all are returned.

    ```
    publication
    ```

    - ```sql
      SELECT * FROM "publication";
      ```

- Here we refer to "publication" the **base table**. Every query has one and only one base table.

- Specify columns by listing them prefixed with `-`

    ```
    publication -id -title
    ```

    - ```sql
      SELECT "id", "title" FROM "publication";
      ```

- Use `:` after a column to give it an alias.

    ```
    publication -id:Identifier -title:Name
    ```

    - ```sql
      SELECT
        "id" AS "Identifier",
        "first_name" AS "Name"
      FROM "publication";
      ```


## White space

White space doesn't matter. The following two queries are identical.

- ```
  publication-id:Identifier-title:Name
  ```

- ```
  publication
  - id: Identifier
  - title: Name
  ```


## Comments

- Comments are enclosed by `/* */`
- Single line comments begin with `//`


## Quoting identifiers

If you want to reference a table name or column name which contains characters other than letters and underscores, enclose it in backticks

```
`Gala Attendees`
- `Given Name`: `First Name`
- `Surname`: `Last Name`
```

- When compiled for Postgres

    ```sql
    SELECT
      "Given Name" AS "First Name",
      "Surname" AS "Last Name"
    FROM "Gala Attendees";
    ```

- When compiled for MySQL

    ```txt
    SELECT
      `Given Name` AS `First Name`,
      `Surname` AS `Last Name`
    FROM `Gala Attendees`;
    ```


## Sorting

- Ascending sorting by one column. The `s` stands for "sort".

    ```
    author -first_name -[s]last_name
    ```

    - ```sql
      SELECT first_name, last_name
      FROM author
      ORDER BY last_name NULLS LAST
      ```

- Descending sorting is indicated via a `d` within parentheses after the `s`.

    ```
    author -first_name -[s(d)]last_name
    ```

    - ```sql
      SELECT first_name, last_name
      FROM author
      ORDER BY last_name NULLS LAST
      ```

- Sorting by multiple columns is done by numbers within the parentheses to indicate ordinality.

    ```
    author
    - id
    - [s(3)] first_name
    - [s(2)] last_name
    - [s(1d)] birth_date
    ```

    - ```sql
      SELECT id, first_name, last_name, birth_date
      FROM author
      ORDER BY
        birth_date DESC NULLS LAST,
        last_name, first_name
      ```

- Sorted columns without any ordinality specified are sorted in the order the appear, after all columns with indicated ordinality.

    ```
    author
    - id
    - [s] first_name
    - [s] last_name
    - [s(1)] birth_date
    ```

    - ```sql
      SELECT id, first_name, last_name, birth_date
      FROM author
      ORDER BY
        birth_date NULLS LAST,
        first_name NULLS LAST,
        last_name NULLS LAST
      ```

- By default, `NULL` values are sorted last, but this behavior can be modified using the `n` flag, which stands for "nulls first".

    ```
    author -first_name -[s(n)]last_name
    ```

    - ```sql
      SELECT first_name, last_name
      FROM author
      ORDER BY last_name NULLS FIRST
      ```

## LIMIT and OFFSET

- Ten authors starting from 100

    ```
    author
    --> limit(10)
    --> offset(100)
    ```

## Literal values

- String literals can be quoted either with single or with double quotes.

    TODO raw strings to make regex strings easier

- Numeric literals are written as-is

- Date literals are written in ISO form

    TODO how to specify a date with time

- TODO raw durations

| LQL | SQL |
| -- | -- |
| `@now` | `now()` |
| `@inf` | `Infinity` |
| `@true` | `TRUE` |
| `@false` | `FALSE` |
| `@null` | `NULL` |


## Conditions

### AND vs OR

- One condition

    ```
    publication {title="Foo"}
    ```

    - ```sql
      SELECT * FROM "publication" WHERE "title" = 'Foo';
      ```

- Spaces delimit multiple conditions

    ```
    publication {title="Foo" year=1999}
    ```
    
    - ```sql
      SELECT * FROM "publication" WHERE "title" = 'Foo' AND "year" = 1999;
      ```

- Square brackets enclose `OR` conditions

    ```
    publication [title="Foo" year=1999]
    ```
    
    - ```sql
      SELECT * FROM "publication" WHERE "title" = 'Foo' OR "year" = 1999;
      ```

- If you omit the braces, then a set of AND conditions is inferred

    ```
    publication title="Foo" year=1999
    ```
    
    - ```sql
      SELECT * FROM "publication" WHERE "title" = 'Foo' AND "year" = 1999;
      ```

- Conditions can be nested

    ```
    publication [
      { title="Foo" year=1999 }
      { title="Bar" year=2000 }
    ]
    ```

    - ```sql
      SELECT *
      FROM "publication"
      WHERE (
        ("title" = 'Foo' AND "year" = 1999) OR
        ("title" = 'Bar' AND "year" = 2000)
      );
      ```

### Comparison operators

- `field = 1`
- `field = other_field`
- `field = "string literal"`
- `field = 'string literal in single quotes'`
- `field = 2017-01-01`
- `field > 1`
- `field > 2017-01-01`
- `field >= 1`
- `field < 1`
- `field <= 1`
- `field != 1`
- `field ~ "^foo.*"` Regex comparison
- `field ~(i) "^foo.*"` Regex comparison with flags (TODO: flesh out specs)
- `field ~~ "foo"` Like comparison


### Comparison expansion

- Comparisons get expanded when one side is enclosed in brackets

    ```
    publication year = [2000 2010]
    ```

    - ```sql
      SELECT * FROM "publication"
      WHERE "year" IN (2000, 2010);
      ```

    ```
    patron {first_name last_name} = @null
    ```

    - ```sql
      SELECT * FROM "patron"
      WHERE "first_name" IS NULL AND "last_name" IS NULL;
      ```

    ```
    patron [first_name last_name] ~ "foo"
    ```

    - ```sql
      SELECT * FROM "patron"
      WHERE "first_name" ~ 'foo' OR "last_name" ~ 'foo';
      ```

- All columns can be specified via the `*` character. Columns which don't support the type of comparison used will be excluded

    ```
    patron [*] ~ "foo"
    ```

    - ```sql
      SELECT * FROM "patron"
      WHERE "first_name" ~ 'foo' OR "last_name" ~ 'foo';
      ```

    Here the `id` column doesn't support the `~` comparison, so it's not used.

- The comparator can be altered per-value by using `?`

    ```
    publication year ? {>=2000 <=2010}
    ```

    - ```sql
      SELECT * FROM "publication"
      WHERE "year" >= 2000 AND "year" <= 2010;
      ```

    This is similar to the SQL `BETWEEN`, but somewhat more explicit.

- When using the `?` comparison, the per-value operators must be placed on the right-hand-side of the `?`. The following won't work:

    ```
    publication {>=2000 <=2010} ? year // INVALID!
    ```

- If both sides of the comparison are enclosed in brackets, then the brackets on left side are used for the outer precedence

    ```
    patron {first_name last_name} ~ ["foo" "bar"]
    ```

    - ```sql
      SELECT * FROM "patron"
      WHERE
        ("first_name" ~ 'foo' OR "first_name" ~ 'bar') AND
        ("last_name" ~ 'foo' OR "last_name" ~ 'bar');
      ```

    ```
    author [birth_date death_date] ? {>=1900-01-01 <2000-01-01}
    ```

    - ```sql
      SELECT * FROM "author"
      WHERE
        ("birth_date" >= '1900-01-01' AND "birth_date" < '2000-01-01') OR
        ("death_date" >= '1900-01-01' AND "death_date" < '2000-01-01');
      ```

## Functions

- The most overdue checkouts 

    ```
    checkout
    - id
    - [s(d)] due_date|minus(@now)|days: days_overdue
    ```

    Note:

    - All functions are applied via the pipe syntax.
    - There are no operators like `+ - * /`. Named functions are used instead, providing for clear, linear chaining of operations.

- Functions

    - `minus()`
    - `plus()`
    - `times()`
    - `divide()`
    - `when()`
    - `segment()`
    - `bins()`
    - `lower_bounded()`
    - `upper_bounded()`
    - `when_null()` (i.e. `COALESCE`)
    - `date_format()`
    - `days`
    - `months`
    - `years`
    - `weeks`
    - `floor`
    - `ceil`
    - `mod`
    - `and()`
    - `or()`
    - `xor()`
    - `not()`
    - ...

## Interpolated strings

- Are specified via `^{value}^`

    ```
    patron -^{first_name} {last_name}^: name
    ```

    - ```sql
      SELECT concat(first_name, ' ', last_name) as "name"
      from patron
      ```

## Incremental column specification

- Use `-()` to specify all columns

    ```
    patron -()
    ```

    - ```sql
      SELECT * FROM patron;
      ```

- Add a column after all columns

    ```
    patron -() -^{first_name} {last_name}^: full_name
    ```

    - ```sql
      SELECT
        id,
        first_name,
        last_name,
        concat(first_name, ' ', last_name) as "full_name"
      FROM patron;
      ```

- Hide a column

    ```
    patron -([h]id)
    ```

    - ```sql
      SELECT
        id,
        last_name,
      FROM patron;

- Sort by columns, leaving their position in the table unchanged.

    ```
    checkout -([s]patron.last_name[s]out_date)
    ```

    - ```sql
      SELECT id, item, patron, out_date, due_date, in_date
      FROM checkout
      LEFT JOIN patron on patron.id = checkout.patron
      ORDER BY patron.last_name, checkout.out_date;
      ```

## Computed fields

- Checkouts which are more than 10 days overdue (with the exact days overdue displayed).

    ```
    checkout.$days_overdue := due_date|minus(@now)|days
    checkout { $days_overdue > 10 } -id -[s(d)]$days_overdue
    ```


## Joining related data

You can bring in data from related tables -- but joins don't work quite like in SQL. In LQL, the number of rows in the results will never be more than the number of rows in the base table.


### Many to one

- Publications with their authors

    ```
    publication -title -author.name
    ```

    - ```sql
      SELECT
        "publication"."title"
        "author"."name"
      FROM "publication"
      LEFT JOIN "author" ON
        "publication"."author" = "author"."id";
      ```
    
    Note:
    
    - In the LQL above, `author` refers to the `author` _column_ within the `publication` table (not the `author` table).
    - We use `LEFT JOIN` so that we don't inadvertently filter out publications with no associated author. A condition can be manually added to filter out those publications if desired.

- Publications from living authors -- _or unknown authors_.

    ```
    publication author.death_date=@null
    ```

    - ```sql
      SELECT "publication".*
      FROM "publication"
      LEFT JOIN "author" ON
        "publication"."author" = "author"."id"
      WHERE
        "author"."death_date" IS NULL;
      ```

- Publications from living authors (excluding unknown authors).

    ```
    publication author!=@null author.death_date=@null
    ```

    - ```sql
      SELECT "publication".*
      FROM "publication"
      LEFT JOIN "author" ON
        "publication"."author" = "author"."id"
      WHERE
        "author"."id" IS NOT NULL AND
        "author"."death_date" IS NULL;
      ```

- Conditions on directly related records

    ```
    publication author.{birth_date>2000-01-01 death_date!=@null}
    ```

    This is just syntactic sugar for
    
    ```
    publication author.birth_date>2000-01-01 author.death_date!=@null
    ```

    - ```sql
      SELECT "publication".*
      FROM "publication"
      LEFT JOIN "author" ON
        "publication"."author" = "author"."id"
      WHERE
        "author"."id" IS NOT NULL AND
        "author"."birth_date" > '2000-01-01' AND
        "author"."death_date" IS NOT NULL;
      ```


### One to many

- Authors and how many publications they have

    ```
    author -name -*publication|%count
    ```

    - ```sql    
      WITH cte_0 AS (
        SELECT
          "author" as pk,
          count(*) AS f0
        FROM "publication"
        GROUP BY "author"
      )
      SELECT
        "author"."name",
        cte_0.f0 as "publication_count"
      FROM "author"
      JOIN cte_0 ON cte_0.f0 = "author"."id";
      ```

    Note:

    - `%count` is an aggregate function. All aggregate functions begin with `%`.

- Authors and the year of their first publication

    ```
    author -name -*publication.year|%min
    ```

    - ```sql
      WITH cte_0 AS (
        SELECT
          "author" as pk,
          min("year") AS f0
        FROM "publication"
        GROUP BY "author"
      )
      SELECT
        "author"."name",
        cte_0.f0 as "publication_count"
      FROM "author"
      JOIN cte_0 ON cte_0.f0 = "author"."id";
      ```

- Authors who have no publications

    ```
    author *publication|%count = 0
    ```

    While we could use a CTE for this, a more performant heuristic would be to use an exclusion join like below. I'm not sure how feasible it will be to implement heuristics like this across the board, but it would be neat to try!

    - ```sql
      SELECT
        "author".*,
      FROM "author"
      LEFT JOIN "publication" ON
        "publication"."author" = "author"."id"
      WHERE
        "publication"."id" IS NULL;
      ```

- Authors who have published books with "Penguin" since year 2000.

    ```
    author *publication{year>2000 publisher.name="Penguin"}|%count > 0
    ```

    - ```sql
      SELECT
        "author".*,
      FROM "author"
      LEFT JOIN "publication" ON
        "publication"."author" = "author"."id"
      WHERE
        "publication"."id" IS NULL;
      ```

- Authors of audio books that have been checked out at least 100 times

    ```
    author *publication{format="Audio"}*item*checkout|%count >= 100
    ```

- Publications from authors with only one publication

    ```
    publication author*publication|%count = 1 
    ```

- Publications checked out in the past month by employees

    ```
    publication
    *item*checkout{out_date>@now|minus(1M)}.patron*patron_tag.tag.name = "Employee"
    -id -title -[s]author.name
    ```

    ```sql
    WITH cte_0 AS (
      SELECT
        "publication"."id" as "pk"
      FROM "publication"
      LEFT JOIN "item" ON
        "item"."publication_id" = "publication"."id"
      LEFT JOIN "checkout" ON
        "checkout"."item_id" = "item"."id" AND
        "checkout"."out_date" > NOW() - 'P1M'::interval
      LEFT JOIN "patron" ON
        "patron"."id" = "checkout"."patron"
      LEFT JOIN "patron_tag" ON
        "patron_tag"."patron" = "patron"."id"
      LEFT JOIN "tag" ON
        "tag"."id" = "patron_tag"."tag"
      WHERE
        "tag"."name" = 'Employee'
    )
    SELECT
      "publication"."id",
      "publication"."title",
      "author"."name"
    FROM "publication"
    JOIN cte_0 on cte_0.pk = "publication"."id"
    LEFT JOIN "author" ON
      "author"."id" = "publication"."author"
    ORDER BY "author"."name" NULLS LAST;
    ```

- If a table links to another table multiple times, then parentheses must be used to specify which foreign key column to use.

    A location with counts of its shipments.

    ```
    location
    - id
    - *shipment(destination)|%count: count_shipments_to_here
    - *shipment(origin)|%count: count_shipments_from_here
    ```

- All aggregate functions

    - `%count` (This is the only aggregate function which can also be applied to the _table_)
    - `%count_distinct`
    - `%sum`
    - `%product`
    - `%min`
    - `%max`
    - `%avg`
    - `%list()` (i.e. `group_concat` or `string_agg`) This function accepts a `separator` argument. TODO: how to sort entries


### Multi-column foreign keys

TODO

### Polymorphic associations

TODO

## Grouping and aggregating

- Grouping is indicated by placing a `g` within the square brackets that prefix the column specifiers.

    ```
    author
    - [g] death_date|when(@null:@true *:@false): is_alive
    - %count
    ```

    Note:

    - All ungrouped columns must contain an aggregate function
    - `%count` can occur on its own (outside of a function pipeline), which is equivalent to `count(*)`.
    - Grouping by multiple columns is done via `[g(1)]` and `[g(2)]`, similar to sorting.


## WINDOW

- Origin locations, with the destination of their most recent shipment

    ```
    shipment %%([s(d)]departure_datetime[p]origin)|%row_number = 1
    - origin
    - origin.addressee
    - destination
    - destination.addressee
    ```

- How many days into each month did it take us to reach 1000 checkouts?

    ```
    checkout %%([s]out_date[p]out_date|year_month)|%row_number = 1000
    - out_date|day_of_month
    ```

    - ```sql
      with t as (
      select
        "Checkout Time" as checkout_time,
        row_number() over (
          partition by to_char("Checkout Time", 'YYYY-MM') order by "Checkout Time"
        ) as row_num
        from "Checkouts" 
      )
      select extract(day from checkout_time)
      from t
      where row_num = 1000
      order by checkout_time
      ```

- Publications that have been on the top 10 most frequently checked-out list every month for the past year.

    TODO

## HAVING

- Doesn't exist, but you can achieve similar behavior using "pipeline of multiple queries" (described below).


## Pipeline of multiple queries

- Books that have been checked out by the same patron at least 5 times in the past year

    ```
    checkout
    { out_date > @now|minus(1Y) }
    - [g] item.publication: publication
    - [g] patron
    - %count: checkout_count
    ~~~
    { checkout_count > 5 }
    - [g] publication
    - patron|%count: patron_count
    - checkout_count|%max: max_checkouts
    ~~~
    - publication.id
    - publication.title
    - publication.author.name
    - patron_count
    - [s(d)] max_checkouts
    ```

## UNION

- History of activity for a specific location

    ```
    shipment { origin = 7  departure_datetime != @null }
    -id -tracking_number -"Send": action -departure_datetime: time
    +++
    shipment { destination = 7  arrival_datetime != @null }
    -id -tracking_number -"Receive": action -arrival_datetime: time
    ~~~
    -[s]time -action -tracking_number
    ```


- TODO what if I want to do a pipeline within a union?

```
#a := (
  shipment { origin = 7  departure_datetime != @null }
  - id
  - tracking_number
  - "Send": action
  - departure_datetime: time
)

#b := (
    shipment { destination = 7  arrival_datetime != @null }
    - id
    - tracking_number
    - "Receive": action
    - arrival_datetime: time
    ~~~
    // more here
);

#a +++ #b
~~~
-[s]time -action -tracking_number
```

## Parameterization

```
author id = &id
```


## Complex examples

- Unpopular publications we should cull

    TODO
    
- Popular publications which would benefit from more copies in stock

    TODO

- Patrons who currently have the highest late fee

    ```
    checkout.$days_overdue := in_date|when(@null:@now|minus(due_date)|days *:@null)
    checkout.$late_fee := $days_overdue|when_null(0)|times(2) // $2.00 per day
    patron.$late_fee := *checkout.$late_fee|%sum
    patron -[s(d)]$late_fee -*email.email|%list
    ```

- Patrons with at least 1 year of checkout history who have never gone more than 14 days without a checkout.

    TODO

- Average days overdue, by month

    ```
    checkout.$days_overdue := in_date|when(@null:@now|minus(due_date)|days *:@null)
    checkout $days_overdue > 0 -[gs]out_date|year_month -$days_overdue|avg
    ```

- TODO examples
    - CRM total number of contacts who have given at least $500 total donations of certain type in past year and who are not registered for a specific event
    - CRM top ten zip codes by average amount of donations over the past 5 years of certain types
    - CRM total amount raised, by event, for all events of a certain type within the past 10 years
    - CRM total amount raised, by fiscal year, including certain contribution types



---

## Symbols and their meanings

```
s      "sort" (in a column control)
d      "descending" (in a sort spec in a column control)
v      "values first" (aka "nulls last") (in a sort spec in a column control)
h      "hide" (in a column control)
g      "group" (in a column control)
p      "partition" (in a column control in a window)

"      string
'      string
`      db entity
^      interpolated string
#   
$      user-defined variable
%      aggregate function
@      literal
~      regex comparison
&      (MAYBE) parameterization
|      pipe
/   
\      escape sequence within string
*      one-to-many join
+   
-      column spec
_      NULL literal
=      equality comparison
<      less than comparison
>      greater than comparison
!      negate a comparison
?      comparison expansion
.      related value
,   
:      alias, associative function arguments
;

( )    function call
[ ]    OR conditions, column spec details
{ }    AND conditions

!~     not RLIKE comparison
~~     not LIKE comparison
~~     LIKE comparison
!=     not equal comparison
>=     greater or equal comparison
<=     less or equal comparison

//     single line comment
/* */  multi-line comment

-( )   relative column spec
-->    LIMIT and OFFSET
%%( )  window

~~~    pipeline
+++    union
```


