# Changelog

All notable changes to `sql-planner` (Haskell) will be documented here.

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
