<!-- learning-concepts: mini-sqlite, sql-backend, sql-execution-engine, sql-lexer, sql-parser, sql-csv-source -->
# From SQL Text To Stored Rows

SQL is declarative: a query describes the result you want, not the loop that
produces it. A database therefore has to translate one statement through
several representations before it can touch stored data.

```text
SQL text
  -> tokens
  -> syntax tree
  -> bound and planned operations
  -> executable operators
  -> backend reads and writes
  -> result rows
```

Each stage removes one kind of ambiguity.

## Lexing And Parsing

The lexer recognizes keywords, identifiers, literals, punctuation, and
operators. Context matters: quoted identifiers and string literals may use
similar delimiters but have different escaping rules. Numeric tokens should
not silently inherit the host language's number grammar.

The parser turns tokens into structure. For:

```sql
SELECT department, COUNT(*) AS people
FROM employees
WHERE active = TRUE
GROUP BY department
HAVING COUNT(*) >= 2
ORDER BY people DESC
LIMIT 5;
```

the AST records clauses and expressions, but it still does not know whether
`employees` exists or what type `active` has.

## Binding Gives Names Meaning

Binding resolves table names, columns, aliases, functions, and parameters
against a catalog or data-source interface. It should detect an unknown or
ambiguous column before execution.

Names pass through scopes. A table alias is visible to expressions that read
input rows; a select-list alias may be visible to `ORDER BY` but not necessarily
to `WHERE`. SQL dialects differ, so these rules belong in an explicit contract.

Binding can also assign types and insert legal coercions. This keeps execution
operators focused on values whose shapes are already known.

## Written Order Is Not Evaluation Order

The logical order of a `SELECT` query is approximately:

```text
FROM and JOIN
WHERE
GROUP BY and aggregates
HAVING
SELECT
DISTINCT
ORDER BY
OFFSET and LIMIT
```

This explains several otherwise surprising rules:

- `WHERE` cannot filter on an aggregate because groups do not exist yet.
- `HAVING` can filter groups because it runs after grouping.
- `DISTINCT` applies to projected rows, not raw input rows.
- `LIMIT` must run after sorting if the query asks for the top rows by order.

An educational engine can materialize the complete row set between stages. A
production-oriented engine often uses iterator operators that request one row
at a time:

```text
Limit
  -> Sort
    -> Project
      -> Filter
        -> TableScan
```

The materialized model is easier to inspect. The iterator model can stream and
stop early. Both should implement the same logical semantics.

## Rows, Values, And NULL

A row maps columns to SQL values. SQL `NULL` means unknown or absent, not zero,
an empty string, or false.

Comparisons use three-valued logic:

| Expression | Result |
| --- | --- |
| `1 = 1` | true |
| `1 = 2` | false |
| `1 = NULL` | unknown |
| `NULL = NULL` | unknown |

`WHERE` retains only true; false and unknown are filtered out. Testing host
language truthiness instead of SQL truth tables creates subtle errors in joins,
filters, and constraints.

Aggregates also define NULL behavior. `COUNT(*)` counts rows, `COUNT(column)`
counts non-NULL values, and most other aggregates ignore NULL inputs but return
NULL when no non-NULL value exists.

## The Backend Boundary

The query engine should not know whether rows came from a CSV file, an in-memory
table, or SQLite pages. A backend contract can expose operations such as:

```text
list tables
describe columns
scan rows
insert, update, delete
begin, commit, rollback
```

A CSV source is a useful first backend. It proves that SQL evaluation can be
separated from storage mechanics. It also exposes schema questions: are all
fields strings, are types inferred, and what happens when rows have different
widths?

Pushdown is an optimization across this boundary. Instead of scanning every row
and filtering later, the engine may ask a capable backend to apply a predicate,
projection, or limit. The result must remain identical whether pushdown occurs.

## Planning And Optimization

A logical plan says what operations implement the query. A physical plan chooses
how:

```text
logical join
  -> nested-loop join
  -> hash join
  -> index lookup join
```

An optimizer uses equivalence rules and cost estimates. It may push a filter
closer to a scan, reorder inner joins, replace a full scan with an index lookup,
or remove unused columns. Every rewrite needs two arguments:

1. the new plan is semantically equivalent;
2. the estimated execution cost is better.

Correctness comes first. Differential tests can execute optimized and
unoptimized plans and compare complete results.

## What Mini-SQLite Adds

A SQLite-compatible engine composes SQL semantics with a durable file format.
Below the query layer it must understand:

- fixed-size pages and page 1 metadata;
- B-tree interior and leaf cells;
- variable-length records and serial types;
- overflow pages for large payloads;
- indexes and row identifiers;
- transactions, locking, journals, or WAL;
- SQLite's detailed function, coercion, ordering, and NULL behavior.

Compatibility is observable behavior, not merely accepting the same syntax.
The strongest test is differential:

```text
same database + same SQL
    |
    +--> repository engine -> rows or error
    +--> SQLite oracle     -> rows or error
                              |
                              v
                         compare exactly
```

When results differ, classify the fault by stage: tokenization, parsing,
binding, expression semantics, logical order, planning, execution, or storage.
That classification turns a giant "SQL is wrong" problem into a tractable one.

Detailed repository contracts live in
[`sql-execution-engine.md`](../../specs/sql-execution-engine.md),
[`sql-backend.md`](../../specs/sql-backend.md), and the
[`mini-sqlite-full-conformance.md`](../../specs/mini-sqlite-full-conformance.md)
roadmap.
