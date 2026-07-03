# Changelog

All notable changes to `CodingAdventures.SqlPlanner.FSharp` will be documented here.

## [0.1.0] - 2026-06-30

### Added
- `SqlValue` discriminated union (Null | Integer | Real | Text | Bool)
- `Expr` discriminated union with 14 variants: Literal, Column, BinaryOp, UnaryOp, FuncCall, IsNull, IsNotNull, Between, In, NotIn, Like, NotLike, Wildcard, AggExpr
- `LogicalPlan` discriminated union with 15 node types: Scan, Filter, Project, Join, Aggregate, Having, Sort, Limit, Distinct, Union, Insert, Update, Delete, CreateTable, DropTable
- `Statement` type with 6 variants: Select, Insert, Update, Delete, CreateTable, DropTable
- `PlanError` type with 6 variants: AmbiguousColumn, UnknownTable, UnknownColumn, InvalidAggregate, UnsupportedStatement, InternalError
- `ISchemaProvider` interface and `InMemorySchemaProvider` implementation
- `Planner.plan` — transforms a single `Statement` into `Result<LogicalPlan, PlanError>`
- `Planner.planAll` — plans a list of statements, failing fast on the first error
- 8-step bottom-up SELECT pipeline: Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
- Column resolution with scope tracking, alias support, and ambiguity detection
- 52 unit tests; 83% line coverage, 100% method coverage
