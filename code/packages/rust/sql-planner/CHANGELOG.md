# Changelog — sql-planner

All notable changes to this package will be documented in this file.

## [0.2.28] - Unreleased

### Fixed

- **An ORDER BY key naming an output ALIAS no longer inherits an unrelated
  column's `COLLATE`.** `plan_order_item` resolved a bare ORDER BY name straight
  against the base table, so `SELECT x AS c FROM t ORDER BY c` — where `x` is
  BINARY but the table also has a `c TEXT COLLATE NOCASE` — sorted
  case-insensitively, silently borrowing the collating sequence of a column the
  query never referenced. The key now resolves through the output list first:
  when an UNQUALIFIED name matches an output alias, the collation comes from what
  that alias STANDS FOR. So `x AS c ... ORDER BY c` is byte order, while
  `c AS y ... ORDER BY y` still folds NOCASE — the collation follows the
  expression, not the label. A qualified name (`t.c`) is always the column and is
  unaffected, as is an explicit `COLLATE` on the key, which still wins outright.
  Found while testing DISTINCT collation; same name-shadowing trap, different
  code path.

## [0.2.27] - Unreleased

### Added

- **Column-defined `COLLATE` flows into DISTINCT.** `LogicalPlan::Distinct` now
  carries one collation per OUTPUT column, computed by the new
  `distinct_output_collations`. SQLite folds a DISTINCT column only when the
  output expression is a BARE COLUMN REFERENCE to a column that declares a
  collation — verified against real SQLite: `c` folds; `*` folds (it expands to
  bare columns); `c AS y` folds (an alias is only a label — the collation comes
  from the expression); `x AS c` does NOT fold (the name `c` is irrelevant, `x`
  is BINARY); `c||''` does NOT fold (an expression drops the collation). The
  mapping is therefore POSITIONAL, never by output name — keying by name would
  wrongly fold `x AS c`. `*` is expanded through the schema's ordered
  `column_names` so the vector stays aligned with what is emitted. Single base
  table only (no JOINs), matching the other collation passes.

## [0.2.26] - Unreleased

### Added

- **Column-defined `COLLATE` flows into GROUP BY keys.** A key that is a bare
  reference to a column declared `COLLATE NOCASE` is wrapped in
  `__collate(col, 'NOCASE')`, so `GROUP BY c` groups `'A'` with `'a'` — reusing
  the representation the ORDER BY and WHERE collation passes already use, via
  the shared `build_collate_ctx` / `resolve_column_collation` helpers. Codegen
  peels the wrapper back off so the collation folds only the grouping key and
  the group still reports its original text. Restricted to a single base table
  (no JOINs), matching the existing ORDER BY / WHERE restriction, since a bare
  column's owning table is ambiguous under a join. A key that already carries an
  explicit `COLLATE` is left alone — explicit outranks declared.

## [0.2.25] - Unreleased

### Added

- **Hexadecimal integer literals (`0x1F`).** The number-literal decoder in
  `plan_primary_token` now recognises a `0x` / `0X` prefix and decodes the hex
  digits as a 64-bit value that wraps: `0x1F` → 31, `0x7FFFFFFFFFFFFFFF` →
  `i64::MAX`, and `0xFFFFFFFFFFFFFFFF` → -1 (parsed as `u64` then bit-cast to
  `i64`, matching SQLite — an `i64` parse would reject values above `i64::MAX`).
  These are always INTEGERs (`typeof(0x1F)` = `'integer'`). More than 16 hex
  digits overflows 64 bits; SQLite rejects that at prepare time ("hex literal too
  big"), so we return a `PlanError::UnsupportedStatement` rather than silently
  falling through to a (nonsensical) column reference — a hex token always starts
  with a digit, so it can never legitimately be a column name. Pairs with the new
  `HEX_INT` token in `sql-lexer` (aliased to `NUMBER`, so no parser rule changed).

## [0.2.24] - Unreleased

### Added

- **`GROUP_CONCAT` recognised as an aggregate.** `try_plan_as_aggregate` now maps
  `GROUP_CONCAT` to `AggFunc::GroupConcat { sep }`, capturing the separator from
  an optional literal second argument (default `","`) via `group_concat_separator`
  / `collect_call_arg_exprs`; the value stream stays single-column (`x`). Pairs
  with sql-codegen 0.6.7 / sql-vm 0.4.30.

## [0.2.23] - Unreleased

### Added

- **Explicit `COLLATE` written before `IN` now applies to the membership test.**
  `plan_comparison`'s `IN`/`NOT IN` branch reads the collation via the existing
  `collate_name_after` and, when present, wraps the value AND every list element
  in `__collate(_, name)` — the same canonicalise-then-byte-compare mechanism the
  `cmp_op` branch uses. So `name COLLATE NOCASE IN ('apple')` lifts a plain column
  to case-insensitive membership, and `name COLLATE BINARY IN (…)` overrides a
  column's declared NOCASE. `__collate` passes NULL/non-text through unchanged, so
  IN's three-valued NULL logic and numeric equality for non-text members are
  preserved. This is the *explicit*-COLLATE path; the *column-defined* collation
  (0.2.22) is still pushed in by `collate_comparisons`, which yields to an
  already-`__collate`-wrapped operand so an explicit clause always wins.
- **Explicit `COLLATE` before `BETWEEN` applies to the range test.** The
  `BETWEEN`/`NOT BETWEEN` branch wraps the value AND both bounds in
  `__collate(_, name)`. Since `x BETWEEN a AND c` is `x >= a AND x <= c`, this
  makes both ordered comparisons canonicalise their text before the byte compare
  — a collated range test. `'B' COLLATE NOCASE BETWEEN 'a' AND 'c'` is now `1`
  (the folded `'b'` falls in `a..c`) where the byte compare is `0`.
- **`COLLATE` before `LIKE`/`GLOB` is validated and ignored.** The LIKE and GLOB
  branches call `collate_name_after` so an unknown collation still errors, then
  discard the name — no `__collate` wrap. Matches SQLite, where LIKE/GLOB carry
  their own case-folding and the `COLLATE` operator has no effect on them (even
  `COLLATE BINARY` does not make LIKE case-sensitive).

## [0.2.22] - Unreleased

### Added

- **Column-defined `COLLATE` now flows into the `IN` operator.** The
  `collate_comparisons` post-pass gained an `SqlExpr::InList` arm: SQLite takes
  IN's collating sequence from the left operand, so when `value` is a base-table
  column with a declared collation (and not already `__collate`-wrapped by an
  explicit `COLLATE`), the value and every list element are wrapped in
  `__collate(_, coll)`. `name IN ('APPLE')` on a NOCASE column now matches
  `'Apple'`/`'apple'`; `NOT IN` inverts it; an explicit `COLLATE BINARY` on the
  value overrides the column's NOCASE. `__collate` passes NULL/non-text through
  unchanged, so IN's NULL semantics are preserved. Extends the WHERE-comparison
  collation from 0.2.20. `DISTINCT`/`GROUP BY` collation remain follow-ups.

## [0.2.21] - Unreleased

### Added

- **Blob literals `x'…'` / `X'…'` plan to `SqlValue::Blob`.** `plan_primary_token`
  recognises the lexer's `BLOB` token (via `type_name`) and decodes the hex body
  into raw bytes through the new `decode_blob_literal` helper: `x'414243'` → the
  three bytes `41 42 43`, `X'FF00'` → `FF 00`, and the empty literal `x''` → the
  zero-byte blob. An odd number of hex digits is rejected with a plan error,
  matching SQLite's tokenizer (`x'012'` is a syntax error there); a non-hex digit
  is guarded defensively even though the lexer regex already excludes it. The VM
  already handled `Blob` values (HEX/QUOTE/TYPEOF/equality), so this closes the
  front-end gap that prevented producing them. `LENGTH()` over a blob (byte count)
  is a separate VM-builtin follow-up.

## [0.2.20] - Unreleased

### Added

- **Column-defined `COLLATE` now flows into WHERE comparisons.** A column
  declared `COLLATE NOCASE` / `RTRIM` previously only affected ORDER BY; bare
  comparisons in a WHERE predicate compared with BINARY, diverging from SQLite.
  A post-planning pass (`collate_comparisons`) over the WHERE predicate now
  wraps both operands of a comparison (`=`, `<>`, `<`, `<=`, `>`, `>=`) in the
  internal `__collate(_, coll)` when a column operand declares a collation —
  reusing the exact mechanism an explicit `COLLATE` clause already uses — so the
  byte comparison honours the column's sequence.
  - SQLite's collation-resolution order is followed: an explicit `COLLATE` on
    either operand wins; else the left operand's column collation; else the
    right operand's; else BINARY. The pass recurses through `AND`/`OR`/`NOT`.
  - An explicit `COLLATE BINARY` now correctly **overrides** a column's NOCASE:
    `collate_name_after` reports BINARY (instead of collapsing it to "no
    collation"), so it is lowered to the identity `__collate(_, 'BINARY')`,
    which both forces byte order and marks the comparison as explicitly
    collated so the column-collation pass leaves it alone.
  - Applies to a single base table (no JOINs), matching the ORDER BY
    restriction. `DISTINCT`, `GROUP BY`, and `IN` collation remain follow-ups.

## [0.2.19] - Unreleased

### Added

- **Positional `ORDER BY <n>` (ordinal column references).** A *bare integer
  literal* in an ORDER BY term is now resolved to the n-th (1-based) column of
  the SELECT output list, matching SQLite: `SELECT a, b FROM t ORDER BY 2` sorts
  by `b`. Direction, `NULLS FIRST/LAST`, `COLLATE`, and multi-key tie-breaks all
  carry through, because the ordinal is rewritten to the real output expression
  and then flows through the ordinary sort machinery.
  - The rule is deliberately narrow — only a lone integer literal is positional.
    An *expression* that evaluates to an integer is **not**: `ORDER BY 1+0` sorts
    by the constant `1` (no reordering), exactly as in SQLite.
  - An out-of-range ordinal (`< 1` or `> column count`) is a planning error,
    matching SQLite's prepare-time "ORDER BY term out of range" — diagnosed only
    when the select list is fully explicit.
  - `SELECT *` positional keys are left unchanged (the column count/identity is
    not known at plan time); positional-over-star and positional-over-aggregate
    remain follow-ups.

## [0.2.18] - Unreleased

### Added

- `SqlExpr::Like` gained an `escape: Option<Box<SqlExpr>>` field, and the
  comparison parser now reads the optional `ESCAPE ch` operand of `LIKE` /
  `NOT LIKE`.

## [0.2.17] - Unreleased

### Added

- **`CAST(x AS NUMERIC)` and the NUMERIC default affinity.** The CAST type-name
  resolver now follows SQLite's full type-name → affinity rules: a name matching
  none of INTEGER / TEXT / REAL / BLOB resolves to the new `CastType::Numeric`
  (so `NUMERIC`, `DECIMAL`, `BOOLEAN`, `DATE`, … all take numeric affinity)
  instead of erroring with "CAST to unsupported type". `BLOB` casts are rejected
  explicitly (still unimplemented) rather than mis-routed into the NUMERIC
  fallback. See sql-vm 0.4.18 for the runtime conversion.

## [0.2.16] - Unreleased

### Added

- **Column-defined `COLLATE` flows into `ORDER BY`.** `apply_col_constraint`
  now parses a column's `COLLATE NAME` (validating NOCASE/RTRIM/BINARY; an
  unknown sequence errors with "no such collating sequence", matching SQLite's
  prepare-time rejection) and stores it on `ColumnDef`. A bare `ORDER BY col`
  (single-table query, no JOINs) inherits that sequence — `CREATE TABLE
  t(x TEXT COLLATE NOCASE); SELECT x FROM t ORDER BY x` now folds case like
  real SQLite. An explicit `COLLATE` on the key (including `COLLATE BINARY`)
  still overrides the column default. Qualified (`t.x`) and aliased (`u.x`)
  references resolve too. Comparison / GROUP BY / DISTINCT collation
  propagation and multi-table resolution remain follow-ups.

## [0.2.15] - Unreleased

### Added

- **Scalar subqueries rejected with a clear error (not yet evaluated).** A
  `select_stmt` node inside a `primary` (a `( SELECT … )`) returns
  `PlanError::UnsupportedStatement("scalar subqueries are not yet supported")`
  rather than mis-planning it. Wiring `SqlExpr::ScalarSubquery` + the VM sub-plan
  evaluation is the follow-up.

## [0.2.14] - Unreleased

### Added

- **`IS [NOT] DISTINCT FROM` lowered onto `plan_is_distinct`.** The IS handler now
  inverts the sense when `DISTINCT` is present: `IS NOT DISTINCT FROM` is the
  null-safe *equality* (`x IS y`) and `IS DISTINCT FROM` its negation (`x IS NOT
  y`). No new codegen/VM — reuses the null-safe CASE machinery from the IS
  slice.

## [0.2.13] - Unreleased

### Added

- **Expr-level COLLATE lowered onto the `__collate` builtin.** A comparison with
  `COLLATE NOCASE`/`RTRIM` wraps BOTH operands in `__collate(operand, 'NAME')`,
  because those collations are *canonicalising* — `x <op> y COLLATE C` equals
  `canon_C(x) <op> canon_C(y)` under byte comparison, including non-text and NULL
  operands (`5 = '5' COLLATE NOCASE` stays 0). No new comparison opcode — mirrors
  the `GLOB → glob()` lowering. `COLLATE BINARY` is a no-op; unknown names error
  (`no such collating sequence`). New helpers `collate_name_after`/`wrap_collate`.

## [0.2.12] - Unreleased

### Added

- **Bitwise `BinaryOp`/`UnaryOp` variants.** `BitAnd`/`BitOr`/`ShiftLeft`/
  `ShiftRight` and `BitNot`; `plan_bitwise` maps the `bitwise` rule's operators
  and `plan_unary` maps `~`. Operands are coerced to integer in the VM.

## [0.2.11] - Unreleased

### Added

- **Simple `CASE` desugars to searched `CASE`.** `plan_case` now captures the
  optional operand (the expression node before the first `WHEN`) and lowers each
  `WHEN value THEN result` into a `(operand = value, result)` branch of the
  existing `SqlExpr::Case` — **no new codegen or VM opcode**. A NULL operand
  makes every `operand = value` NULL (never true), so it falls through to ELSE,
  matching SQLite. Exact because mini-sqlite expressions are pure (SQLite
  evaluates the operand once; a pure operand is unchanged by re-evaluation).

## [0.2.10] - Unreleased

### Added

- **`COLLATE name` read into `SortKey.collation`.** `plan_order_item` now scans
  for a `COLLATE` token and validates the following name against the three
  built-in sequences: `BINARY` (folded to `None`, the default byte order),
  `NOCASE`, and `RTRIM`. An unknown collation is a planning error
  (`no such collating sequence: X`), matching SQLite. New `collation:
  Option<String>` field on `SortKey` (stored uppercased).

## [0.2.9] - Unreleased

### Added

- **`a IS b` / `a IS NOT b` null-safe (in)equality.** The IS handler now
  distinguishes `IS [NOT] NULL` (no right operand) from `IS [NOT] <expr>` (a
  right operand node) and lowers the latter via new `plan_is_distinct` onto
  `CASE WHEN a IS NULL AND b IS NULL THEN 1 WHEN a IS NULL OR b IS NULL THEN 0
  ELSE a = b END` (negated → wrapped in `NOT`). Reuses the CASE node added in
  0.2.8 plus existing `IsNull`/`BinaryOp`/`UnaryOp` — **no codegen or VM opcode
  needed**. (The naive `(a=b) OR (a IS NULL AND b IS NULL)` is wrong — it yields
  NULL, not 0, for `1 IS NULL`.)

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
