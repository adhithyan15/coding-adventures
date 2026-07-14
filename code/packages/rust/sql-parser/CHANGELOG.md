# Changelog

All notable changes to this project will be documented in this file.

## [0.1.11] - Unreleased

### Added

- **`COLLATE name` clause in `ORDER BY`.** The `order_item` grammar rule gains
  an optional `[ "COLLATE" NAME ]` between the sort expression and the `ASC` /
  `DESC` direction (`expr COLLATE name [ASC|DESC] [NULLS ...]`), matching
  SQLite's grammar. `COLLATE` is matched by literal text and the collation name
  is an ordinary NAME token (validated in the planner), so no lexer keyword was
  added. Existing `ASC`/`DESC`/`NULLS` ordering is unaffected.

## [0.1.10] - Unreleased

### Added

- **`IS <expr>` / `IS NOT <expr>` (null-safe (in)equality).** Two `comparison`
  alternatives added AFTER the `IS NULL` / `IS NOT NULL` sequences (so those
  still match first) and with `IS NOT <expr>` before `IS <expr>`. The planner
  (sql-planner 0.2.9) lowers them onto a CASE, so no codegen/VM change.

## [0.1.9] - Unreleased

### Added

- **Searched `CASE WHEN … THEN … [ELSE …] END`.** Added to the `primary` rule
  (before `function_call`): one `WHEN/THEN` is mandatory, further ones repeat,
  and `ELSE` is optional. Conditions and values are full `expr`s. The planner
  (sql-planner 0.2.8) turns it into `SqlExpr::Case`.

## [0.1.8] - Unreleased

### Added

- **`NULLS FIRST` / `NULLS LAST` in `ORDER BY`.** `order_item` now accepts an
  optional `NULLS <name>` clause after the ASC/DESC direction. `FIRST`/`LAST`
  are NOT reserved keywords (they are common column names), so a generic NAME is
  accepted and the planner (sql-planner 0.2.7) validates it. Omitting the clause
  keeps SQLite's default (NULLs first for ASC, last for DESC).

## [0.1.7] - Unreleased

### Added

- **`CAST(expr AS type)` expressions.** A `CAST` alternative was added to the
  `primary` rule, before `function_call` so the ordered-choice parser matches
  the AS-typed form first. The type is a `NAME` token (INTEGER/REAL/TEXT and
  their affinity synonyms). The planner (sql-planner 0.2.6) turns it into a
  `SqlExpr::Cast`.

## [0.1.6] - Unreleased

### Added

- **`GLOB` and `NOT GLOB` infix operators.** `WHERE s GLOB 'x*'` now parses —
  case-sensitive Unix-glob matching (`*` any run, `?` one char, `[…]` classes).
  Added as `comparison` alternatives mirroring `LIKE`/`NOT LIKE`. The planner
  (0.2.5) lowers `X GLOB Y` onto the existing `glob(Y, X)` builtin, so no new
  codegen/VM opcode was needed.

## [0.1.5] - Unreleased

### Added

- **`LIMIT off, count` MySQL shorthand.** The `limit_clause` tail now accepts
  `, NUMBER` as an alternative to `OFFSET NUMBER`, so `LIMIT 1, 2` parses.
  SQLite accepts this MySQL-compatibility spelling. Note the argument order
  flips: in the comma form the FIRST number is the offset and the SECOND is
  the count (`LIMIT 1, 2` == `LIMIT 2 OFFSET 1`) — the sql-planner 0.2.4 change
  does the swap.

## [0.1.4] - Unreleased

### Fixed

- **A table alias may omit the `AS` keyword.** `FROM users u` now aliases the
  table exactly like `FROM users AS u` — SQLite and standard SQL accept both.
  The generated `table_ref` grammar required `AS`; it now matches the
  `sql.grammar` source (`table_name [ [ "AS" ] NAME ]`) by making `AS`
  optional. The alias `NAME` cannot swallow a following keyword
  (`JOIN`/`WHERE`/`ON`/…) — `NAME` only matches `Name`-type tokens — so bare
  aliases work across joins (`FROM a x JOIN b y ON …`). Mirrors the 0.1.3
  column-alias fix; paired with a sql-planner 0.2.3 change that reads the
  implicit table alias.

## [0.1.3] - Unreleased

### Fixed

- **A column alias may omit the `AS` keyword.** `SELECT a col1` now parses and
  names the output column `col1`, exactly like `SELECT a AS col1` — SQLite and
  standard SQL accept both spellings. The generated `select_item` grammar
  required `AS`; it now matches the `sql.grammar` source
  (`expr [ [ "AS" ] NAME ]`) by making `AS` optional. The alias `NAME` cannot
  swallow a following keyword (`FROM`/`WHERE`/…) — `NAME` only matches
  `Name`-type tokens — nor a comma, so `SELECT a, b` and `SELECT a FROM t` are
  unaffected. Paired with a sql-planner 0.2.2 change that reads a bare trailing
  alias token (no `AS` keyword to key off).

## [0.1.2] - Unreleased

### Fixed

- **A join with no `ON` condition** (a Cartesian product) now parses:
  `FROM a JOIN b` and `FROM a CROSS JOIN b`. The generated grammar required an
  `ON expr` after every join; the `ON expr` is now `Optional`. The planner already
  returns `None` for a missing `ON`, and codegen already emits every pair (no
  condition check) for a conditionless INNER join, so no downstream change was
  needed.

## [0.1.1] - Unreleased

### Fixed

- **Bare `JOIN`** (without an `INNER`/`LEFT`/… keyword) now parses. The generated
  grammar's `join_clause` required a `join_type` before `JOIN`, so `FROM a JOIN b`
  failed while `FROM a INNER JOIN b` worked. `join_type` is now `Optional`,
  matching the `sql.grammar` source (`[ join_type ]`); the planner already
  defaults a missing `join_type` to INNER, so no downstream change was needed.

## [0.1.0] - 2026-03-23

### Added

- Initial release of the SQL parser crate.
- `create_sql_parser()` factory function returning `Result<GrammarParser, String>` configured for SQL.
- `parse_sql()` convenience function returning `Result<GrammarASTNode, String>` directly.
- Loads the `sql.grammar` file at runtime from the shared `grammars/` directory.
- Parses all SQL statement types: SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE.
- Full expression hierarchy: OR → AND → NOT → comparison → additive → multiplicative → unary → primary.
- Comparison operators: =, !=/<>, <, >, <=, >=, BETWEEN, IN, LIKE, IS NULL, IS NOT NULL.
- GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET clauses in SELECT.
- JOIN clauses: INNER, LEFT [OUTER], RIGHT [OUTER], FULL [OUTER], CROSS.
- Column constraints in CREATE TABLE: NOT NULL, PRIMARY KEY, UNIQUE, DEFAULT.
- Result-returning API for clean error propagation.
- 33 unit tests covering: all statement types, expression precedence, case-insensitive keywords, DISTINCT, NULL/TRUE/FALSE literals, function calls, qualified column references, multiple statements, trailing semicolons, BETWEEN, IN, LIKE, IS NULL, AND/OR/NOT, GROUP BY, HAVING, ORDER BY, LIMIT, arithmetic expressions, factory function, invalid SQL error path, tokenization error propagation.
