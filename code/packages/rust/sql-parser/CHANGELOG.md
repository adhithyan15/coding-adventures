# Changelog

All notable changes to this project will be documented in this file.

## [0.1.23] - Unreleased

### Changed

- **A bare `*` is now a `select_item` alternative, so it composes in a comma
  list.** `select_list` was `STAR | select_item { "," select_item }` — `*` was
  only accepted as the ENTIRE list, so `SELECT a, *` and `SELECT *, a` failed to
  parse. `select_list` is now simply `select_item { "," select_item }` and
  `select_item` is `STAR | ( expr [ COLLATE name ] [ [ "AS" ] NAME ] )` (STAR
  tried first by ordered choice). Bare `SELECT *` still parses (via the STAR
  alternative); `SELECT a`, `count(*)`, and `a * b` fall through to the expr form
  because their first token is not a bare `*`. The planner emits a `*` placeholder
  per wildcard item, which `expand_star_columns` expands in place.

## [0.1.22] - Unreleased

### Added

- **Optional trailing `COLLATE name` on select-items and GROUP BY keys.** The
  `select_item` rule now parses `expr COLLATE name [ alias ]` (so `SELECT DISTINCT
  b COLLATE NOCASE` is accepted) and `group_clause` parses a per-key `COLLATE
  name` tail (so `GROUP BY b COLLATE NOCASE`, and per-key `GROUP BY g, b COLLATE
  NOCASE`). Both mirror ORDER BY's existing `COLLATE` tail: `COLLATE` is matched
  as literal text (it is not a lexer keyword, so it and the collation name arrive
  as `NAME` tokens) and the collation name is validated in the planner. Previously
  a `COLLATE` suffix parsed only inside a comparison operand.

## [0.1.21] - Unreleased

### Added

- **`COLLATE name` may now precede the `IN` operator** in the `comparison` rule
  (`x COLLATE NOCASE IN (…)`, and `NOT IN`). An optional `[COLLATE NAME]` prefix
  was added to the `IN` / `NOT IN` alternatives. As with the existing left
  `COLLATE` on a `cmp_op` comparison, the clause lives INSIDE each alternative —
  not hoisted before the whole alternation — so a bare trailing `COLLATE` (e.g.
  `ORDER BY x COLLATE NOCASE`, where `order_item` owns the clause) still fails the
  alternative, backtracks, and leaves the token for the caller.
- **`COLLATE name` may now precede `BETWEEN`** (`x COLLATE NOCASE BETWEEN a AND
  c`, and `NOT BETWEEN`). Same optional `[COLLATE NAME]` prefix on the `BETWEEN`
  / `NOT BETWEEN` alternatives, with the same inside-the-alternative placement
  for correct backtracking.
- **`COLLATE name` may now precede `LIKE`/`GLOB`** (all four LIKE variants
  incl. `ESCAPE`, and `GLOB`/`NOT GLOB`). SQLite parses this but LIKE/GLOB ignore
  the collation, so the planner validates the name and discards it. Added for
  parse-surface parity — mini previously rejected `COLLATE` before these.

## [0.1.20] - Unreleased

### Added

- **`primary` now accepts a `BLOB` literal token** (`x'…'` / `X'…'`), placed
  after `STRING` in the alternation. The planner decodes the hex body into raw
  bytes. Pairs with sql-lexer 0.1.3's `BLOB_HEX` token.

## [0.1.19] - Unreleased

### Added

- Grammar for the `LIKE pattern ESCAPE ch` clause (and `NOT LIKE … ESCAPE`). The
  ESCAPE variants are ordered before the plain `LIKE pattern` alternatives so the
  backtracking parser prefers the longer match when an `ESCAPE` clause is present.

## [0.1.18] - Unreleased

### Added

- **Column-level `COLLATE name` in `CREATE TABLE`.** `col_constraint` now accepts
  a `COLLATE NAME` alternative (`x TEXT COLLATE NOCASE`), mirroring the existing
  `order_item` COLLATE clause. `COLLATE` arrives as a NAME token matched
  case-insensitively; the collation name that follows is a generic NAME the
  planner validates. Enables sql-planner to persist and honour column-defined
  collations.

## [0.1.17] - Unreleased

### Added

- **Scalar subquery `( SELECT … )` parses as a `primary`.** The generated
  `primary` rule gains `"(" select_stmt ")"` before the plain `"(" expr ")"` form
  (matching the grammar source), so a subquery is matched when the token after `(`
  is `SELECT` and a non-SELECT `(` backtracks to the expression form. Parsing is
  wired; evaluation is a follow-up (the planner errors NYI).

## [0.1.16] - Unreleased

### Added

- **`COLLATE name` on the LEFT comparison operand** (`col COLLATE NOCASE = 'x'`),
  mirroring the right-operand COLLATE. Placed at the START of the `cmp_op`
  alternative so a trailing `COLLATE` with no following operator (an
  `ORDER BY … COLLATE …` clause, whose `order_item` owns the collation)
  backtracks and is left for that caller — no regression to sort collation. The
  planner already takes the first COLLATE token, so a left collation wins over a
  right one, matching SQLite.
## [0.1.15] - Unreleased

### Added

- **`IS [NOT] DISTINCT FROM`** in comparisons — the standard-SQL spelling of the
  null-safe compare. The grammar adds `IS DISTINCT FROM <expr>` and `IS NOT
  DISTINCT FROM <expr>` sequences ahead of the plain `IS [NOT] <expr>` forms so
  ordered choice matches the DISTINCT keyword first.

## [0.1.14] - Unreleased

### Added

- **`COLLATE name` on a comparison's right operand.** The `comparison` grammar
  accepts an optional `[COLLATE NAME]` after the right-hand `bitwise` operand
  (`col = 'x' COLLATE NOCASE`, `a < b COLLATE RTRIM`). The planner applies the
  collation to both sides.

## [0.1.13] - Unreleased

### Added

- **Bitwise operators in the expression grammar.** Wired the generated grammar's
  `bitwise` precedence level (`bitwise = additive {{ ("&"|"|"|"<<"|">>") additive }}`)
  between `additive` and `comparison` — all four are one left-associative level,
  so `5 | 3 & 2` = `(5|3)&2`. Added the `~` (and `+`) prefix to `unary`. Both were
  present in the grammar source but stale in the generated parser.

## [0.1.12] - Unreleased

### Added

- **Simple (operand) `CASE` form.** The `case_expr` grammar gains an optional
  operand expression between `CASE` and the first `WHEN`, so
  `CASE x WHEN v THEN r … END` parses alongside the searched
  `CASE WHEN cond THEN r … END`. `WHEN` is a keyword (not a NAME), so the
  optional operand never swallows the searched form's `WHEN`. The planner tells
  the two forms apart and desugars the simple form.

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
