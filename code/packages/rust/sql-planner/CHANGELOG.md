# Changelog — sql-planner

All notable changes to this package will be documented in this file.

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
