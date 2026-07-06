# sql-vm (TypeScript)

SQL virtual machine: executes a compiled IR `Program` (from
`@coding-adventures/sql-codegen`) against an in-memory `Database` and returns
a `QueryResult`.

## Where It Fits

```
sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
                                                               ↑
                                                          this package
```

## Usage

```typescript
import { execute, VmError } from "@coding-adventures/sql-vm";
import type { Database, QueryResult } from "@coding-adventures/sql-vm";

const db: Database = new Map();

try {
  const result: QueryResult = execute(program, db);
  console.log(result.columns);      // ["id", "name"]
  console.log(result.rows);         // [[1, "Alice"], [2, "Bob"]]
  console.log(result.rowsAffected); // -1 for SELECT; row count for DML
} catch (err) {
  if (err instanceof VmError) {
    console.error("VM error:", err.message);
  }
}
```

## Architecture

The VM is a classic **dispatch loop**: a `while (ip < instructions.length)`
loop that switches on `instructions[ip].op`, executes the handler, and advances
`ip`. No recursion. Every opcode is a small, self-contained block.

### State spaces

| Name | Type | Purpose |
|------|------|---------|
| `stack` | `SqlValue[]` | Operand stack — expressions push/pop values here |
| `cursors` | `Map<number, {rows, pos}>` | Open table iterators keyed by cursor ID |
| `rowBuffer` | `Record<string, SqlValue>` | Accumulates the current output row |
| `aggBuffer` | `Record<string, SqlValue>` | Holds finalized aggregate results |
| `groupMap` | `Map<string, GroupEntry>` | Per-group aggregate accumulators |
| `resultColumns` | `string[]` | Final output column names |
| `resultRows` | `SqlValue[][]` | Final output rows |

### Database interface

The `Database` is a `Map<string, TableData>` where `TableData` holds `columns`
and `rows` (as `Record<string, SqlValue>[]`). DML instructions mutate the map
directly; there is no transaction buffer (transactions are no-ops in this
in-memory engine).

### NULL semantics

- All arithmetic and comparisons propagate NULL (return `null`) when either
  operand is `null`, except AND/OR which follow SQL three-valued logic.
- `AND`: `false AND null → false`, `true AND null → null`
- `OR`: `true OR null → true`, `false OR null → null`

### Cursor positioning

`OpenScan` sets `pos = 0`. `JumpIfExhausted` checks `pos >= rows.length`.
`AdvanceCursor` increments `pos`. Row reads always use `rows[pos]`.

### Special tables

- `__dual__` — a virtual single-row, zero-column table used for FROM-less
  SELECT (e.g. `SELECT 1 + 1`).

### `rowsAffected` semantics

- `-1`: result of a SELECT or DDL statement
- `≥0`: number of rows inserted/updated/deleted by a DML statement

## Test coverage

- 121 tests, 97.6% statement / 100% function / 99.5% line coverage.
