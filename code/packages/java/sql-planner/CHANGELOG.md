# Changelog

All notable changes to `coding-adventures-sql-planner` (Java) will be documented here.

## [0.1.0] - 2026-06-30

### Added
- `SqlExpr` sealed interface with 14 record variants: Literal, Column, BinaryOp, UnaryOp, FuncCall, IsNull, IsNotNull, Between, In, NotIn, Like, NotLike, Wildcard, AggExpr
- `LogicalPlan` sealed interface with 15 record variants: Scan, Filter, Project, Join, Aggregate, Having, Sort, Limit, Distinct, Union, Insert, Update, Delete, CreateTable, DropTable
- `Statement` sealed interface with 6 record variants: Select, Insert, Update, Delete, CreateTable, DropTable
- `PlanException` base class with 5 subtypes: AmbiguousColumnException, UnknownTableException, UnknownColumnException, InvalidAggregateException, UnsupportedStatementException
- `SchemaProvider` interface and `InMemorySchemaProvider` implementation
- `SqlPlanner.plan(Statement)` — transforms a single statement, throws on error
- `SqlPlanner.planAll(List<Statement>)` — plans a list, failing on first error
- 8-step bottom-up SELECT pipeline: Scan → Filter → Aggregate → Having → Distinct → Sort → Limit → Project
- Column resolution with scope tracking, alias support, and ambiguity detection
- 50 unit tests; ≥80% line coverage (JaCoCo)
