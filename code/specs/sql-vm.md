# SQL Virtual Machine Specification

## Overview

This document specifies the `sql-vm` package: a **virtual machine** that executes
a compiled IR `Program` (produced by `sql-codegen`) against a pluggable `Backend`
and returns a `QueryResult`.

The VM is the final stage of the SQL pipeline:

```
Program (from sql-codegen)
    │  sql-vm.execute()
    ▼                        ← calls into Backend for table data
QueryResult { columns, rows }
```

**What is a virtual machine?**

A virtual machine is a program that simulates a simple computer. Instead of real
CPU instructions, it executes *virtual instructions* — the IR opcodes defined in
`sql-codegen`. Like a real CPU, the VM has:

- A **program counter** (`pc`): the index of the next instruction to execute
- A **stack**: a last-in, first-out buffer of values
- A **main loop**: fetch the instruction at `pc`, execute it, advance `pc`

This design is called a **dispatch loop** or **interpreter loop**. It is simple,
portable, and correct. Every one of the 17 repository languages can implement
this loop identically.

---

## Where It Fits

```
Depends on: sql-codegen (for Program and Instruction types)
            sql-backend (for the Backend interface)
Used by:    application code, sql-csv-source adapter, storage backends
```

---

## Supported Languages

All 17 repository languages implement this package.

Entry points:

**Rust**
```rust
pub fn execute(program: &Program, backend: &dyn Backend) -> Result<QueryResult, VmError>
```

**TypeScript**
```typescript
export function execute(program: Program, backend: Backend): QueryResult
// throws VmError
```

**Go**
```go
func Execute(program *Program, backend Backend) (*QueryResult, error)
```

**Python**
```python
def execute(program: Program, backend: Backend) -> QueryResult: ...
# raises VmError
```

**Ruby**
```ruby
def execute(program, backend) # → QueryResult, raises VmError
```

**Elixir**
```elixir
@spec execute(program(), backend()) :: {:ok, query_result()} | {:error, vm_error()}
```

---

## VM State

The VM holds all of its working state in a single `VmState` struct (or equivalent
mutable context object). All fields are private to the VM; only `QueryResult` is
returned to the caller.

```
VmState {
    pc:            usize                          -- program counter
    stack:         Vec<SqlValue>                  -- value stack
    cursors:       Map<u32, Cursor>               -- open table scans
    result_schema: Vec<String>                    -- output column names
    result_buffer: Vec<Row>                       -- completed output rows
    row_buffer:    Map<String, SqlValue>          -- row currently being assembled
    agg_table:     Map<GroupKey, Vec<AggState>>   -- per-group aggregate state
    group_key:     Vec<SqlValue>                  -- current group key (during scan)
    backend:       &dyn Backend                  -- pluggable data source
}

Row         = Vec<SqlValue>              -- ordered by result_schema
GroupKey    = Vec<SqlValue>              -- values of GROUP BY expressions
SqlValue    = Null | Int(i64) | Float(f64) | Text(String) | Bool(bool)
```

The `Cursor` wraps a backend iterator — an object that yields one row at a time
(a `Map<String, SqlValue>`). Some backends return all rows at once (in-memory); others
stream rows. The Cursor abstraction hides this difference.

---

## Execution Loop

The main loop is the core of the VM. It is intentionally simple:

```
fn execute(program, backend) → QueryResult:
    state = VmState::new(backend)
    loop:
        if state.pc >= program.instructions.len():
            break
        instruction = program.instructions[state.pc]
        state.pc += 1
        execute_instruction(instruction, &mut state)?
    return QueryResult {
        columns: state.result_schema,
        rows:    state.result_buffer
    }
```

`execute_instruction` is a match/switch on the instruction type. Each arm handles
one opcode. If the instruction succeeds, execution continues. If it fails, a `VmError`
is returned and execution halts immediately.

---

## Instruction Execution Semantics

### Stack Instructions

**`LoadConst(value)`**
```
state.stack.push(value)
```

**`LoadColumn(cursor_id, column)`**
```
row = state.cursors[cursor_id].current_row()
val = row.get(column).unwrap_or(Null)
state.stack.push(val)
```
If `cursor_id` does not exist or has no current row, push `Null`. This handles
LEFT JOIN NULLs gracefully.

**`Pop`**
```
state.stack.pop()   -- discard; no error if already empty (no-op)
```

---

### Arithmetic and Logic Instructions

**`BinaryOp(op)`**
```
right = state.stack.pop()
left  = state.stack.pop()
result = apply_binary_op(op, left, right)
state.stack.push(result)
```

**`apply_binary_op` semantics:**

NULL propagation: any arithmetic or comparison with NULL input yields NULL.

```
Arithmetic (Add, Sub, Mul, Mod):
  Int op Int   → Int
  Int op Float → Float (promote Int to Float)
  Float op Int → Float
  Float op Float → Float
  _ op _ → VmError::TypeMismatch if non-numeric types involved

Division (Div):
  _ / 0 → VmError::DivisionByZero
  Int / Int → Int (truncating division, sign follows dividend)
  Others → Float rules above

Comparison (Eq, NotEq, Lt, Lte, Gt, Gte):
  Same types → Bool (using natural ordering)
  Int vs Float → promote and compare
  Text vs Text → lexicographic (byte order)
  Bool vs Bool → FALSE < TRUE
  Mixed incompatible types → VmError::TypeMismatch
  Any NULL → NULL (three-valued logic)

And:
  TRUE  AND TRUE  = TRUE
  TRUE  AND FALSE = FALSE
  FALSE AND TRUE  = FALSE
  FALSE AND FALSE = FALSE
  NULL  AND TRUE  = NULL
  NULL  AND FALSE = FALSE
  TRUE  AND NULL  = NULL
  FALSE AND NULL  = FALSE
  NULL  AND NULL  = NULL

Or:
  TRUE  OR TRUE  = TRUE
  TRUE  OR FALSE = TRUE
  FALSE OR TRUE  = TRUE
  FALSE OR FALSE = FALSE
  NULL  OR TRUE  = TRUE
  NULL  OR FALSE = NULL
  TRUE  OR NULL  = TRUE
  FALSE OR NULL  = NULL
  NULL  OR NULL  = NULL

Concat:
  Text || Text → Text (concatenation)
  Null || _    → Null
  _ || Null    → Null
  Non-text + non-text → VmError::TypeMismatch
```

**`UnaryOp(op)`**
```
Neg: Int   → Int (negated)
     Float → Float (negated)
     Null  → Null
     Other → VmError::TypeMismatch

Not: TRUE  → FALSE
     FALSE → TRUE
     Null  → Null
     Other → VmError::TypeMismatch
```

**`IsNull`**
```
val = stack.pop()
stack.push(Bool(val == Null))
```

**`IsNotNull`**
```
val = stack.pop()
stack.push(Bool(val != Null))
```

**`Between`**
```
high  = stack.pop()
low   = stack.pop()
value = stack.pop()
-- equivalent to value >= low AND value <= high (with NULL propagation)
result = and(gte(value, low), lte(value, high))
stack.push(result)
```

**`InList(n)`**
```
list  = stack.pop_n(n)   -- n items
value = stack.pop()
if value == Null:
    stack.push(Null)
    return
found_null = false
for item in list:
    if item == Null:
        found_null = true
    elif item == value:
        stack.push(TRUE)
        return
stack.push(if found_null { Null } else { FALSE })
```

**`Like`**
```
pattern = stack.pop()
value   = stack.pop()
if value == Null or pattern == Null:
    stack.push(Null)
else:
    stack.push(Bool(like_match(value.as_text()?, pattern.as_text()?)))
```

`like_match` rules:
- `%` matches any sequence of zero or more characters
- `_` matches exactly one character
- All other characters match literally (case-sensitive by default)

**`Coalesce(n)`**
```
values = stack.pop_n(n)  -- popped in LIFO order (last arg first)
for v in values.iter() (first-to-last):
    if v != Null:
        stack.push(v)
        return
stack.push(Null)
```

---

### Scan Instructions

**`OpenScan(cursor_id, table)`**
```
iterator = state.backend.scan(table)?
state.cursors.insert(cursor_id, Cursor::new(iterator))
```
Raises `VmError::TableNotFound` if the backend does not know the table.

**`AdvanceCursor(cursor_id, on_exhausted)`**
```
cursor = state.cursors.get_mut(cursor_id)?
if cursor.advance():
    -- cursor has a new current row; continue
else:
    state.pc = state.program.labels[on_exhausted]
```

**`CloseScan(cursor_id)`**
```
cursor = state.cursors.remove(cursor_id)
cursor.close()   -- backend may release resources
```

---

### Row Output Instructions

**`BeginRow`**
```
state.row_buffer.clear()
```

**`EmitColumn(name)`**
```
value = state.stack.pop()
state.row_buffer.insert(name, value)
```

**`EmitRow`**
```
-- Build ordered row from row_buffer according to result_schema
row = state.result_schema.map(|col| state.row_buffer.get(col).unwrap_or(Null))
state.result_buffer.push(row)
state.row_buffer.clear()
```

**`SetResultSchema(columns)`**
```
state.result_schema = columns
```
Must be called before the first `EmitRow`. Calling it more than once overwrites
the schema (but this should not happen in a well-formed program).

---

### Aggregate Instructions

The aggregate state table maps `GroupKey → Vec<AggState>`. A `GroupKey` is the
vector of SQL values produced by the GROUP BY expressions for a given row.

**`InitAgg(slot, func)`**
```
-- Called when a new group key is first seen
key = state.group_key.clone()
if key not in state.agg_table:
    state.agg_table.insert(key, Vec::new())
slot_vec = state.agg_table.get_mut(key)
while slot_vec.len() <= slot:
    slot_vec.push(AggState::new(func))
```

`AggState` initial values:
```
COUNT(*):  { count: 0 }
COUNT(col): { count: 0 }
SUM:        { sum: Null }
AVG:        { sum: Null, count: 0 }
MIN:        { min: Null }
MAX:        { max: Null }
```

**`UpdateAgg(slot)`**
```
value = state.stack.pop()
key   = state.group_key.clone()
agg   = state.agg_table.get_mut(key)[slot]
match agg.func:
    CountStar   → agg.count += 1
    Count(col)  → if value != Null: agg.count += 1
    Sum         → if value != Null: agg.sum = agg.sum + value (or value if null)
    Avg         → if value != Null: agg.sum = agg.sum + value; agg.count += 1
    Min         → if value != Null: agg.min = min(agg.min, value)
    Max         → if value != Null: agg.max = max(agg.max, value)
```

**`FinalizeAgg(slot)`**
```
key = state.group_key.clone()
agg = state.agg_table.get(key)[slot]
result = match agg.func:
    CountStar | Count → Int(agg.count)
    Sum               → agg.sum
    Avg               → if agg.count == 0: Null else: Float(agg.sum / agg.count)
    Min               → agg.min
    Max               → agg.max
state.stack.push(result)
```

**`SaveGroupKey(n)`**
```
values = state.stack.pop_n(n)
state.group_key = values  -- ordered by GROUP BY position
```

**`LoadGroupKey(i)`**
```
state.stack.push(state.group_key[i].clone())
```

**GROUP BY execution model:**

When executing a GROUP BY query, the scan loop runs to completion, accumulating
aggregate state for each group into `agg_table`. After the scan loop, a second
loop (generated by `sql-codegen`) iterates over all groups in `agg_table`,
sets `group_key` for each group, and emits one output row per group.

---

### Sort, Limit, and Distinct Instructions

These operate on `state.result_buffer` directly.

**`SortResult(keys)`**
```
state.result_buffer.sort_by(|a, b| {
    for key in keys:
        col_idx = result_schema.index(key.column)
        a_val = a[col_idx]
        b_val = b[col_idx]
        ord = compare_sql_values(a_val, b_val, key.nulls)
        if key.direction == Desc: ord = ord.reverse()
        if ord != Equal: return ord
    return Equal
})
```

NULL ordering:
- `NullsLast (ASC default)`: NULLs sort after all non-NULL values
- `NullsFirst (DESC default)`: NULLs sort before all non-NULL values

**`LimitResult(count, offset)`**
```
start = offset.unwrap_or(0)
end   = match count:
    Some(n) → min(start + n, result_buffer.len())
    None    → result_buffer.len()
state.result_buffer = result_buffer[start..end]
```

**`DistinctResult`**
```
seen = HashSet::new()
result_buffer.retain(|row| {
    key = DedupeKey(row)   -- NULLs compare equal for deduplication
    seen.insert(key)       -- returns false if already present
})
```

---

### Mutation Instructions

**`InsertRow(table, columns)`**
```
values = stack.pop_n(columns.len())   -- last column value popped first
row    = zip(columns, values).into_map()
state.backend.insert(table, row)?
```
Raises `VmError::TableNotFound` or `VmError::ConstraintViolation`.

**`UpdateRows(table, assignments)`**
```
count = 0
-- The backend's cursor is already open and positioned (by the enclosing scan loop)
values = stack.pop_n(assignments.len())
updates = zip(assignments, values).into_map()
state.backend.update_current_row(table, cursor_id, updates)?
count += 1
stack.push(Int(count))
```

**`DeleteRows(table)`**
```
state.backend.delete_current_row(table, cursor_id)?
stack.push(Int(1))   -- rows deleted (always 1 per invocation in this model)
```

**`CreateTable(table, columns, if_not_exists)`**
```
state.backend.create_table(table, columns, if_not_exists)?
```
Raises `VmError::TableAlreadyExists` if `if_not_exists=false` and table exists.

**`DropTable(table, if_exists)`**
```
state.backend.drop_table(table, if_exists)?
```
Raises `VmError::TableNotFound` if `if_exists=false` and table doesn't exist.

---

### Control Flow Instructions

**`Label(name)`**
No-op at runtime. Labels are resolved before execution starts; the instruction
is kept as a placeholder so instruction indices remain stable.

**`Jump(label)`**
```
state.pc = state.program.labels[label]
```

**`JumpIfFalse(label)`**
```
val = state.stack.pop()
if val == FALSE or val == Null:
    state.pc = state.program.labels[label]
```

**`JumpIfTrue(label)`**
```
val = state.stack.pop()
if val == TRUE:
    state.pc = state.program.labels[label]
```

**`Halt`**
```
state.pc = program.instructions.len()   -- causes loop exit
```

---

## QueryResult

The return type of `execute`:

```
QueryResult {
    columns: Vec<String>      -- column names in output order
    rows:    Vec<Vec<SqlValue>>  -- rows; each row is ordered by columns
}
```

An empty result (no rows) is valid. An empty result with no columns is valid
(returned for DML statements like INSERT, UPDATE, DELETE, and for DDL).

For DML, `QueryResult` carries a `rows_affected` count:

```
QueryResult {
    columns:       []
    rows:          []
    rows_affected: Option<u64>   -- Some(n) for INSERT/UPDATE/DELETE; None for SELECT/DDL
}
```

---

## Error Types

```
VmError =
    | TableNotFound      { table: String }
    | ColumnNotFound     { cursor_id: u32, column: String }
    | TypeMismatch       { expected: String, got: String, context: String }
    | DivisionByZero
    | ConstraintViolation { table: String, column: String, message: String }
    | TableAlreadyExists { table: String }
    | StackUnderflow                          -- internal: pop from empty stack
    | InvalidLabel       { label: String }   -- internal: jump to unknown label
    | BackendError       { message: String }  -- backend returned an error
    | InternalError      { message: String }  -- invariant violated
```

`StackUnderflow` and `InvalidLabel` should never occur for programs produced by a
correct `sql-codegen`. They indicate a code generator bug.

---

## Execution Trace (Debug Mode)

Implementations may optionally support a **trace mode** that logs each instruction
as it executes:

```
[pc=0]  SetResultSchema(["name", "salary"])
[pc=1]  OpenScan(cursor=0, table="employees")
[pc=2]  Label("scan_0_loop")               -- no-op
[pc=3]  AdvanceCursor(cursor=0)            -- row: {name:"Alice", salary:90000, active:true}
[pc=4]  LoadColumn(cursor=0, "active")     -- stack: [TRUE]
[pc=5]  LoadConst(TRUE)                    -- stack: [TRUE, TRUE]
[pc=6]  BinaryOp(Eq)                       -- stack: [TRUE]
[pc=7]  JumpIfFalse("scan_0_loop")         -- not taken (TRUE)
[pc=8]  BeginRow
...
```

Trace mode is gated behind a boolean flag passed to `execute` (or a separate
`execute_traced` entry point) — it must not impose overhead in normal mode.

---

## Test Harness

The `sql-vm` package ships a shared `conformance` module. Tests provide a `Program`
directly (not going through the planner/optimizer/codegen) and an `InMemoryBackend`
to isolate VM behavior from the rest of the pipeline.

Conformance tests cover:
1. Simple SELECT executes correctly and returns correct rows and column names
2. WHERE filter excludes non-matching rows
3. NULL values propagate through arithmetic and comparisons
4. AND / OR truth tables match SQL three-valued logic spec
5. ORDER BY sorts ascending and descending; NULLs last/first
6. LIMIT restricts count; OFFSET skips rows
7. DISTINCT removes duplicate rows
8. COUNT(*), SUM, AVG, MIN, MAX return correct values
9. GROUP BY partitions correctly; empty groups produce no output
10. HAVING filters groups after aggregation
11. INNER JOIN returns only matching row pairs
12. LEFT JOIN includes all left rows; unmatched right columns are NULL
13. EmptyResult returns zero rows immediately
14. INSERT calls backend.insert with correct column values
15. UPDATE calls backend.update for matching rows
16. DELETE calls backend.delete for matching rows
17. Division by zero raises VmError::DivisionByZero
18. Unknown table raises VmError::TableNotFound
19. Type mismatch raises VmError::TypeMismatch

---

## Relationship to Existing Packages

- **Depends on** `sql-codegen` for `Program` and `Instruction` types.
- **Depends on** `sql-backend` for the `Backend` interface.
- The existing `sql-execution-engine` fused planning, optimization, and execution in
  one pass. This package is the execution-only stage — it knows nothing about SQL
  syntax, ASTs, or plan trees. It only executes instructions.
- `sql-csv-source` implements `Backend` for CSV files and will work with the new VM
  without modification, as long as it implements the updated `Backend` interface.
