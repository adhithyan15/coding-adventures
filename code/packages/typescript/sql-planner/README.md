# sql-planner (TypeScript)

SQL query planner: converts a parsed SQL AST (from
`coding-adventures-sql-parser`) into a typed `LogicalPlan` tree.

## Where It Fits

```
sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
                 ↑
            this package
```

## Usage

```typescript
import { plan, PlanError } from "@coding-adventures/sql-planner";
import type { LogicalPlan } from "@coding-adventures/sql-planner";
import { parseSQL } from "coding-adventures-sql-parser";

const ast = parseSQL("SELECT name FROM users WHERE id = 1");
const logical: LogicalPlan = plan(ast);
```

## Architecture

The planner performs a single tree-walk over the parsed AST and builds a
typed `LogicalPlan` tree. This tree separates *what to compute* from *how*
to compute it — the optimizer can rewrite it, and the code generator
translates it to bytecode without revisiting the original SQL text.

### LogicalPlan node types

| Node | Description |
|------|-------------|
| `ScanNode` | Full table scan |
| `FilterNode` | WHERE predicate |
| `ProjectNode` | SELECT column list |
| `SortNode` | ORDER BY with ascending/nullsLast flags |
| `LimitNode` | LIMIT / OFFSET |
| `DistinctNode` | SELECT DISTINCT |
| `AggregateNode` | GROUP BY + aggregate functions |
| `HavingNode` | HAVING predicate |
| `JoinNode` | INNER/LEFT/RIGHT/FULL JOIN |
| `InsertNode` | INSERT INTO |
| `UpdateNode` | UPDATE SET |
| `DeleteNode` | DELETE FROM |
| `CreateTableNode` | CREATE TABLE |
| `DropTableNode` | DROP TABLE |
| `TransactionNode` | BEGIN/COMMIT/ROLLBACK |

### NULL ordering convention

`SortNode.nullsLast` is set by the planner to `ascending` — ASC sorts
produce nulls last; DESC sorts produce nulls first. This matches SQLite's
default behavior.
