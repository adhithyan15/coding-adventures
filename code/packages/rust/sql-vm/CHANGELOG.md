# Changelog — coding-adventures-sql-vm

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] - Unreleased

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
