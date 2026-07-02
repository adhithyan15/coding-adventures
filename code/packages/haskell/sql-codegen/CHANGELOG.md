# Changelog — coding-adventures-sql-codegen

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.2.0] — 2026-07-01

### Fixed

- **`compileScanLoop` cursor alias mismatch (runtime bug)**: the planner's
  `resolveColumn` assigns every column the alias `seAlias`, which is the
  bare table name when no explicit AS alias is given (e.g. `Column (Just
  "users") "id"` for `SELECT id FROM users`).  `LoadColumn (Just "users")
  "id"` therefore looks for the cursor row under key `"users"` in
  `vmCurrentRow`.  Previously `compileScanLoop` emitted `OpenScan table
  Nothing`, which stored the cursor row under key `""` (from
  `cursorKey Nothing`).  The mismatch caused every `LoadColumn (Just tbl)
  col` to return `SqlNull` instead of the real value.  Fixed by normalising
  the `Nothing` alias to `Just table` inside `compileScanLoop`, so the
  cursor key always matches the table name the planner used.

- **`SELECT *` wildcard expands at runtime (correctness fix)**: `OutputStar`
  was compiled to `LoadConst (LitText "*")`, which pushed a marker onto the
  stack but emitted no columns — every `SELECT * FROM t` returned rows with
  zero columns.  Fixed in the VM: when `EmitRow` fires with no `EmitColumn`
  calls having happened (`vmColOrder` is empty) and the stack top is
  `SqlText "*"`, the wildcard marker is popped and all columns from the
  current cursor row are copied into the output buffer.

- **Output column order preserved (correctness fix)**: `buildResult` derived
  the column name list from `Map.keys` of the first output row, which is
  alphabetical (Haskell's `Data.Map.Strict`).  Queries like `SELECT id,
  name, age` therefore returned columns in alphabetical order (`age, id,
  name`), failing tests that expected the original SELECT-list order.  Fixed
  by adding `vmColOrder :: [String]` (reset by `BeginRow`, appended by
  `EmitColumn`) and `vmOutputColumns :: [String]` (captured on first
  `EmitRow`) to `VmState`.  `buildResult` now reads `vmOutputColumns`
  instead of calling `Map.keys`.

- **Aggregate output column names (`AS` alias honoured)**: aggregate queries
  emitted `EmitColumn "_agg0"` etc. (synthetic names from `collectAggs`)
  instead of the user-supplied `AS` alias (e.g. `SELECT COUNT(*) AS n`).
  Fixed by passing the outer `Project`'s column list to
  `compileAggregateQuery`; when a project column has a user alias, that alias
  is used for the corresponding `EmitColumn`.

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
