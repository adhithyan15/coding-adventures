# sql-planner (Python)

Translates a structured SQL AST into a **Logical Plan Tree** — a tree of
relational-algebra nodes that captures the intent of a query independent of
any particular execution strategy or storage backend.

## Where it fits

```
sql string
    │  sql-lexer + sql-parser
    ▼
parse tree (grammar ASTNode)
    │  parse_tree_adapter (separate concern — not in this package)
    ▼
structured AST  ──────┐
                      │  sql-planner.plan()  ← this package
                      ▼
                LogicalPlan
                      │  sql-optimizer
                      ▼
                OptimizedPlan
                      │  sql-codegen → sql-vm
                      ▼
                  QueryResult
```

## Two interfaces

The planner has **two explicit interface layers**:

1. **Input interface:** `sql_planner.ast` — a typed, structured Statement
   hierarchy. Tests construct these directly; a separate adapter (future
   work) converts the raw parse tree from `sql-parser` into them.
2. **Output interface:** `sql_planner.plan` — a LogicalPlan tree consumed
   by the optimizer and codegen.

The planner core (`sql_planner.planner`) implements the translation
between them.

## Usage

```python
from sql_planner import (
    plan,
    SelectStmt, TableRef, SelectItem,
    Column, BinaryExpr, BinaryOp, Literal,
    InMemorySchemaProvider,
)

# Schema the planner will consult for column resolution.
schema = InMemorySchemaProvider({"users": ["id", "name", "age"]})

# Build a structured AST directly — no parsing involved.
ast = SelectStmt(
    from_=TableRef(table="users"),
    items=(SelectItem(expr=Column(table=None, col="name")),),
    where=BinaryExpr(
        op=BinaryOp.GT,
        left=Column(table=None, col="age"),
        right=Literal(value=18),
    ),
)

# Plan it.
tree = plan(ast, schema)

# The result:
# Project(
#   items=(ProjectionItem(expr=Column(table="users", col="name"), alias="name"),),
#   input=Filter(
#     predicate=BinaryExpr(op=GT, left=Column(table="users", col="age"), right=Literal(18)),
#     input=Scan(table="users", alias=None),
#   ),
# )
```

## What it plans today

- `SELECT` with all clauses: `FROM`, `JOIN` (INNER / LEFT / RIGHT / FULL /
  CROSS), `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT / OFFSET`,
  `DISTINCT`, aggregate functions (`COUNT`, `SUM`, `AVG`, `MIN`, `MAX`,
  including `COUNT(DISTINCT)` and `COUNT(*)`).
- `INSERT INTO t (cols) VALUES (...)`.
- `UPDATE t SET col = expr WHERE ...`.
- `DELETE FROM t WHERE ...`.
- `CREATE TABLE [IF NOT EXISTS] t (...)` and `DROP TABLE [IF EXISTS] t`.
- Column resolution with alias-aware ambiguity detection.

## Not yet supported

- `UNION` / `UNION ALL`.
- Subqueries (in `FROM`, in `WHERE`, in `SELECT`).
- `INSERT INTO t SELECT ...`.
- Window functions.
- `WITH` / CTEs.

These raise `UnsupportedStatement` with the statement kind in the error
message.

## Errors

```
PlanError
├── AmbiguousColumn      — bare column matches more than one in-scope table
├── UnknownTable         — FROM references an unknown table
├── UnknownColumn        — column doesn't exist in any in-scope table
├── InvalidAggregate     — aggregate in WHERE clause, etc.
├── UnsupportedStatement — planner doesn't handle this shape yet
└── InternalError        — escape hatch
```

## Development

```
uv venv --clear
uv pip install -e ../sql-backend -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```

## Specification

See [`code/specs/sql-planner.md`](../../../specs/sql-planner.md) for the
full node taxonomy and planning rules.
