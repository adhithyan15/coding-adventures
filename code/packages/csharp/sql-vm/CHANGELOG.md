# Changelog

All notable changes to `CodingAdventures.SqlVm` are documented here.

## [0.1.0] — 2026-06-30

### Added

- Initial implementation of the C# SQL stack-machine VM.
- `SqlVm.Execute(Program, Backend) → QueryResult` — the single public entry point.
- `QueryResult` record with `Columns`, `Rows`, and `RowsAffected` fields.
- Complete dispatch for all Level-1 instruction types from `SqlCodegen`:
  - Stack/constants: `LoadConst`, `LoadColumn`, `Pop`
  - Binary operators: Add, Sub, Mul, Div, Mod, Eq, Neq, Lt, Lte, Gt, Gte, And, Or, Concat
  - Unary operators: Neg, Not (with three-valued logic for NOT NULL)
  - Predicates: `IsNullInstr`, `IsNotNullInstr`, `BetweenInstr`, `InListInstr`, `LikeInstr`
  - Scalar functions: ABS, UPPER, LOWER, LENGTH, TRIM, LTRIM, RTRIM, COALESCE, IFNULL, NULLIF, TYPEOF, ROUND, SUBSTR/SUBSTRING, REPLACE, HEX, MAX, MIN
  - Cursors: `OpenScan`, `AdvanceCursor`, `CloseScan`
  - Row assembly: `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema`
  - Aggregates: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`, `LoadGroupKey`, `AdvanceGroupKey`
  - Post-ops: `SortResult`, `LimitResult`, `DistinctResult`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - LEFT JOIN: `JoinBeginRow`, `JoinSetMatched`, `JoinIfMatched`
  - Control flow: `CodegenLabel`, `Jump`, `JumpIfFalse`, `JumpIfTrue`, `Halt`
- Three-valued logic (SQL NULL semantics) throughout all operators and predicates.
- `GroupKey` structural equality type for GROUP BY accumulation with correct NULL grouping.
- `LikeMatch(value, pattern)` utility method using regex conversion for SQL LIKE semantics.
- `RowIteratorCursor` fallback adapter for backends that expose only `Scan()`.
- 100+ integration tests covering all instruction categories, NULL handling, aggregates, post-ops, DML, DDL, LEFT JOIN, scalar functions, LIKE matching, and error paths.
- `BUILD` / `BUILD_windows` with ≥80% line coverage gate via coverlet.
- `required_capabilities.json` declaring .NET 9 requirement.

### Security

- `LikeMatch`: use `RegexOptions.NonBacktracking` (.NET 7+) to eliminate ReDoS risk from
  crafted LIKE patterns such as `%a%a%a%a%` matched against long strings (MEDIUM).
- `ApplyLimit`: clamp `long` offset/count to `int.MaxValue` before casting to avoid integer
  overflow when extreme LIMIT/OFFSET values are supplied (LOW).
- `EvalSubstr`: clamp `long` start/length arguments to `[int.MinValue, int.MaxValue]` before
  casting to avoid integer overflow in SUBSTR with extreme arguments (LOW).
