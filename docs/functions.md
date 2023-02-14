# Querydown functions

⚠️ TODO ⚠️

## Scalar functions

Applied via `|`

- `ago`
- `away`
- `is_null`
- `is_non_null`
- `has_value`
- `bool`
- `not`
- `when()`
- `if()`
- `segment()`
- `bins()`
- `above()`
- `below()`
- `else()` (i.e. `COALESCE`)
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
- `minus()`
- `plus()`
- `times()`
- `divide()`
- ...

## Aggregate functions

Applied via `%`

- `%count` (This is the only aggregate function which can also be applied to the _table_)
- `%count_distinct`
- `%sum`
- `%product`
- `%min`
- `%max`
- `%avg`
- `%list()` (i.e. `group_concat` or `string_agg`) This function accepts a `separator` argument. TODO: how to sort entries
