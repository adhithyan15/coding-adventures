# sql-vm

Dispatch-loop virtual machine that executes compiled SQL programs.

## Where this fits

```
Program  (from sql-codegen)
   │ sql_vm.execute(program, backend)
   ▼
QueryResult { columns, rows, rows_affected }
```

The VM is a textbook interpreter loop: fetch the instruction at the program
counter, execute it, advance the counter. State lives on a handful of
containers:

- `stack` — a Python list used as a LIFO of SQL values
- `cursors` — currently-open backend iterators, keyed by `cursor_id`
- `row_buffer` — the row currently being assembled
- `result_buffer` — completed output rows
- `left_result` — saved left-side rows during `INTERSECT`/`EXCEPT` evaluation
- `agg_table` — `{group_key: [AggState, ...]}` for GROUP BY queries
- `result_schema` — the output column names, set by `SetResultSchema`
- `transaction_handle` — active transaction handle set by `BeginTransaction`

Everything the VM does, it does by matching on the IR instruction type and
mutating one of those containers. There are no nested interpreters, no
code generation at runtime, nothing exotic.

## Usage

```python
from sql_backend import InMemoryBackend
from sql_codegen import compile as codegen_compile
from sql_planner import plan_from_ast
from sql_vm import execute

program = codegen_compile(plan_from_ast(ast))
result = execute(program, backend)
# result.columns → ("name", "salary")
# result.rows → [("Alice", 90000), ("Bob", 70000)]
```

## NULL and three-valued logic

The VM implements SQL's three-valued logic exactly as specified in the
spec. `NULL AND FALSE = FALSE`; `NULL OR TRUE = TRUE`; everything else
involving NULL yields NULL. `JumpIfFalse` treats NULL as false (so
WHERE clauses correctly skip NULL predicates).

## Scalar functions

The `CallScalar(func, n_args)` instruction dispatches to a built-in registry
of ~40 SQLite-compatible functions.  Arguments are popped left-to-right from
the stack; the result is pushed back.

| Category | Functions |
|----------|-----------|
| NULL-handling | `COALESCE`, `IFNULL`, `NULLIF`, `IIF` |
| Type | `TYPEOF`, `CAST` |
| Numeric | `ABS`, `ROUND`, `CEIL`/`CEILING`, `FLOOR`, `SIGN`, `MOD` |
| Math | `SQRT`, `POW`/`POWER`, `LOG`/`LN`, `LOG2`, `LOG10`, `EXP`, `PI`, `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`, `ATAN2`, `DEGREES`, `RADIANS` |
| String | `UPPER`, `LOWER`, `LENGTH`/`LEN`, `TRIM`, `LTRIM`, `RTRIM`, `SUBSTR`/`SUBSTRING`, `REPLACE`, `INSTR` |
| Blob/hex | `HEX`, `UNHEX`, `ZEROBLOB`, `RANDOMBLOB` |
| Misc | `QUOTE`, `CHAR`, `UNICODE`, `SOUNDEX`, `PRINTF`/`FORMAT`, `RANDOM`, `LAST_INSERT_ROWID` |

All math functions return `NULL` for out-of-domain inputs rather than raising.
Null-propagating functions short-circuit to `NULL` when any argument is `NULL`.

You can call any registered function directly:

```python
from sql_vm import call_scalar
call_scalar("upper", ["hello"])  # → "HELLO"
call_scalar("coalesce", [None, 42])  # → 42
```

## Set operations

`UNION`, `INTERSECT`, and `EXCEPT` are all supported in both `DISTINCT` (default)
and `ALL` modes.  The VM evaluates the left side first and saves its result, then
evaluates the right side, then merges:

| Plan node | Semantics |
|-----------|-----------|
| `Union(all=False)` | All rows from both sides, deduplicated |
| `Union(all=True)` | All rows from both sides, duplicates kept |
| `Intersect(all=False)` | Only rows that appear in **both** sides (distinct) |
| `Intersect(all=True)` | `min(left_count, right_count)` copies of each shared row |
| `Except(all=False)` | Rows in left that do not appear in right (distinct) |
| `Except(all=True)` | `max(0, left_count − right_count)` copies of each left row |

## Transactions

Explicit transaction control is supported across multiple `execute()` calls:

```python
from sql_vm import execute, TransactionError

execute(compile(Begin()), backend)       # opens a transaction
execute(compile(insert_plan), backend)   # runs inside the transaction
execute(compile(Commit()), backend)      # commits

# Nesting is rejected:
try:
    execute(compile(Begin()), backend)
    execute(compile(Begin()), backend)   # raises TransactionError
except TransactionError as e:
    ...
```

The VM bridges fresh `_VmState` objects across calls by calling
`backend.current_transaction()` — so `COMMIT`/`ROLLBACK` always find the
handle opened by a prior `BEGIN` even though each `execute()` creates new
state.

## Errors

All failures surface as `VmError` subclasses:

- `TableNotFound`, `ColumnNotFound`
- `TypeMismatch`, `DivisionByZero`
- `ConstraintViolation`, `TableAlreadyExists`
- `StackUnderflow`, `InvalidLabel` (codegen-bug signals)
- `BackendError`, `InternalError`
- `UnsupportedFunction(name)` — function name not in registry
- `WrongNumberOfArguments(name, expected, got)` — arity mismatch
- `TransactionError(message)` — nested BEGIN, or COMMIT/ROLLBACK without BEGIN

## Relationship to other packages

- **Depends on** `sql-codegen` for the `Program` / `Instruction` types
- **Depends on** `sql-backend` for the pluggable `Backend` interface
- **Used by** the `mini-sqlite` façade (PEP 249 DB-API 2.0 adapter)
