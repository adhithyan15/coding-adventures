# SQL FOREIGN KEY Constraints

**Spec ID**: sql-foreign-keys
**Status**: Implemented (v1.1.0 sql-vm / v1.0.0 mini-sqlite)
**Packages**: `sql-vm`, `sql-codegen`, `sql-backend`, `sql-lexer`, `sql-parser`, `mini-sqlite`
**Depends on**: sql-vm v1.0.x (CHECK constraint support already present)

---

## 1. Motivation

A FOREIGN KEY constraint declares a referential dependency between two tables.
The column in the *child* table must contain a value that exists in the
referenced column of the *parent* table:

```sql
-- Parent: canonical identity
CREATE TABLE customers (
    id   INTEGER PRIMARY KEY,
    name TEXT
);

-- Child: every order must reference a real customer
CREATE TABLE orders (
    id          INTEGER PRIMARY KEY,
    customer_id INTEGER REFERENCES customers(id)
);
```

Without FK constraints the database cannot guarantee referential integrity —
a child row can reference a parent that was never inserted or has since been
deleted, silently corrupting query joins.

---

## 2. SQL semantics

### 2.1 Column-level REFERENCES

This spec implements *column-level* REFERENCES only:

```
col_def = NAME TYPE { col_constraint } ;
col_constraint = ... | ( "REFERENCES" parent_table [ "(" parent_col ")" ] ) ;
```

When the parent column is omitted (`REFERENCES customers`), the constraint
references the PRIMARY KEY of the parent table.  The VM discovers the PK at
enforcement time by scanning `backend.columns(parent_table)`.

### 2.2 NULL semantics

A NULL value in the FK column is *always allowed*, regardless of whether a
matching parent row exists.  This matches the SQL standard: NULL means "this
reference is unknown or inapplicable", not "violation".

### 2.3 Enforcement timing

| Operation | Check |
|---|---|
| `INSERT` child | Verify parent row exists |
| `UPDATE` child | Verify new FK value's parent row exists |
| `DELETE` parent | Verify no child rows reference this row (RESTRICT) |
| `UPDATE` parent PK | Not enforced in this phase |

### 2.4 Default action: RESTRICT

When a parent row is deleted and at least one child row references it, the
delete is rejected with a `ConstraintViolation`.  `CASCADE` and `SET NULL`
are out of scope for Phase 4b.

---

## 3. Grammar changes

### 3.1 New keyword: `REFERENCES`

`REFERENCES` is added to `sql.tokens` and the compiled `keywords` list in
`sql_lexer/_grammar.py`.

### 3.2 `col_constraint` rule extension

```
col_constraint = ... | ( "REFERENCES" NAME [ "(" NAME ")" ] ) ;
```

The `Optional(Group(...))` wrapper around the column clause means both forms
tokenize and parse without ambiguity.

---

## 4. Pipeline changes

### 4.1 sql-backend: `ColumnDef.foreign_key`

```python
foreign_key: object = field(default=None, compare=False, hash=False)
# Runtime value: (ref_table: str, ref_col: str | None) or None
```

Typed `object` to avoid a circular import between `sql-backend` and the
planner's expression types.  `compare=False, hash=False` preserves existing
equality/hash semantics.

### 4.2 mini-sqlite adapter: `_col_def()` parsing

The adapter recognises `REFERENCES` in the `col_constraint` child nodes and
collects the NAME tokens:

- First NAME token → `ref_table`
- Second NAME token (inside optional `(col)` group) → `ref_col` or `None`

The pair is stored as `BackendColumnDef.foreign_key = (ref_table, ref_col)`.

### 4.3 sql-codegen: `IrColumnDef.foreign_key`

```python
@dataclass(frozen=True, slots=True)
class ColumnDef:
    ...
    foreign_key: tuple[str, str | None] | None = None
```

`_to_ir_col()` copies `c.foreign_key` directly into the IR struct.  No
compilation step is needed (FK enforcement is a data lookup, not expression
evaluation).

### 4.4 sql-vm: FK registries and enforcement

#### Registry shape

```python
fk_child: dict[str, list[tuple[str, str, str | None]]]
# child_table → [(child_col, parent_table, parent_col_or_None), ...]

fk_parent: dict[str, list[tuple[str, str, str | None]]]
# parent_table → [(child_table, child_col, parent_col_or_None), ...]
```

Both dicts are passed in from `Connection` and persist across `execute()` calls.

#### `_do_create_table` population

For every column with a non-`None` `foreign_key`:

```python
ref_table, ref_col = col.foreign_key
# Forward lookup (child inserts/updates)
st.fk_child.setdefault(ins.table, []).append((col.name, ref_table, ref_col))
# Reverse lookup (parent deletes)
st.fk_parent.setdefault(ref_table, []).append((ins.table, col.name, ref_col))
```

#### `_fk_find_pk(table, backend)`

Scans `backend.columns(table)` for a column with `primary_key=True`.
Falls back to `"id"` if none is found (or if `backend.columns` raises).

#### `_fk_row_exists(table, col, value, backend)`

Opens a scan cursor on `table`, walks rows until it finds one where
`row.get(col) == value`, then closes the cursor.  O(n) — acceptable for the
reference implementation.

#### `_check_fk_child(table, row, st)`

For each `(child_col, parent_table, parent_col)` in `fk_child[table]`:
1. Get `value = row.get(child_col)`.
2. If `value is None`, skip.
3. Resolve `parent_col` via `_fk_find_pk` if it is `None`.
4. If `_fk_row_exists(parent_table, parent_col, value, backend)` is False →
   raise `ConstraintViolation`.

#### `_check_fk_parent(table, row, st)`

For each `(child_table, child_col, parent_col)` in `fk_parent[table]`:
1. Resolve `parent_col` via `_fk_find_pk(table, backend)` if None.
2. Get `value = row.get(parent_col)`.
3. If `value is None`, skip (no child can reference a NULL PK).
4. If `_fk_row_exists(child_table, child_col, value, backend)` is True →
   raise `ConstraintViolation`.

#### INSERT and UPDATE enforcement

`_do_insert` calls `_check_fk_child(ins.table, row, st)` after `_check_constraints`
but before `backend.insert`.

`_do_update` merges `{**current, **assignments}` and calls `_check_fk_child`
on the merged row before `backend.update`.

#### DELETE enforcement

`_do_delete` reads `st.current_row.get(ins.cursor_id, {})` and calls
`_check_fk_parent(ins.table, current_row, st)` before `backend.delete`.

### 4.5 mini-sqlite: `Connection._fk_child` / `_fk_parent`

```python
self._fk_child: dict = {}
self._fk_parent: dict = {}
```

Both dicts initialized in `Connection.__init__` and passed as keyword
arguments through `Cursor.execute()` → `engine.run()` → `vm.execute()`.

---

## 5. Error handling

A FK violation raises `ConstraintViolation` from `sql_vm`.  The mini-sqlite
layer translates this to `IntegrityError` via the existing `translate()`
mapping.  The error message identifies the direction of the failure:

```
# Child side (missing parent):
FOREIGN KEY constraint failed: orders.customer_id → customers.id = 42

# Parent side (RESTRICT delete):
FOREIGN KEY constraint failed: cannot delete customers.id = 42,
                                referenced by orders.customer_id
```

---

## 6. Limitations and future work

- **Table-level FOREIGN KEY** — `FOREIGN KEY (col) REFERENCES t(col)` after
  all column definitions is not yet supported.
- **CASCADE / SET NULL / SET DEFAULT** — only RESTRICT (the default) is
  implemented.
- **UPDATE parent PK** — updating a parent's referenced column is not checked
  in this phase.
- **Deferred constraints** — not applicable without SAVEPOINT support
  (Phase 7).
- **Performance** — FK checks use O(n) scans.  A future phase can use the
  index advisor's auto-index infrastructure to speed up FK lookups.

---

## 7. Test strategy

### Pipeline unit tests

| Test | What it verifies |
|---|---|
| `test_grammar_parses_references` | Grammar accepts `REFERENCES t(col)` |
| `test_grammar_parses_references_no_column` | Grammar accepts `REFERENCES t` |
| `test_adapter_populates_foreign_key` | `ColumnDef.foreign_key = ("customers", "id")` |
| `test_adapter_populates_foreign_key_no_column` | ref_col is None when omitted |
| `test_codegen_stores_foreign_key_in_ir` | IR `ColumnDef.foreign_key` matches |

### Integration tests

| Test | What it verifies |
|---|---|
| `test_insert_child_valid` | Valid INSERT succeeds |
| `test_insert_multiple_children_same_parent` | Many children for one parent |
| `test_insert_null_fk_passes` | NULL FK value is allowed |
| `test_delete_child_then_delete_parent` | Delete after removing children |
| `test_no_fk_table_unaffected` | Tables without FK unchanged |
| `test_fk_survives_many_inserts` | Registry persists across many calls |

### Error cases

| Test | What it verifies |
|---|---|
| `test_insert_child_missing_parent` | Missing parent → IntegrityError |
| `test_insert_child_missing_parent_no_customers_at_all` | Empty parent table |
| `test_update_child_to_missing_parent` | UPDATE FK violation; row unchanged |
| `test_delete_parent_with_child_raises` | RESTRICT delete; parent row survives |
| `test_violation_message_mentions_fk` | Error mentions "FOREIGN KEY" |
| `test_second_fk_column_enforced` | Multi-FK columns each enforced |
| `test_table_not_found_still_raises_operational_error` | Unrelated errors unchanged |

### VM-level tests (`test_dml_ddl.py`)

| Test | What it verifies |
|---|---|
| `test_fk_insert_valid` | Valid INSERT at VM level |
| `test_fk_insert_violates` | Missing parent at VM level |
| `test_fk_null_child_passes` | NULL passthrough at VM level |
| `test_fk_update_child_violates` | UPDATE violation at VM level; row unchanged |
| `test_fk_delete_parent_restricted` | RESTRICT at VM level; parent row survives |
| `test_fk_delete_parent_no_children` | Unreferenced parent delete succeeds |
