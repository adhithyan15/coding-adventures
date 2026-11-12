# Changelog

All notable changes to `sql-planner` (Haskell) will be documented here.

## [Unreleased]

### Changed

- `BUILD_windows` now runs `cabal test all` unconditionally. It used to run Cabal only if
  `%ProgramFiles%\cabal\bin\cabal.exe` existed, which it never does on the CI runner, so the
  tests were silently skipped on Windows.

- `BUILD_windows` runs under `cmd.exe`: the POSIX `sh` form (`command -v`, `/dev/null`) failed there, so it now uses the repo's `if exist "%ProgramFiles%\cabal\bin\cabal.exe" (...)` idiom.

### Fixed

- `BUILD_windows` runs under `cmd.exe`: the POSIX `sh` form (`command -v`, `/dev/null`) failed there, so it now uses the repo's `if exist "%ProgramFiles%\cabal\bin\cabal.exe" (...)` idiom.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## [0.1.0] - 2026-06-30

### Added
- `SqlExpr` algebraic data type with 14 variants: Literal, Column, BinaryOp, UnaryOp, FuncCall, IsNull, IsNotNull, Between, InExpr, NotInExpr, Like, NotLike, Wildcard, AggExpr
- `LogicalPlan` algebraic data type with 15 variants: Scan, Filter, Project, JoinPlan, Aggregate, Having, Sort, Limit, Distinct, Union, InsertPlan, UpdatePlan, DeletePlan, CreateTablePlan, DropTablePlan
- `Statement` algebraic data type with 6 variants: SelectStmt, InsertStmt, UpdateStmt, DeleteStmt, CreateTableStmt, DropTableStmt
- `PlanError` sum type with 5 variants: AmbiguousColumn, UnknownTable, UnknownColumn, InvalidAggregate, UnsupportedStatement
- `SchemaProvider` newtype + `inMemorySchema` association-list constructor
- `plan :: SchemaProvider -> Statement -> Either PlanError LogicalPlan`
- `planAll :: SchemaProvider -> [Statement] -> Either PlanError [LogicalPlan]`
- 8-step bottom-up SELECT pipeline: Scan → Filter → Aggregate → Having → Distinct → Sort → Limit → Project
- Column resolution with scope tracking, alias support, and ambiguity detection
- 47 hspec tests: C1–C13 conformance, structural, error-path, and expression-type coverage
