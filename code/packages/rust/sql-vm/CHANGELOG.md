# Changelog — coding-adventures-sql-vm

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.4.8] - Unreleased

### Added

- **`OCTET_LENGTH(x)`** — the number of *bytes*, in contrast to `LENGTH`'s count
  of characters: `OCTET_LENGTH('héllo')` = 6 where `LENGTH('héllo')` = 5. Text is
  measured as its UTF-8 bytes, a blob as its raw byte count, and an
  integer/boolean as its decimal-text bytes (`OCTET_LENGTH(123)` = 3); NULL →
  NULL. Floats are declined (their byte length depends on SQLite's subtle float
  text form — same convention as HEX/QUOTE). Arity is checked before indexing.
  (The `LENGTH` doc is corrected: it counts characters, not bytes.)

## [0.4.7] - Unreleased

### Fixed

- **`HEX(NULL)`** now returns the empty string `''` (a text value), matching
  SQLite, instead of NULL. SQLite casts HEX's argument to a blob first, so
  `NULL` becomes an empty blob and hexing it yields `''` (`typeof` is `text`).
  Surfaced while adding UNHEX (`HEX(UNHEX('abc'))` should be `''`, not NULL).

## [0.4.6] - Unreleased

### Added

- **`UNHEX(x)` / `UNHEX(x, ignore)`** — the inverse of `HEX`: decode a string of
  hexadecimal digit pairs into a blob (case-insensitive). `unhex('414243')` →
  `x'414243'`, `unhex('')` → empty blob; an odd number of digits or a non-hex
  character yields NULL. The optional second argument is a set of ignorable
  characters, which SQLite permits only at a byte boundary — never splitting a
  pair: `unhex('41.42', '.')` → `x'4142'` but `unhex('4-1-4-2', '-')` → NULL.
  Integer/boolean arguments coerce to their decimal text; NULL in either argument
  → NULL. Output is bounded by the input length (no unbounded allocation), and
  arity is checked before indexing.

## [0.4.5] - Unreleased

### Fixed

- **`ROUND(x, n)` with a negative digit count** now matches SQLite, which treats
  a negative `n` as zero rather than rounding to tens/hundreds:
  `ROUND(2.567, -1)` = `ROUND(2.567, 0)` = `3.0` (was `0.0`), and
  `ROUND(12.5, -1)` = `13.0`. Positive/zero digit counts, round-half-away-from-
  zero, and NULL propagation are unchanged.

## [0.4.4] - Unreleased

### Added

- **`CONCAT(x, …)`** — concatenate every argument's text. A NULL argument
  contributes the empty string (it does not nullify), so
  `CONCAT('a', NULL, 'c')` = `'ac'`; the result is always text; at least one
  argument is required.
- **`CONCAT_WS(sep, x, …)`** — join the value arguments with `sep`. Unlike
  CONCAT, a NULL value argument is *skipped* entirely (`CONCAT_WS('-','a',NULL,'c')`
  = `'a-c'`); a NULL separator makes the whole result NULL; at least two
  arguments are required.
- **`SUBSTRING`** — an accepted spelling of the existing `SUBSTR`.

  Integer/boolean arguments coerce to their decimal text; Float/Blob arguments
  are declined (their SQLite text form is subtle — same convention as HEX/QUOTE).

## [0.4.3] - Unreleased

### Added

- **Two-argument `TRIM(x, y)` / `LTRIM(x, y)` / `RTRIM(x, y)`** — the second
  argument is a *set of characters* stripped from both / the left / the right
  end, matching SQLite: `TRIM('xxhixx', 'x')` → `'hi'`,
  `TRIM('abcHIcba', 'abc')` → `'HI'`. Trimming operates on Unicode characters,
  not bytes (`TRIM('héllo', 'h')` → `'éllo'`); an empty set removes nothing; a
  NULL in either argument propagates to NULL; integer/boolean arguments coerce
  to their decimal text (`TRIM(12321, '1')` → `'232'`). The single-argument
  whitespace forms are unchanged. The three arms now share one `trim_builtin`
  helper. The helper validates arity before indexing (so `TRIM()` — which the
  grammar permits — is a clean error, not an out-of-bounds panic) and resolves
  the trim-set through a `HashSet` so the operation stays `O(N + M)` rather than
  `O(N·M)` in the subject / set lengths.

### Added

- **Scalar `MAX(a, b, …)` / `MIN(a, b, …)`** (two-or-more arguments): return the
  largest / smallest argument, or NULL if any argument is NULL, comparing with
  SQL value order. The single-argument aggregate forms are unchanged (compiled to
  `FinalizeAgg`); the planner (sql-planner 0.2.0) now routes only the multi-arg
  calls here. Fixes `SELECT MAX(3, 9, 5)` returning `3` instead of `9`.

## [0.4.1] - Unreleased

### Added

- **`IIF(x, y, z)`** — SQLite's function-form conditional, equivalent to
  `CASE WHEN x THEN y ELSE z END`: returns `y` when `x` is truthy (SQL
  three-valued logic — a NULL or falsy `x` selects `z`), reusing the engine's
  `is_truthy` helper. Unit-tested and validated against real SQLite by the
  mini-sqlite differential oracle. (A pure-VM partial for `CASE`, which is
  blocked on a stale generated grammar — see the mini-sqlite notes.)

## [0.4.0] - Unreleased

### Added

- **Five more scalar built-in functions** in `call_builtin`, matching SQLite:
  - `SIGN(x)` — `-1`/`0`/`+1` for a negative/zero/positive number; NULL for a
    NULL or non-numeric argument.
  - `UNICODE(s)` — the code point of the first character of `s`; NULL for a NULL
    or empty string.
  - `CHAR(x1, …)` — a string built from the argument code points (out-of-range
    or non-integer arguments contribute nothing; no args → `""`).
  - `ZEROBLOB(n)` — a BLOB of `n` zero bytes (`n < 0` → empty); NULL → NULL. The
    length is capped at 1,000,000 (returning `ResourceLimit`, like the GROUP BY /
    COUNT(DISTINCT) guards) so a query such as `zeroblob(9999999999)` can't force
    a multi-gigabyte eager allocation.
  - `QUOTE(x)` — the value as an SQL literal (NULL → `NULL`, text single-quoted
    with doubled inner quotes, blob as `X'…'` hex, integer as its digits; floats
    declined, like `HEX`).
  Each is unit-tested and validated end-to-end against real SQLite by the
  mini-sqlite differential oracle.

## [0.3.0] - Unreleased

### Added

- **Five scalar built-in functions** in `call_builtin`, matching SQLite:
  - `IFNULL(a, b)` — the two-argument `COALESCE`.
  - `NULLIF(a, b)` — NULL when the arguments are equal, else `a`.
  - `TYPEOF(x)` — the storage-class name (`null`/`integer`/`real`/`text`/`blob`).
  - `INSTR(haystack, needle)` — 1-based **character** index of the first match
    (0 if absent, NULL on a NULL argument, 1 for an empty needle).
  - `HEX(x)` — uppercase hex of the argument's bytes (text → UTF-8, blob → raw,
    integer → decimal-text bytes; NULL → NULL; floats are declined).
  These parsed as function calls already but hit the `unknown built-in function`
  fallthrough; each is now implemented and unit-tested, and validated end-to-end
  against real SQLite by the mini-sqlite differential oracle.

## [0.2.1] - Unreleased

### Fixed

- **Same-named output columns no longer collide.** Phase-4 materialization used
  to collapse each row's positional `(name, value)` pairs into a
  `HashMap<String, SqlValue>` and then re-read one value per output-column name.
  Two output columns that share a name — e.g. `SELECT UPPER(x), LENGTH(x)` (both
  default to `?`) or `SELECT id, id` — collided in the map, so every such column
  returned the *last* value (`SELECT UPPER(x), LENGTH(x)` came back as
  `LENGTH(x), LENGTH(x)`). The row buffer is already positional and parallel to
  the locked `output_columns` (both are produced by the same `EmitColumn`
  sequence, and hidden sort-key columns are truncated off both together), so we
  now project by **position** and drop the name-keyed map entirely. Fixes the
  differential-oracle `string_functions` divergence.

### Added

- **Outer-join match flag** — three instructions with no value-stack effect that
  let `sql-codegen` implement `LEFT`/`RIGHT OUTER JOIN`: `ClearMatch` (reset at
  the start of each outer row), `SetMatch` (an inner row satisfied `ON`), and
  `JumpIfMatched` (skip the NULL-padded emit when the outer row matched). The VM
  keeps a single `join_matched` boolean; a false condition still advances the
  loop, so termination and stack balance are unchanged.

## [0.1.0] — 2026-07-01

### Added

- Initial implementation of the Mini-SQLite Level 1 stack-machine VM.
- `execute(program, backend)` public entry point returns `QueryResult`.
- `QueryResult` struct with `columns`, `rows`, and `rows_affected` fields.
- `VmError` enum covering StackUnderflow, CursorNotFound, LabelNotFound,
  TypeMismatch, DivisionByZero, AggIndexOutOfRange, BackendError.
- Full instruction set support:
  - Stack: `LoadConst`, `LoadColumn`
  - Arithmetic: `BinaryOpInstr` (Add, Sub, Mul, Div, Mod)
  - Comparison: `BinaryOpInstr` (Eq, Neq, Lt, Lte, Gt, Gte)
  - Logic: `BinaryOpInstr` (And, Or) with SQL three-valued / Kleene logic
  - String: `BinaryOpInstr` (Concat)
  - Unary: `UnaryOpInstr` (Neg, Not) with NULL propagation
  - NULL tests: `IsNull`, `IsNotNull`
  - Pattern: `Like` (iterative NFA, no regex, ReDoS-safe)
  - Range: `Between` (inclusive and exclusive bounds)
  - Membership: `InList`
  - Scan: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row assembly: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey` (no-op)
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - DML: `InsertRow` (functional), `UpdateRows` (Level 1 stub), `DeleteRows` (Level 1 stub)
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-ops: `SortResult`, `DistinctResult`, `LimitResult`
- Eager cursor buffering: all rows fetched at `OpenScan` time.
- Exhaustion-flag cursor model: `AdvanceCursor` sets `exhausted` flag;
  `JumpIfExhausted` reads the flag so the last row is consumed before jumping.
- Post-op pass: a second instruction-scan after `Halt` collects SortResult /
  DistinctResult / LimitResult so they apply to the final result buffer.
- Literate programming style: all functions include inline explanations,
  truth tables, diagrams, and examples.
- 75 unit tests covering all instruction groups, edge cases, and error paths.
- BUILD (bash) and BUILD_windows (PowerShell) scripts.

### Known limitations (Level 1)

- `UpdateRows` counts rows affected but does not persistently update the
  backend.  The `Backend::update()` API requires a `Cursor` keyed to the
  table, which can only be constructed via `InMemoryBackend::open_cursor`
  (a non-trait method).  Level 2 will close this gap.
- `DeleteRows` removes rows from the local cursor buffer (preventing re-visits)
  but does not call `Backend::delete()` for the same reason.
- GROUP BY aggregation uses a single global accumulator; per-group aggregation
  requires a hash-map grouping strategy (Level 2).
