# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-07-01

### Level 1 graduation — full five-stage pipeline

- Wired SQL through the complete pipeline: parse → plan → optimize → codegen → vm.
- Added `executeGroupByInMemory` for GROUP BY and aggregate queries; routes all
  aggregate SELECTs in-memory to bypass the codegen limitation that produces a
  single accumulated row regardless of GROUP BY keys.
- Fixed `peelWrappers` in `SqlCodegen.fs`: postOps were accumulating in wrong
  order (LimitResult before SortResult); changed to prepend so SortResult always
  precedes LimitResult.
- Fixed `compileHavingExpr` in `SqlCodegen.fs`: HAVING predicates containing
  `AggExpr` nodes were compiled as `LoadConst Null`; now maps each `AggExpr` to
  its `FinalizeAgg` slot.
- Extended `FuncEval.evalExpr` in `MiniSqlite.fs` with full SQL expression
  evaluation: arithmetic (Sub, Mul, Div, Mod), comparisons (Eq, NotEq, Lt, Lte,
  Gt, Gte with NULL propagation), logical (And, Or, IsNull, IsNotNull), and
  predicates (Between, In, NotIn, Like, NotLike).
- Fixed `__farg_X__` hidden columns being included in `userAliases` (causing
  `List.mapi2` length mismatch for queries with scalar function calls).
- Fixed NULL sort order in both `SqlVm.fs` and `MiniSqlite.fs`: NullOrder now
  applies independently of sort direction — NULLs sort first/last per the
  NullOrder flag; only non-null comparisons are direction-flipped.
- All 33 conformance tests (24 fixture + 9 unit) pass.

## [0.1.0] - 2026-05-02

- Added a Level 0 in-memory mini-sqlite facade for F#.
