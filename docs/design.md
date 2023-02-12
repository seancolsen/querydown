# Querydown Design

## Basics

- Write the name of a table to select from it. when no columns are specified, all are returned.

    ```
    issues
    ```

- Here we refer to "publication" the **base table**. Every query has one and only one base table.

- Specify columns by listing them prefixed with `:`

    ```
    issues :id :title
    ```

- Use `->` after a column to give it an alias.

    ```
    issues :id->Identifier :title->Subject
    ```

## White space

White space doesn't matter. The following two queries are identical.

- ```
  issues:id->Identifier:title->Subject
  ```

- ```
  issues
  : id    -> Identifier
  : title -> Subject
  ```

## Quoting identifiers

If you want to reference a table name or column name which contains characters other than letters and underscores, enclose it in backticks

```
`Gala Attendees`
: `Given Name`-> `First Name`
: `Surname`-> `Last Name`
```


## Sorting

- Ascending sorting by one column. The `s` stands for "sort".

    ```
    issues :title :created_at \s
    ```

- Descending sorting is indicated via a `d` after the `s`.

    ```
    issues :title :created_at \sd
    ```

- Sorting by multiple columns is done via numbers to indicate ordinality.

    ```
    author
    : id
    : first_name \s3
    : last_name \s2
    : birth_date \sd1
    ```

- Sorted columns without any ordinality specified are sorted in the order the appear, after all columns with indicated ordinality.

    ```
    author
    : id
    : first_name \s
    : last_name \s
    : birth_date \s1
    ```

- By default, `NULL` values are sorted last, but this behavior can be modified using the `n` flag, which stands for "nulls first".

    ```
    author :first_name :last_name \sn
    ```

## LIMIT and OFFSET

- Ten authors starting from 100

    ```
    author
    --> limit(10)
    --> offset(100)
    ```

## Conditions

### AND vs OR

- One condition

    ```
    publication {title="Foo"}
    ```

- Spaces delimit multiple conditions

    ```
    publication {title="Foo" year=1999}
    ```

- Square brackets enclose `OR` conditions

    ```
    publication [title="Foo" year=1999]
    ```

- If you omit the braces, then a set of AND conditions is inferred

    ```
    publication title="Foo" year=1999
    ```

- Conditions can be nested

    ```
    publication [
      { title="Foo" year=1999 }
      { title="Bar" year=2000 }
    ]
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
    
    (This is like the SQL `IN` operator.)

    ```
    patron {first_name last_name} = @null
    ```

    ```
    patron [first_name last_name] ~ "foo"
    ```

- All columns can be specified via the `*` character. Columns which don't support the type of comparison used will be excluded

    ```
    patron [*] ~ "foo"
    ```

    Here the `id` column doesn't support the `~` comparison, so it's not used.

- If both sides of the comparison are enclosed in brackets, then the brackets on left side are used for the outer precedence

    ```
    patron {first_name last_name} ~ ["foo" "bar"]
    ```

### Scoped conditionals

- The comparator can be altered per-value by using `?`, the **scoped conditional operator**

    ```
    publication year ? {& >= 2000 & <= 2010}
    ```

    Within the braces, `&` is called a **slot** and refers to the value from the outer scope (i.e. `year` in this case)

    The above code can be made slightly more readable and idiomatic by reversing the first inner condition so that it reads more like math notation

    ```
    publication year ? {2000 <= & & <= 2010}
    ```

    This is similar to the SQL `BETWEEN`, but with more explicit control over the comparison operators 

- Slots can only be used with the scoped conditional operator, and they must be placed _after_ the operator.

    ```
    publication {2000 <= & & <=2010} ? year // INVALID!
    ```

- Scoped conditionals can be mixed with comparison expansion as follows.

    Authors who were either born or who died during the 20th century:

    ```
    author [birth_date death_date] ? {@1900-01-01 <= & & < @2000-01-01}
    ```

## Functions

- The most overdue checkouts 

    ```
    checkout
    : id
    : due_date|minus(@now)|days-> days_overdue \sd
    ```

    Note:

    - All scalar functions are applied via the pipe syntax.
    - There are no operators like `+ - * /`. Named functions are used instead, providing for clear, linear chaining of operations.

- Functions

    - `minus()`
    - `plus()`
    - `times()`
    - `divide()`
    - `is_null`
    - `is_non_null`
    - `has_value`
    - `bool`
    - `not`
    - `when()`
    - `if()`
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
    patron :^{first_name} {last_name}^-> name
    ```

## Incremental column specification

-  Use `:[]` to specify all columns, giving you control to add a column after all columns

    ```
    patron :[] :^{first_name} {last_name}^->full_name
    ```

- Hide a column

    ```
    patron :[id\h]
    ```

- Sort by columns, leaving their position in the table unchanged.

    ```
    checkout :[patron.last_name \s out_date \s]
    ```


## Joining related data

You can bring in data from related tables -- but joins don't work quite like in SQL. In querydown, the number of rows in the results will never be more than the number of rows in the base table.


### Many to one

- Publications with their authors

    ```
    publication :title :author.name
    ```

    Note:
    
    - In the querydown code above, `author` refers to the `author` _column_ within the `publication` table (not the `author` table).
    - We use `LEFT JOIN` so that we don't inadvertently filter out publications with no associated author. A condition can be manually added to filter out those publications if desired.

- Publications from living authors -- _or unknown authors_.

    ```
    publication author.death_date=@null
    ```

- Publications from living authors (excluding unknown authors).

    ```
    publication author != @null author.death_date = @null
    ```

- When the **scoped conditional operator** (`?`) is used on a foreign key column, the scope of the related table is used inside the braces.

    ```
    publication author ? {birth_date > @2000-01-01 death_date != @null}
    ```

    This expands to
    
    ```
    publication author.birth_date > @2000-01-01 author.death_date != @null
    ```

- Checkouts by deceased authors

    ```
    checkout ..author.death_date != @null
    ```

    Here, `author` refers to the `author` _table_ (not column). Note that the `checkout` table does not have an `author` column, but each `checkout` record _is_ directly related to one `author` record (via the `item` and `publication` tables). So the above code is shorthand for:

    ```
    checkout item.publication.author.death_date != @null}
    ```

    This shorthand only works if there is one unambiguous path from the base table to the linked table. The longer form is required if there is more than one way to join the two tables.

### One to many

- Authors that have at least one publication

    ```
    author #publication > 0
    ```

    ```
    author ++publication
    ```

- Authors that have no publications

    ```
    author #publication = 0
    ```

    ```
    author --publication
    ```

- Authors and how many publications they have

    ```
    author :name :#publication
    ```


- Authors and the year of their first publication

    ```
    author :name :#publication.year%min
    ```

    Note:

    - `%min` is an aggregate function. All aggregate functions begin with `%`.

- Authors who have published books with "Penguin" since year 2000.

    ```
    author ++publication{year > 2000 publisher.name = "Penguin"} > 0
    ```

- Authors of publications which have been checked out in the past week

    ```
    author ++checkout{out_date > @1W|ago}
    ```

    The `checkout` table is not directly related to the `author` table, but that's okay. The above code is shorthand for the following more explicit code:

    ```
    author #publication.#item.#checkout{out_date > @1W|ago} > 0
    ```
    
    We can use the shorthand in this case because there is only one path through which `author` can be joined to `checkout`. If tables can be joined through multiple paths, then a path will need to be specified which is not ambiguous.

- If one table directly links to another table multiple times, then parentheses must be used to specify which foreign key column to use.

    A location with counts of its shipments.

    ```
    location
    : id
    : #shipment(destination)->count_shipments_to_here
    : #shipment(origin)->count_shipments_from_here
    ```

- Publications checked out in the past month by employees

    ```
    publication ++checkout{out_date > @1M|ago ++tag{name = "Employee"}}
    ```

- Checkouts of Biography books

    ```
    checkout ++genre{name = "Biography"}
    ```

- Checkouts by patrons with no emails

    ```
    checkout patron ? {--email}
    ```
    
- Patrons who, in the past week, have checked out at least one publication which is authored by "Foo" and _not_ categorized as "Biography"

    ```
    patron ++checkout{out_date > @1W|ago ..author.name = "Foo" --genre{name = "Biography"}}
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
    author :death_date|is_null->is_alive \g :%count
    ```

    Note:

    - All ungrouped columns must contain an aggregate function
    - `%count` can occur on its own (outside of a function pipeline), which is equivalent to `count(*)`.
    - Grouping by multiple columns is done via `\g1` and `\g2`, similar to sorting.


## Window functions

- Origin locations, with the destination of their most recent shipment

    ```
    shipment %%(departure_datetime \sd origin \p)%row_number = 1
    : origin
    : origin.addressee
    : destination
    : destination.addressee
    ```

- How many days into each month did it take us to reach 1000 checkouts?

    ```
    checkout %%(out_date \s out_date|year_month \p)%row_number = 1000
    : out_date|year_month
    : out_date|day_of_month
    ```

- Publications that have been on the top 10 most frequently checked-out list every month for the past year.

    TODO

## HAVING

- Doesn't exist, but you can achieve similar behavior using "pipeline of multiple queries" (described below).


## Pipeline of multiple queries

- Books that have been checked out by the same patron at least 5 times in the past year

    ```
    checkout {out_date > @1y|ago}
    :item.publication->publication \g
    :patron \g
    :%count->checkout_count
    ~~~
    {checkout_count > 5}
    :publication \g
    :patron%count->patron_count
    :checkout_count%max->max_checkouts
    ~~~
    :publication.id
    :publication.title
    :publication.author.name
    :patron_count
    :max_checkouts \sd
    ```

## UNION

- History of activity for a specific location

    ```
    shipment {origin = 7  departure_datetime != @null}
    :id :tracking_number :"Send"->action :departure_datetime->time
    +++
    shipment {destination = 7  arrival_datetime != @null}
    :id :tracking_number :"Receive"->action :arrival_datetime->time
    ~~~
    :time \s :action :tracking_number
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


## Complex examples

- Unpopular publications we should cull

    TODO
    
- Popular publications which would benefit from more copies in stock

    TODO

- Patrons who currently have the highest late fee

    ```
    checkout.$days_overdue := in_date|when(@null:@now|minus(due_date)|days *:@null)
    checkout.$late_fee := $days_overdue|when_null(0)|times(2) // $2.00 per day
    patron.$late_fee := *checkout.$late_fee%sum
    patron -[s(d)]$late_fee -*email.email%list
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




