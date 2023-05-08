# E2E tests to incorporate

## ⛔ Variables

### Search points

```qd
@@search_points = field_value search_value; ?
  field_value:search_value => 2
  field_value:~search_value => 1
  *=> 0
@search = "foo"
@@points = field; field|search_points(@search)
people.points = @@max(first_name|points last_name|points)
people $[] $points \sd :::limit(10)
```

### Drinking age

```qd
@drinking_age = 21
users.age = birth_date|age|years
users.can_purchase_alcohol = age:>=@drinking_age
users $can_purchase_alcohol \g $%count
```

### Generation

```qd
@@generation = birth_date; birth_date|year|(birth_year; ?
  birth_year:>=2010 => "Alpha"
  birth_year:>=1997 => "Z"
  birth_year:>=1981 => "Millennial"
  birth_year:>=1965 => "X"
  birth_year:>=1946 => "Boomer"
  birth_year:>=1928 => "Silent"
  birth_year:>=1901 => "Greatest"
  birth_year:>=1883 => "Lost"
  * => @null)
people.generation = birth_date|generation
```

### Completion ratio by client

```qd
clients.open_count = #issues{status:"open"}
clients.closed_count = #issues{status:"closed"}
clients.completion = closed_count/(closed_issues+open_count)
clients $[] $completion \s
```

