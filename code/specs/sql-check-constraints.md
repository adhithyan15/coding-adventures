# SQL CHECK Constraints

**Spec ID**: sql-check-constraints
**Status**: Implemented (v1.0.0 sql-vm / v0.9.0 mini-sqlite)
**Packages**: `sql-vm`, `sql-codegen`, `sql-backend`, `sql-lexer`, `sql-parser`, `mini-sqlite`
**Depends on**: sql-vm v0.9.x (ALTER TABLE support already present)

---

## 1. Motivation

A `CHECK` constraint is an inline predicate attached to a column definition.  The
database engine evaluates it on every `INSERT` and `UPDATE`; if the result is
`FALSE` the write is rejected with an integrity error.

```sql
-- Positive score only
CREATE TABLE scores (id INTEGER, value INTEGER CHECK (value > 0));

-- Range constraint using AND
CREATE TABLE percentages (id INTEGER, pct INTEGER CHECK (pct >= 0 AND pct <= 100));

-- Multiple columns each with their own constraint
CREATE TABLE inventory (
    id INTEGER,
    qty  INTEGER CHECK (qty  >= 0),
    cost INTEGER CHECK (cost >= 0)
);
```

Without `CHECK`, callers must validate inputs in application code; a CHECK
constraint makes the invariant a database-enforced property regardless of which
code path performs the write.

---

## 2. SQL semantics

### 2.1 Three-valued logic

SQL uses *three-valued logic*: `TRUE`, `FALSE`, and `UNKNOWN` (produced whenever
one operand is NULL).  A CHECK constraint passes if and only if its expression
does **not** evaluate to `FALSE`.  Specifically:

| Expression result | Write allowed? |
|---|---|
| TRUE  | yes |
| UNKNOWN (NULL) | yes — constraint is satisfied by convention |
| FALSE | **no** — `ConstraintViolation` raised |

This matches the SQL standard and SQLite's behaviour.

### 2.2 Column-level vs table-level

This spec covers *column-level* CHECK constraints only (`col_def CHECK (expr)`).
Table-level CHECK constraints (`CONSTRAINT name CHECK (expr)` after all column
definitions) are out of scope.

### 2.3 Expression scope

The expression may reference any column defined in the same `CREATE TABLE`
statement, not just the column it is attached to.  For example:

```sql
CREATE TABLE range_check (
    lo INTEGER,
    hi INTEGER CHECK (hi > lo)
);
```

---

## 3. Grammar changes

### 3.1 New keyword: `CHECK`

`CHECK` is added to the token list (`sql.tokens`) and compiled into the lexer's
`keywords` list in `sql_lexer/_grammar.py`.  Without this, the lexer emits
`CHECK` as a NAME token and the parser cannot match the literal.

### 3.2 `col_constraint` rule extension

```
col_constraint = ( "NOT" "NULL" ) | "NULL" | ( "PRIMARY" "KEY" )
               | "UNIQUE" | ( "DEFAULT" primary )
               | ( "CHECK" "(" expr ")" ) ;
```

The `expr` rule reference allows any valid SQL expression — comparisons,
compound `AND`/`OR`, function calls.  Because `expr` is already defined and
recursive, no additional grammar machinery is needed.

---

## 4. Pipeline changes

### 4.1 sql-backend: `ColumnDef.check_expr`

```python
@dataclass
class ColumnDef:
    name: str
    type_name: str
    nullable: bool = True
    primary_key: bool = False
    default: object = None
    check_expr: object = field(default=None, compare=False, hash=False)
```

`check_expr` is typed as `object` to avoid a circular dependency: the backend
package must not import planner `Expr` types.  `compare=False, hash=False`
preserve the existing equality and hash semantics for downstream code that
compares `ColumnDef` objects.

### 4.2 mini-sqlite adapter: `_col_def()` parsing

The adapter recognises the new `CHECK ( expr )` grammar variant within the
`col_constraint` child nodes of a `col_def` parse-tree node.  When found, it
calls `_expr(expr_node)` to convert the AST expression node into a planner
`Expr` object and stores it in `BackendColumnDef.check_expr`.

### 4.3 sql-codegen: `CHECK_CURSOR_ID` and `IrColumnDef.check_instrs`

```python
CHECK_CURSOR_ID: int = -1

@dataclass(frozen=True, slots=True)
class ColumnDef:
    name: str
    type: str
    nullable: bool = True
    check_instrs: tuple[Instruction, ...] = ()
```

`CHECK_CURSOR_ID = -1` is a sentinel that can never be allocated by normal cursor
management (real cursors start at 0).  It is used as a synthetic cursor identity
during CHECK expression compilation and evaluation.

`_to_ir_col()` in the compiler checks whether `AstColumnDef.check_expr` is
non-`None`.  If so, it creates a fresh `_Ctx` with
`alias_to_cursor[""] = CHECK_CURSOR_ID`, then calls `_compile_expr` on the
expression.  Unqualified column references (`Column(table=None, col="x")`) are
resolved through the empty-string alias, so they produce
`LoadColumn(cursor_id=-1, column="x")` instructions that work against any row
dict keyed by column name.

### 4.4 sql-vm: check registry and enforcement

#### Registry shape

```python
check_registry: dict[str, list[tuple[str, tuple[Instruction, ...]]]]
# table → [(col_name, check_instrs), ...]
```

The registry is a plain mutable `dict` passed into `vm.execute()` from the
caller.  `Connection` owns the single dict instance for its lifetime; all
`Cursor.execute()` calls share the same object so mutations from
`CREATE TABLE` are visible to subsequent `INSERT`/`UPDATE` calls.

#### `_do_create_table` population

When the VM executes a `CreateTable` instruction, it iterates the IR
`ColumnDef` list and collects any column whose `check_instrs` is non-empty
into the registry:

```python
checks = [(c.name, c.check_instrs) for c in ins.columns if c.check_instrs]
if checks:
    st.check_registry[ins.table] = checks
```

#### `_check_constraints()` helper

```python
def _check_constraints(table, row, st):
    constraints = st.check_registry.get(table)
    if not constraints:
        return
    st.current_row[CHECK_CURSOR_ID] = row
    try:
        for col_name, instrs in constraints:
            for instr in instrs:
                _dispatch(instr, st)
            result = st.pop()
            if result is False:
                raise ConstraintViolation(table=table, column=col_name, ...)
    finally:
        st.current_row.pop(CHECK_CURSOR_ID, None)
```

The key insight is that `st.current_row` is the same dict used by `LoadColumn`
to resolve cursor rows.  By temporarily inserting `CHECK_CURSOR_ID → row`, all
`LoadColumn(cursor_id=-1, column=c)` instructions resolve against the incoming
row without any special-casing in the `LoadColumn` dispatch.

Three-valued logic: only `result is False` raises; `None` (NULL) falls through.

#### INSERT enforcement

`_do_insert` calls `_check_constraints(ins.table, row, st)` after building
the row dict from the value stack but before calling `backend.insert`.  If a
violation is raised, the backend write never happens.

#### UPDATE enforcement

`_do_update` merges pending assignments with the current row and validates the
merged dict before writing:

```python
current = st.current_row.get(ins.cursor_id, {})
_check_constraints(ins.table, {**current, **assignments}, st)
```

This ensures the constraint is evaluated against the *post-update* state.  If
a violation is raised, the backend write is skipped and the row is unchanged.

### 4.5 mini-sqlite: `Connection._check_registry`

```python
self._check_registry: dict = {}
```

Initialized once in `Connection.__init__` and passed as
`check_registry=self._connection._check_registry` in `Cursor.execute()`.

---

## 5. Error handling

A CHECK violation raises `ConstraintViolation` from `sql_vm`.  The mini-sqlite
layer translates this to `IntegrityError` via the existing `translate()`
mapping.  The error message includes the table and column name:

```
CHECK constraint failed: scores.value
```

---

## 6. Limitations and future work

- **Table-level CHECK** — constraints declared after all column definitions
  (`CONSTRAINT chk_name CHECK (expr)`) are not yet supported.
- **Column references across tables** — multi-table CHECK expressions are
  architecturally unsupported by the current sentinel-cursor approach; they
  require correlated sub-select semantics.
- **FOREIGN KEY** — cross-table referential integrity is a separate Phase 4b
  feature tracked in `sql-foreign-keys.md`.
- **Deferred constraints** — not applicable; mini-sqlite has no SAVEPOINT-aware
  deferred evaluation layer yet.

---

## 7. Test strategy

### Unit tests (pipeline stages)

| Test | What it verifies |
|---|---|
| `test_grammar_parses_check_constraint` | Grammar accepts `CHECK (expr)` without parse error |
| `test_grammar_parses_compound_check` | Grammar accepts `CHECK (x >= 0 AND x <= 100)` |
| `test_adapter_populates_check_expr` | `ColumnDef.check_expr` is a `BinaryExpr` with correct `op` |
| `test_codegen_compiles_check_instrs` | IR `ColumnDef.check_instrs` is non-empty; first is `LoadColumn` with `CHECK_CURSOR_ID` |

### Integration tests (end-to-end SQL)

| Test | What it verifies |
|---|---|
| `test_insert_valid_row_passes` | Valid INSERT succeeds |
| `test_insert_boundary_value_passes` | Boundary value passes |
| `test_insert_null_passes` | NULL bypasses CHECK (three-valued logic) |
| `test_update_valid_passes` | UPDATE to valid value passes |
| `test_multiple_check_columns` | All column constraints enforced independently |
| `test_compound_check_expression` | AND-compound range constraint works |
| `test_no_check_table_unaffected` | Tables without CHECK are unaffected |
| `test_check_survives_other_inserts` | Registry persists across many `execute()` calls |

### Error cases

| Test | What it verifies |
|---|---|
| `test_insert_violates_check` | Negative value raises `IntegrityError` |
| `test_insert_zero_violates_check` | Zero violates `> 0` |
| `test_update_violates_check` | Violating UPDATE raises; row unchanged |
| `test_second_column_check_violated` | Each column's constraint enforced |
| `test_violation_message_mentions_column` | Error message contains column name |
| `test_compound_check_lower_bound_violated` | Lower bound of range enforced |
| `test_compound_check_upper_bound_violated` | Upper bound of range enforced |
| `test_table_not_found_still_raises_operational_error` | Unrelated errors unaffected |

### VM-level tests (sql-vm `test_dml_ddl.py`)

| Test | What it verifies |
|---|---|
| `test_check_constraint_insert_valid` | Valid INSERT at VM level |
| `test_check_constraint_insert_violates` | Violating INSERT at VM level |
| `test_check_constraint_update_violates` | Violating UPDATE at VM level; row unchanged |
| `test_check_null_passes` | NULL passthrough at VM level |
