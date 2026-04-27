# SQL Code Generator Specification

## Overview

This document specifies the `sql-codegen` package: a **code generator** that
translates an optimized `LogicalPlan` tree (produced by `sql-optimizer`) into a flat
sequence of **IR instructions** — a bytecode program that the `sql-vm` can execute.

The code generator sits between the optimizer and the VM:

```
OptimizedPlan (from sql-optimizer)
    │  sql-codegen.compile()
    ▼
Program { instructions: Vec<Instruction>, labels: Map<String, usize> }
    │  sql-vm  (next stage)
    ▼
QueryResult
```

**Why compile to bytecode?**

The LogicalPlan tree is great for reasoning and optimization but awkward for direct
execution: tree traversal requires recursion, and recursive interpreters are hard to
debug, hard to profile, and harder to port identically across 17 languages.

A **flat bytecode program** solves this: the VM is a simple loop — read the next
instruction, execute it, advance the program counter. No recursion. Each instruction
is small and self-contained. The program can be printed, inspected, diffed, and
serialized.

This is the same approach taken by the Python bytecode compiler, the JVM, WebAssembly,
and SQLite's VDBE (Virtual DataBase Engine). The insight is the same in all cases:
compile the high-level representation once into something the loop can eat.

---

## Where It Fits

```
Depends on: sql-planner (for LogicalPlan types), sql-optimizer (for OptimizedPlan)
Used by:    sql-vm (executes the Program)
```

The code generator is **purely functional** — it produces a `Program` from a plan with
no side effects.

---

## Supported Languages

All 17 repository languages implement this package.

Entry points:

**Rust**
```rust
pub fn compile(plan: &LogicalPlan) -> Result<Program, CodegenError>
```

**TypeScript**
```typescript
export function compile(plan: LogicalPlan): Program  // throws CodegenError
```

**Go**
```go
func Compile(plan *LogicalPlan) (*Program, error)
```

**Python**
```python
def compile(plan: LogicalPlan) -> Program: ...  # raises CodegenError
```

**Ruby / Elixir / others:** analogous patterns per language idiom.

---

## The IR Instruction Set

The IR is a **register-free stack machine**: operands are pushed onto a value stack;
operations pop their arguments and push their results. There is no fixed set of named
registers — all intermediate values live on the stack.

Think of it like a calculator with RPN (Reverse Polish Notation): to compute `3 + 4`,
you push `3`, push `4`, then invoke `Add`. The result `7` remains on the stack.

The VM also maintains:
- A **cursor table**: open iterators over table data, each addressed by a cursor ID
- A **row buffer**: the output row currently being assembled
- A **result buffer**: all completed output rows
- An **aggregate state table**: per-slot running aggregate values

### Instruction Types

#### Stack Instructions

**`LoadConst(value: SqlValue)`**
Push a literal value onto the stack. `SqlValue = Null | Int(i64) | Float(f64) | Text(String) | Bool(bool)`.

**`LoadColumn(cursor_id: u32, column: String)`**
Read a column value from the current row of the named cursor and push it onto the stack.
Pushes `Null` if the cursor's current row does not contain the column.

**`Pop`**
Discard the top value from the stack. Used to clean up unused expression results.

---

#### Arithmetic and Logic Instructions

**`BinaryOp(op: BinaryOp)`**
Pop two values (right operand first, then left operand), apply the operator, push the result.

```
BinaryOp = Add | Sub | Mul | Div | Mod
         | Eq | NotEq | Lt | Lte | Gt | Gte
         | And | Or
         | Concat    -- string concatenation
```

Follows three-valued logic for boolean operations (NULL propagates through arithmetic
and comparisons; AND and OR follow SQL's truth tables):

```
NULL AND TRUE  = NULL    NULL OR TRUE  = TRUE
NULL AND FALSE = FALSE   NULL OR FALSE = NULL
NULL AND NULL  = NULL    NULL OR NULL  = NULL
```

**`UnaryOp(op: UnaryOp)`**
Pop one value, apply the operator, push the result.

```
UnaryOp = Neg    -- arithmetic negation: -(5) = -5, -(NULL) = NULL
        | Not    -- boolean negation: NOT TRUE = FALSE, NOT NULL = NULL
```

**`IsNull`**
Pop one value. Push `TRUE` if it is `Null`, `FALSE` otherwise.

**`IsNotNull`**
Pop one value. Push `FALSE` if it is `Null`, `TRUE` otherwise.

**`Between`**
Pop three values in order: `high`, `low`, `value` (high pushed first). Push `TRUE`
if `low <= value <= value <= high`, using three-valued logic. Equivalent to
`value >= low AND value <= high`.

**`InList(n: usize)`**
Pop `n` values (the list) then one more value (the needle). Push `TRUE` if any list
element equals the needle; `FALSE` if none do and no `NULL` was in the list;
`NULL` if no match was found but a `NULL` was in the list. Implements SQL `IN (...)`.

**`Like`**
Pop two strings: `pattern` first, then `value`. Push `TRUE` if value matches the
SQL LIKE pattern (`%` = any sequence, `_` = any single character). Push `NULL`
if either operand is `NULL`.

**`Coalesce(n: usize)`**
Pop `n` values and return the first non-NULL one. If all are NULL, return `NULL`.

---

#### Scan Instructions

**`OpenScan(cursor_id: u32, table: String)`**
Ask the backend to open an iterator over the named table. The iterator is stored in
the cursor table under `cursor_id`. The first call to `AdvanceCursor` will position
the cursor on the first row.

**`AdvanceCursor(cursor_id: u32, on_exhausted: Label)`**
Move the cursor at `cursor_id` to the next row. If the cursor is exhausted (no more
rows), jump to `on_exhausted`. Otherwise continue to the next instruction.
Equivalent to a `for` loop's loop condition.

**`CloseScan(cursor_id: u32)`**
Release the cursor. Backends may use this to free resources (file handles, locks).

---

#### Row Output Instructions

**`BeginRow`**
Clear the row buffer. Begin assembling a new output row.

**`EmitColumn(name: String)`**
Pop the top value from the stack. Store it in the row buffer under `name`.

**`EmitRow`**
Finalize the current row buffer and append it to the result buffer. The row buffer
is cleared.

**`SetResultSchema(columns: Vec<String>)`**
Set the output column names for the result. Must be emitted once before any
`EmitRow` instruction. The columns list defines the order of columns in the output.

---

#### Aggregate Instructions

Aggregates are computed in two phases: **accumulate** (one call per input row) and
**finalize** (one call per group, after all rows in the group are processed).

**`InitAgg(slot: u32, func: AggFunc)`**
Initialize aggregate slot `slot` to its zero state.
```
AggFunc zero states:
  COUNT → 0 (integer)
  SUM   → NULL (will become 0 when first non-NULL row arrives)
  AVG   → (sum=NULL, count=0)
  MIN   → NULL
  MAX   → NULL
```

**`UpdateAgg(slot: u32)`**
Pop the top value from the stack. Feed it into the aggregate at `slot`.
If the value is `NULL`, COUNT(col) ignores it; SUM/AVG/MIN/MAX ignore it.
`COUNT(*)` uses a separate `CountStarAgg` slot that always increments.

**`FinalizeAgg(slot: u32)`**
Compute the final aggregate value for `slot` and push it onto the stack.
For `AVG`, computes `sum / count`; returns `NULL` if count is 0.

**`SaveGroupKey(n: usize)`**
Pop `n` values and save them as the current group key. Used before emitting a
group's output row.

**`LoadGroupKey(i: usize)`**
Push the `i`-th value from the saved group key onto the stack.

---

#### Sort and Limit Instructions

**`SortResult(keys: Vec<SortKey>)`**
Sort the result buffer in place by the given keys. `SortKey { column: String, direction: Asc|Desc, nulls: NullsFirst|NullsLast }`.

Applied after all `EmitRow` instructions have run. The result buffer is sorted once
at the end of the program.

**`LimitResult(count: Option<u64>, offset: Option<u64>)`**
Truncate the result buffer: skip the first `offset` rows (default 0), then keep
at most `count` rows (default unlimited).

Applied after `SortResult` (or directly if there is no sort).

**`DistinctResult`**
Remove duplicate rows from the result buffer. Two rows are duplicates if every column
value compares equal (NULLs compare equal for deduplication).

---

#### Mutation Instructions

**`InsertRow(table: String, columns: Vec<String>)`**
Pop a value for each column in `columns` (last column first) and ask the backend to
insert the row.

**`UpdateRows(table: String, assignments: Vec<String>)`**
For each row in the cursor currently open on `table` that the predicate accepted,
pop values for each assignment column and ask the backend to update the row.
The count of updated rows is pushed onto the stack.

**`DeleteRows(table: String)`**
Delete all rows that the current scan cursor has marked for deletion.
The count of deleted rows is pushed onto the stack.

**`CreateTable(table: String, columns: Vec<ColumnDef>, if_not_exists: bool)`**
Ask the backend to create a new table. If `if_not_exists` is true and the table
already exists, this is a no-op.

**`DropTable(table: String, if_exists: bool)`**
Ask the backend to drop the table. If `if_exists` is true and the table does not
exist, this is a no-op.

---

#### Control Flow Instructions

**`Label(name: String)`**
A named jump target. Labels are resolved to instruction indices before execution;
the `Label` instruction itself is a no-op at runtime.

**`Jump(label: String)`**
Unconditionally set the program counter to the instruction at `label`.

**`JumpIfFalse(label: String)`**
Pop the top of the stack. If it is `FALSE` or `NULL`, jump to `label`.
If it is `TRUE`, continue to the next instruction.

**`JumpIfTrue(label: String)`**
Pop the top of the stack. If it is `TRUE`, jump to `label`.
If it is `FALSE` or `NULL`, continue.

**`Halt`**
Stop execution. The result buffer contains the final output.

---

## The Program Type

```
Program {
    instructions: Vec<Instruction>
    labels:       Map<String, usize>   -- label name → instruction index
    result_schema: Vec<String>         -- output column names (set by SetResultSchema)
}
```

Label resolution happens once after all instructions are generated. Labels in
`Jump*` instructions are stored as strings during code generation and resolved to
indices in a post-processing pass.

---

## Code Generation Rules

Each `LogicalPlan` node maps to a sequence of instructions. The compiler performs a
**recursive post-order traversal**: generate code for children before the parent.

### Scan

```
Plan:  Scan { table: "employees", alias: Some("e") }

Code:
  OpenScan(cursor_id=0, table="employees")
  Label("scan_0_loop")
  AdvanceCursor(cursor_id=0, on_exhausted="scan_0_end")
  ... (body generated by parent node)
  Jump("scan_0_loop")
  Label("scan_0_end")
  CloseScan(cursor_id=0)
```

The loop structure is emitted by the Scan code generator. The body (Filter, Project,
etc.) is inserted into the loop by the parent node calling the child recursively.

Cursor IDs are assigned sequentially. Each Scan in the plan gets a unique ID.

### Filter

```
Plan:  Filter { predicate: salary > 50000, input: Scan(...) }

Code:
  [Scan loop header]
    [predicate expression]    ← salary > 50000
    JumpIfFalse("filter_0_skip")
    [body from parent]        ← Project, EmitRow, etc.
    Label("filter_0_skip")
  [Scan loop footer]
```

The filter wraps the body: if the predicate is false, skip the body and advance
the cursor.

### Project

```
Plan:  Project { columns: [name AS "name", salary AS "salary"] }

Code:
  SetResultSchema(["name", "salary"])   ← emitted once at program start
  [Scan loop, Filter wrap]
    BeginRow
    LoadColumn(cursor_id=0, "name")
    EmitColumn("name")
    LoadColumn(cursor_id=0, "salary")
    EmitColumn("salary")
    EmitRow
```

`SetResultSchema` is emitted once at the beginning of the program, not per row.

**SELECT \***:
The compiler inserts a `ScanAllColumns(cursor_id)` pseudo-instruction that expands
at runtime by asking the backend for its column list. This avoids requiring the
compiler to know column names ahead of time.

### Join

A join compiles to a **nested loop**: the outer loop iterates over the left side,
and for each left row, the inner loop iterates over the right side.

```
Plan:  Join { kind: Inner, condition: e.dept_id = d.id,
              left: Scan(employees AS e), right: Scan(departments AS d) }

Code:
  OpenScan(cursor_id=0, table="employees")
  Label("outer_loop")
  AdvanceCursor(cursor_id=0, on_exhausted="outer_end")

    OpenScan(cursor_id=1, table="departments")
    Label("inner_loop")
    AdvanceCursor(cursor_id=1, on_exhausted="inner_end")

      [condition: e.dept_id = d.id]
      JumpIfFalse("inner_continue")
      [body: BeginRow, EmitColumn, EmitRow]
      Label("inner_continue")

    Jump("inner_loop")
    Label("inner_end")
    CloseScan(cursor_id=1)

  Jump("outer_loop")
  Label("outer_end")
  CloseScan(cursor_id=0)
```

**LEFT JOIN:** For each left row that finds no matching right row, emit a row with
NULLs for all right columns. Requires tracking "matched" state with a flag.

**RIGHT JOIN:** Swap left and right, compile as LEFT JOIN, reorder columns.

**FULL JOIN:** Compile as LEFT JOIN, then a second pass over the right side for
unmatched rows.

**CROSS JOIN:** Same nested loop but no condition check.

### Aggregate

Aggregation compiles in two phases: a scan loop that accumulates values, followed
by finalize instructions that emit one row per group.

```
Plan:  Aggregate { group_by: [dept], aggregates: [Count(*)→"cnt"] }
       (wrapping a Scan)

Code:
  [Scan loop]
    [group key: LoadColumn(dept), SaveGroupKey(1)]
    InitAgg(slot=0, func=CountStar)   ← initialized once per group in practice
    UpdateAgg(slot=0)                 ← per row
  [end of scan loop]

  [for each group — emit one row]
  SetResultSchema(["dept", "cnt"])
  BeginRow
  LoadGroupKey(0)
  EmitColumn("dept")
  FinalizeAgg(slot=0)
  EmitColumn("cnt")
  EmitRow
```

The grouping logic uses an internal hash map maintained by the VM: each unique group
key maps to its aggregate slots. The code generator emits group-tracking instructions
(`SaveGroupKey`, `LoadGroupKey`) and the VM manages the hash map.

### Sort, Limit, Distinct

These operate on the completed result buffer and are emitted at the end of the program:

```
SortResult([{ column: "salary", direction: Desc, nulls: NullsLast }])
LimitResult(count=10, offset=0)
DistinctResult
```

### Insert

```
Plan:  Insert { table: "t", columns: ["name", "age"], values: [[Lit("Alice"), Lit(30)]] }

Code:
  LoadConst("Alice")
  LoadConst(30)
  InsertRow(table="t", columns=["name", "age"])
```

For `INSERT INTO t SELECT ...`, the SELECT subplan is compiled first; its EmitRow
instructions are replaced by InsertRow instructions.

### Update and Delete

```
Plan:  Update { table: "employees", assignments: [salary=salary*1.1], predicate: dept='Eng' }

Code:
  OpenScan(cursor_id=0, table="employees")
  Label("update_loop")
  AdvanceCursor(cursor_id=0, on_exhausted="update_end")
    [predicate: dept = 'Eng']
    JumpIfFalse("update_skip")
    LoadColumn(cursor_id=0, "salary")
    LoadConst(1.1)
    BinaryOp(Mul)
    UpdateRows(table="employees", assignments=["salary"])
    Label("update_skip")
  Jump("update_loop")
  Label("update_end")
  CloseScan(cursor_id=0)
```

### DDL

```
Plan:  CreateTable { table: "t", columns: [...], if_not_exists: true }

Code:
  CreateTable(table="t", columns=[...], if_not_exists=true)
  Halt
```

### EmptyResult (from optimizer)

```
Plan:  EmptyResult { columns: ["id", "name"] }

Code:
  SetResultSchema(["id", "name"])
  Halt
```

No scan, no data. The result buffer stays empty.

---

## Label Naming Convention

Labels are prefixed by their logical role and a unique sequential counter:

```
scan_{n}_loop     — top of scan loop for cursor n
scan_{n}_end      — after scan loop for cursor n
filter_{n}_skip   — skip body if filter fails
outer_{n}_loop    — outer join loop
outer_{n}_end     — end of outer join loop
inner_{n}_loop    — inner join loop
inner_{n}_end     — end of inner join loop
update_loop       — update scan loop
update_end        — end of update loop
update_skip       — skip update for non-matching row
agg_loop          — aggregate accumulation loop
group_{n}_start   — start of group-emit block
```

---

## Program Example: Full Pipeline

```sql
SELECT dept, COUNT(*) AS cnt
FROM employees
WHERE active = TRUE
GROUP BY dept
HAVING COUNT(*) > 2
ORDER BY cnt DESC
LIMIT 3
```

Generated program (simplified):

```
00  SetResultSchema(["dept", "cnt"])
01  OpenScan(cursor=0, table="employees")
02  Label("scan_0_loop")
03  AdvanceCursor(cursor=0, on_exhausted="scan_0_end")
04  LoadColumn(cursor=0, "active")    -- WHERE active = TRUE
05  LoadConst(TRUE)
06  BinaryOp(Eq)
07  JumpIfFalse("scan_0_loop")        -- skip if WHERE fails
08  LoadColumn(cursor=0, "dept")      -- group key
09  SaveGroupKey(1)
10  InitAgg(slot=0, CountStar)        -- only on first row of new group
11  UpdateAgg(slot=0)
12  Jump("scan_0_loop")
13  Label("scan_0_end")
14  CloseScan(cursor=0)
-- emit one row per group
15  Label("emit_groups")
16  [for each group in hash map]
17  FinalizeAgg(slot=0)
18  LoadConst(2)
19  BinaryOp(Gt)                      -- HAVING COUNT(*) > 2
20  JumpIfFalse("next_group")
21  BeginRow
22  LoadGroupKey(0)
23  EmitColumn("dept")
24  FinalizeAgg(slot=0)
25  EmitColumn("cnt")
26  EmitRow
27  Label("next_group")
28  [advance group iterator]
-- post-process result buffer
29  SortResult([{column:"cnt", direction:Desc, nulls:NullsLast}])
30  LimitResult(count=3, offset=0)
31  Halt
```

---

## Error Types

```
CodegenError =
    | UnsupportedNode { node_kind: String }
        -- plan node type not yet implemented in the code generator
    | InternalError   { message: String }
        -- invariant violated (e.g. cursor ID counter overflows)
```

The code generator does not perform semantic validation — that was the planner's job.
If the planner produced a valid plan, the code generator should always succeed.

---

## Test Harness

The `sql-codegen` package ships a shared `conformance` module. Tests compile a
`LogicalPlan` and compare the generated instruction sequence against an expected
program.

Conformance tests cover:
1. Scan produces OpenScan + loop + CloseScan
2. Filter inserts JumpIfFalse around the body
3. Project emits SetResultSchema + BeginRow + EmitColumn * n + EmitRow
4. Inner Join produces nested loop
5. Left Join emits NULL-filling for unmatched rows
6. Aggregate emits InitAgg + UpdateAgg + FinalizeAgg
7. Sort emits SortResult at end
8. Limit emits LimitResult at end
9. Distinct emits DistinctResult at end
10. EmptyResult emits only SetResultSchema + Halt
11. Insert emits LoadConst(s) + InsertRow
12. Update emits scan loop + UpdateRows
13. Delete emits scan loop + DeleteRows
14. CreateTable emits CreateTable + Halt
15. DropTable emits DropTable + Halt

---

## Relationship to Existing Packages

- **Depends on** `sql-planner` and `sql-optimizer` for plan types.
- **Used by** `sql-vm` which executes the compiled Program.
- The existing `sql-execution-engine` did execution by directly traversing the AST.
  This package formalizes the "compile to bytecode" step, making execution a clean
  separate concern.
