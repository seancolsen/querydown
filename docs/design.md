# Querydown Language Design

_See also: **[Syntax Cheat Sheet](./syntax-cheat-sheet.md)**_

## Example schemas

💡 Most examples in this document are based on a **[issue-tracker sample schema](../core/resources/test/issue_schema.json)**. It has an [ER diagram](../core/resources/test/issue_schema.diagram.png) too. Understanding the schema will be important to understand some of the examples.

## Basics

Write the name of a table to select from it. When no conditions and no columns are specified, all rows and columns are returned.

```
issues
```

Here we refer to "issues" the **base table**. Every query has one and only one base table.

---

Specify columns to show by listing them prefixed with `$`

```
issues $id $title
```

---

Use `->` after a column to give it an alias.

```
issues $id->Identifier $title->Subject
```

## White space

Most white space doesn't matter. The following two queries are identical.

```
issues$id->Identifier$title->Subject
```

```
issues
$ id    -> Identifier
$ title -> Subject
```

## Quoting identifiers

If you want to reference a table name or column name which contains characters other than letters and underscores, enclose it in backticks

```
`Gala Attendees`
$ `Given Name`-> `First Name`
$ `Surname`-> `Last Name`
```


## Sorting

Ascending sorting by one column. The `s` stands for "sort".

> Issues sorted by their creation date

```
issues $title $created_at \s
```

---

Descending sorting is indicated via a `d` after the `s`.

```
issues $title $created_at \sd
```

---

Sorting by multiple columns is done via numbers to indicate ordinality.

```
issues $title \s2 $created_at \sd1
```

Sorted columns without any ordinality specified are sorted in the order the appear, after all columns with indicated ordinality.

---

By default, `NULL` values are sorted last, but this behavior can be modified using the `n` flag, which stands for "nulls first".

```
issues $title $created_at \sdn
```


## LIMIT and OFFSET

TODO

## Conditions

### AND vs OR

Curly braces enclose multiple `AND` conditions.

> Issues that are open **and** created after 2023-03-04

```
issues {status:"open" created_at:>@2023-03-04}
```

---

Square brackets enclose `OR` conditions.

> Issues that are open **or** created after 2023-03-04

```
issues [status:"open" created_at:>@2023-03-04]
```

---

If you omit the top-level braces, then a set of AND conditions is inferred.

```
issues status:"open" created_at:>@2023-03-04
```

---

Conditions can be nested

> Issues that are open and created after 2023-03-04 _or_ reopened and created after 2022-11-22:

```
issues [
  {status:"open" created_at:>@2023-03-04}
  {status:"reopened" created_at:>@2022-11-22}
]
```

---

See the [Syntax cheat sheet](./syntax-cheat-sheet.md) for a reference to all the comparison operators.


### Comparison expansion

Comparisons get expanded when one side is enclosed in brackets

> Issues that are either open or reopened:

```
issues status:["open" "reopened"]
```

> Issues that are missing a title and description:

```
issues {title description}:@null
```

> Issues where the title or description contains "foo":

```
issues [title description]:~"foo"
```

---

If both sides of the comparison are enclosed in brackets, then the brackets on left side are used for the outer precedence

> Issues where the title and description both contain "foo" or contain "bar":

```
issue {title description}:~["foo" "bar"]
```

### Ranges

> Issues created in the 2010's decade

```
issues created_at|year:~2010..2019
```

The range `2010..2019` **includes** both 2010 and 2019. You can use exclamation marks on either side of the `..` to make the range exclude either of the bounds, i.e. `2010!..2019` or `2010..!2019` or `2010!..!2019`.


## Computations and functions

Functions are applied to values via `|` syntax.

> The most overdue issues

```
issues $id $title $(deadline-@now)|days|max(0)\sd
```

Here:

1. `deadline` and `@now` are both dates. Subtracting the two produces an interval.
1. Then we pipe the interval into the `days` function to produce a number of days.
1. Then we pipe the number of days into the (scalar) `max` function, along with 0, taking the maximum of the two values. This eliminates negative numbers, replacing them with zero instead.

## Interpolated strings

Are specified via `^{value}^`

```
users $^"{username}" <{email}>^
```

## Incremental column specification

Use `$[]` to specify all columns, giving you control to add a column after all columns

> Issues with all columns, plus a special concatenation of the username and email:

```
users $[] $^"{username}" <{email}>^
```

---

Within the set of all columns, you can add column names and flags to control the behavior of columns.

---

Use `\h` to hide a column.

> Issues with all columns except description:

```
issues $[description \h]
```

---

Use `\s` (and similar flags) to sort by columns, leaving their position in the table unchanged.

```
issues $[created_at \sd]
```


## Joining related data

You can refer to data from related tables &mdash; but joins don't work quite like in SQL. In querydown, the number of rows in the results will never be more than the number of rows in the base table.


### Referring to _single related records_

When a column links to another table, the `.` character can be used after the column to refer to columns in the related table.

> Issues created by members of the backend team, displaying the issue title and author's username

```
issues author.team.name:"Backend" $id $title $author.username
```

---

> Issues for all projects under the "Foo" product which are due within two months:

```
issues project:~{deadline:<@2m|away product.name:"Foo"}
```

This expands to

```
issues project.deadline:<@2m|away project.product.name:"Foo"
```

---

You can also refer to related tables by name.

> All issues associated with the "Foo" client.

```
issues >>clients.name:"Foo"
```

This expands to:

```
issues project.product.client.name:"Foo"
```

The `>>` syntax is shorthand only works if there is one unambiguous path from the base table to the linked table. The longer form is required if there is more than one way to join the two tables.

### Referring to _multiple related records_

> Users, and the number of issues they have created

```
users $username $#issues
```

In our schema, each user has multiple issues. We use `#` to refer to a related table which has multiple records for each record in the base table.

The rows returned from the query still correspond directly to the rows in the base table &mdash; all data joined with `#` will be aggregated vs the base table. The default aggregation is to _count_ the related records.

---

Specific aggregate functions can be applied via `%` (similar to pipe syntax).

> Users, along with most recent date on which they created a ticket

```
users $username $#issues.created_at%max
```

---

You can use the `++` and `--` shorthand syntax to construct conditions based on aggregate counts

> Users that have created at least one issue

```
users ++#issues
```

This expands to 

```
users #issues:>0
```

> Users that have not created any issues

```
users --#issues
```

This expands to 

```
users #issues:0
```

---

You can add a condition block after any aggregated table

> Users who have not created any issues within the past year

```
users --#issues{created_at:>@1y|ago}
```

---

You can refer to distantly-related tables

> Clients, sorted by the highest number of associated open issues

```
clients $id $name $#issues{status:"Open"} \sd
```

Here, the `issues` table is not directly related to the `clients` table, but that's okay. The above code is shorthand for the following:

```
clients $id $name $#products.#projects.#issues{status:"Open"}
```

The shorthand works in this case because there is only one path through which `clients` can be joined to `issues`. Querydown will choose the shortest unambiguous path it can find.

---

If the related table can be joined via multiple routes which tie as being the shortest path, then Querydown will throw an error.

> Attempt to display the number of users associated with each issue.

```
issues $id $title $#users // ERROR!
```

This doesn't work because `#users` can be joined either through the `assignments` table or through the `comments` table.

This works:

> The number of unique users _who have commented_ on each ticket

```
issues $id $title $#comments.#users.id%count_distinct
```

---

If one table directly links to another table multiple times, then parentheses must be used to specify which foreign key column to use.

> Issues that are blocking other issues

```
issues ++#blocks(blocker)
```

> Issues that are not blocked by any other issues

```
issues --#blocks(blocking)
```

### Multi-column foreign keys

TODO

### Polymorphic associations

TODO

## Grouping and aggregating

Grouping is indicated by the `g` flag, similar to sorting.

> The count of tickets, by status, for the Foo project:

```
issues project.name:"Foo" $status \g $%count \sd
```

- All ungrouped columns must contain an aggregate function
- `%count` can occur on its own (outside of a function pipeline), which is equivalent to `count(*)`.
- Grouping by multiple columns is done via `\g1` and `\g2`, similar to sorting.


## Pipeline of multiple queries

Books that have been checked out by the same patron at least 5 times in the past year

```
checkout out_date:>@1y|ago
$item.publication->publication \g
$patron \g
$%count->checkout_count
~~~
checkout_count:>5
$publication \g
$patron%count->patron_count
$checkout_count%max->max_checkouts
~~~
$publication.id
$publication.title
$publication.author.name
$patron_count
$max_checkouts \sd
```

## Window functions

Window functions are defined via `%%[ ]`. Inside the braces, you use the same syntax as with incremental column, but one additional flag is available: `\p` for "partition".

After the window function definition, you apply an aggregate function, such as `row_number`, `lag`, `dense_rank`, etc.

> Issues which have a lot of sequential comments from the same user, showing the max number of sequential comments within the issue, along with the names of all the users who tied for making that many sequential comments

```qd
comments
$issue
$user
$%%[issue\p user\p created_on\s]%row_number -> count
~~~
%%[issue\p count\sd]%row_number:1
$issue \g
$count
$user.username%list
```

---

> Origin locations, with the destination of their most recent shipment

```
shipment %%[departure_datetime \sd origin \p]%row_number:1
$origin
$origin.addressee
$destination
$destination.addressee
```

---

> How many days into each month did it take us to reach 1000 checkouts?

```
checkout %%[out_date \s out_date:year_month \p]%row_number:1000
$out_date|year_month
$out_date|day_of_month
```

## UNION

The `+++` operator performs an SQL `UNION`. Tables on both sides must have identical column structures.

> History of activity for a specific location

```
@location_id = 7

shipment
origin:@location_id  departure_datetime!@null
$id $tracking_number $"Send"->action $departure_datetime->time
+++
shipment
destination:@location_id arrival_datetime!@null
$id $tracking_number $"Receive"->action $arrival_datetime->time
~~~
$time \s $action $tracking_number
```

Union has higher precedence than pipeline (the union will be performed before the pipeline). Temporary tables can be used if you need a pipeline within a union.

## Temporary tables

Pipeline within a union

``` 
@location_id = 7
#a = (
  shipment origin:@location_id  departure_datetime!@null
  $id
  $tracking_number
  $"Send" -> action
  $departure_datetime-> time
)
#b := (
    shipment destination:@location_id  arrival_datetime!@null
    $id
    $tracking_number
    $"Receive" -> action
    $arrival_datetime -> time
    ~~~
    // more here
);

a +++ b
~~~
$time \s $action $tracking_number
```

## Complex examples

- Unpopular publications we should cull

    TODO
    
- Popular publications which would benefit from more copies in stock

    TODO

- Patrons who currently have the highest late fee

    ```
    checkout.days_overdue = ? in_date:@null ~ 0 ~~ due_date|ago|days|max(0)|ceil
    checkout.late_fee = days_overdue|times(2) // $2.00 per day
    patron.late_fee = #checkouts.late_fee%sum
    patron $[] $late_fee \sd $#email.email%list
    ```

- Patrons with at least 1 year of checkout history who have never gone more than 14 days without a checkout.

    TODO

- Average days overdue, by month

    ```
    checkout.days_overdue = ? in_date:@null ~ 0 ~~ due_date|ago|days|max(0)|ceil
    checkout
    $days_overdue:>0
    $out_date|year_month \gs
    $days_overdue%avg
    ```

- Publications that have been on the top 10 most frequently checked-out list every month for the past year.

    TODO

- TODO examples
    - CRM total number of contacts who have given at least $500 total donations of certain type in past year and who are not registered for a specific event
    - CRM top ten zip codes by average amount of donations over the past 5 years of certain types
    - CRM total amount raised, by event, for all events of a certain type within the past 10 years
    - CRM total amount raised, by fiscal year, including certain contribution types




