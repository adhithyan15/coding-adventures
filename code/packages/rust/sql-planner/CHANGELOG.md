# Changelog — sql-planner

All notable changes to this package will be documented in this file.

## [0.2.8] - Unreleased

### Added

- **`SqlExpr::Case { branches, else_val }`** for searched CASE, plus `plan_case`
  which walks the `CASE`/`WHEN`/`THEN`/`ELSE`/`END` keyword tokens and their
  interleaved expression nodes into `(cond, value)` branch pairs and an optional
  ELSE. Rejects a CASE with no `WHEN` branch.

## [0.2.7] - Unreleased

### Added

- **`SortKey.nulls_first: Option<bool>`** — explicit NULL placement from a
  `NULLS FIRST`/`NULLS LAST` clause (`None` = SQLite default = NULLs first for
  ASC, last for DESC). `plan_order_item` parses the clause and rejects anything
  other than FIRST/LAST after `NULLS`. Threaded to codegen + the VM comparator.

## [0.2.6] - Unreleased

### Added

- **`CAST(expr AS type)` planning.** New `SqlExpr::Cast { expr, ty }` variant
  and `CastType { Integer, Real, Text }` enum. `plan_cast` extracts the inner
  expression and resolves the declared type name to a `CastType` using SQLite's
  substring **affinity** rule (so `INT`/`VARCHAR`/`FLOAT` synonyms resolve
  correctly). `BLOB` and `NUMERIC` are not yet supported and return an
  `UnsupportedStatement` error (a later increment). Codegen/VM apply the
  conversion (sql-codegen 0.6.1, sql-vm 0.4.12).

## [0.2.5] - Unreleased

### Added

- **`GLOB` / `NOT GLOB` operator lowering.** `plan_comparison` now recognises
  the `GLOB` token (sql-parser 0.1.6) and lowers `X GLOB Y` onto the existing
  `glob` builtin as `FunctionCall { name: "glob", args: [Y, X] }` — SQLite
  defines the operator exactly as `glob(Y, X)` (pattern first). `NOT GLOB`
  wraps the call in `UnaryOp::Not`. No dedicated `Glob` expr node, codegen, or
  VM opcode was added — it reuses the function-call path end to end.

## [0.2.4] - Unreleased

### Added

- **`LIMIT off, count` planning (MySQL shorthand).** `plan_limit` now detects
  the comma form (sql-parser 0.1.5) and swaps the operands: in `LIMIT o, c`
  the first number is the offset and the second is the count, the reverse of
  `LIMIT c OFFSET o`. Both spellings now produce the identical `Limit { count,
  offset }` plan, so `LIMIT 1, 2` and `LIMIT 2 OFFSET 1` return the same rows.
  The rewrite collects the numeric operands positionally and maps them onto
  `(count, offset)` per the detected form; the `LIMIT -1` "no limit" sign
  handling for the `OFFSET` form is preserved.

## [0.2.3] - Unreleased

### Fixed

- **Bare table aliases (no `AS`).** `extract_table_ref` keyed the table alias
  off the `AS` keyword, so `FROM users u` (which sql-parser 0.1.4 now accepts)
  lost its alias and qualified references like `u.id` failed to resolve. It now
  also recognises the implicit form: with the table name nested in its own
  `table_name` node, a bare `Name`-type token directly under `table_ref` is the
  alias. Guarded on the `table_name` node being present, so the degenerate
  no-node fallback (where the lone token *is* the table name) is unaffected.
  Mirrors the 0.2.2 column-alias fix.

## [0.2.2] - Unreleased

### Fixed

- **Bare column aliases (no `AS`).** `extract_as_alias` keyed the output-column
  alias off the `AS` keyword, so `SELECT a col1` (which sql-parser 0.1.3 now
  accepts) lost its alias. It now also recognises the implicit form: when there
  is no `AS`, the lone `Name`-type token directly under `select_item` is the
  alias. The expression is always a nested node, so a bare identifier token can
  only be the alias — no ambiguity. `SELECT a` (no alias) still yields `None`.

## [0.2.1] - Unreleased

### Fixed

- **`''` unescaping in string literals.** A `String` token's value is the raw
  inner text (the lexer strips only the surrounding quotes), so a doubled single
  quote must be collapsed to one when the literal is built:
  `'it''s'` → the string `it's`. Paired with the sql-lexer 0.1.1 tokenizer fix.

## [0.2.0] - Unreleased

### Fixed

- **Multi-argument `MIN`/`MAX` are now the SCALAR functions, not the aggregate.**
  `MIN`/`MAX` are overloaded in SQL: one argument is the aggregate (min/max over
  a column), but two-or-more is the scalar that returns the smallest/largest of
  its arguments. Both aggregate-detection sites (`try_plan_as_aggregate` and
  `plan_function_call`) previously treated *any* `MIN`/`MAX` as the aggregate, so
  `SELECT MAX(3, 9, 5)` used only the first argument and returned `3` instead of
  `9`. They now check the argument count (new `call_arg_count` helper) and leave
  the 2+-argument form as a `FunctionCall`, routed to the VM's `call_builtin`.

## [0.1.0] — 2026-06-30

### Added

- `SqlExpr` — recursive SQL expression enum with variants:
  `Literal`, `Column`, `BinaryOp`, `UnaryOp`, `IsNull`, `IsNotNull`,
  `Between`, `Like`, `InList`, `FunctionCall`, `Aggregate`
- `BinaryOp` — arithmetic (`+`, `-`, `*`, `/`, `%`), comparison
  (`=`, `!=`, `<`, `<=`, `>`, `>=`), logical (`AND`, `OR`), concatenation (`||`)
- `UnaryOp` — `Neg` (unary minus), `Not` (logical NOT)
- `AggFunc` — `Count`, `Sum`, `Avg`, `Min`, `Max`
- `LogicalPlan` — tree of plan nodes:
  `Scan`, `Filter`, `Project`, `Join`, `Aggregate`, `Having`,
  `Sort`, `Limit`, `Distinct`, `Union`, `Insert`, `Update`,
  `Delete`, `CreateTable`, `DropTable`
- `OutputColumn`, `JoinKind`, `SortKey`, `AggregateItem`, `Assignment`, `InsertSource`
- `PlanError` — `UnknownTable`, `UnknownColumn`, `UnsupportedStatement`,
  `ParseError`, `AmbiguousColumn`
- `plan(ast, schema) -> Result<LogicalPlan, PlanError>` — plan from pre-parsed AST
- `plan_sql(sql, schema) -> Result<LogicalPlan, PlanError>` — parse + plan in one step
- `plan_expr(node) -> Result<SqlExpr, PlanError>` — plan a standalone expression node
- Full SELECT pipeline: `Scan → Filter → Aggregate → Having → Distinct → Sort → Limit → Project`
  with `Project` always outermost (per lessons.md critical ordering requirement)
- DML planners: INSERT (with/without column list, multi-row VALUES),
  UPDATE (multiple assignments, optional WHERE), DELETE (optional WHERE)
- DDL planners: CREATE TABLE (with IF NOT EXISTS, column constraints),
  DROP TABLE (with IF EXISTS)
- JOIN support: INNER, LEFT, RIGHT, FULL, CROSS joins with ON condition
- Expression planner covering full SQL expression grammar:
  OR → AND → NOT → comparison → additive → multiplicative → unary → primary
- Schema validation: unknown tables produce `PlanError::UnknownTable`
- 58 unit tests with `MockSchema`
