# Changelog

All notable changes to `CodingAdventures.SqlPlanner.CSharp` will be documented here.

## [0.1.0] - 2026-06-30

### Added
- `SqlExpr` abstract record hierarchy with 14 variants: Literal, Column, BinaryOp, UnaryOp, FuncCall, IsNull, IsNotNull, Between, In, NotIn, Like, NotLike, Wildcard, AggExpr
- `LogicalPlan` abstract record hierarchy with 15 node types: ScanPlan, FilterPlan, ProjectPlan, JoinPlan, AggregatePlan, HavingPlan, SortPlan, LimitPlan, DistinctPlan, UnionPlan, InsertPlan, UpdatePlan, DeletePlan, CreateTablePlan, DropTablePlan
- `Statement` abstract record hierarchy with 6 variants: SelectStatement, InsertStatement, UpdateStatement, DeleteStatement, CreateTableStatement, DropTableStatement
- `PlanException` base class with 5 subtypes: AmbiguousColumnException, UnknownTableException, UnknownColumnException, InvalidAggregateException, UnsupportedStatementException
- `ISchemaProvider` interface and `InMemorySchemaProvider` implementation
- `SqlPlanner.Plan(Statement)` — transforms a single statement, throws on error
- `SqlPlanner.PlanAll(IEnumerable<Statement>)` — plans a list, failing on first error
- 8-step bottom-up SELECT pipeline: Scan → Filter → Aggregate → Having → Project → Distinct → Sort → Limit
- Column resolution with scope tracking, alias support, and ambiguity detection
- 40 unit tests; 89% line coverage, 91% method coverage
