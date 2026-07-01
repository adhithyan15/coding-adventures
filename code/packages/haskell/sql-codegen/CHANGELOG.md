# Changelog — coding-adventures-sql-codegen

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1.0] — 2026-07-01

### Added

- **`CallBuiltin String Int` instruction** — emitted by `compileExpr` for
  `FuncCall` nodes instead of the previous `LoadNull` stub.  The `String` is
  the (lowercased) function name; `Int` is the arity.  The VM pops `arity`
  arguments from the stack, applies the named built-in scalar function, and
  pushes the result.  Supports `length`, `upper`, `lower`, `substr`, `trim`,
  `ltrim`, `rtrim`, `replace`, `abs`, `concat`, `coalesce`, `ifnull`.

### Changed

- `compileExpr` for `P.FuncCall name args` now emits
  `concatMap compileExpr args ++ [CallBuiltin name (length args)]` instead of
  `concatMap compileExpr args ++ [LoadNull]`.

## [0.1.0.0] — 2026-06-30

### Added

- **`SqlCodegen` module** — bytecode code generator for the Mini-SQLite Level 1 pipeline.
- **`Instruction` ADT** — complete stack-machine instruction set:
  - Stack ops: `LoadConst`, `LoadNull`, `LoadColumn`, `LoadParam`, `LoadGroupKey`, `Pop`
  - Arithmetic/comparison: `BinaryOpInstr BinaryOp`
  - Unary: `UnaryOpInstr UnaryOp`
  - Predicates: `IsNull`, `IsNotNull`, `Between`, `Like`, `InList`
  - Cursor control: `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan`
  - Row assembly: `BeginRow`, `EmitColumn`, `EmitRow`
  - Aggregation: `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`, `AdvanceGroup`
  - Control flow: `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt`
  - DDL: `CreateTableInstr`, `DropTableInstr`
  - DML: `InsertRow`, `UpdateRows`, `DeleteRows`
  - Transactions: `BeginTransaction`, `CommitTransaction`, `RollbackTransaction`
  - Post-processing: `SortResult`, `DistinctResult`, `LimitResult`
- **`BinaryOp`**, **`UnaryOp`**, **`AggFn`** supporting operator types.
- **`Program`** newtype wrapping `[Instruction]`.
- **`compile :: OptimizedPlan -> Program`** — top-level entry point.
- **`compileExpr :: SqlExpr -> [Instruction]`** — expression compiler (exported for testing).
- **46+ hspec tests** covering all plan node types and expression forms.
- Knuth-style literate inline documentation throughout.
- `cabal.project` referencing `sql-planner` and `sql-optimizer` siblings.
- `BUILD` and `BUILD_windows` scripts (`cabal test all --test-show-details=direct`).
- `required_capabilities.json` (no capabilities needed — pure library).
