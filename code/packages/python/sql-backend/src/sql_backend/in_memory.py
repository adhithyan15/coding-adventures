"""
InMemoryBackend — the reference Backend implementation
======================================================

This is the backend the mini-sqlite facade uses for ``connect(":memory:")``,
the one the test suite runs most of its assertions against, and the model
every other backend is measured against via the conformance tests.

Design choices
--------------

**Storage shape.** Each table is represented by a :class:`_Table` holding a
list of :class:`ColumnDef` (the schema) and a list of :data:`Row` (the rows,
in insertion order). Tables live in ``self._tables`` keyed by table name.

We use a list for rows rather than a dict keyed by some rowid for two
reasons. First, SQL semantics care about *insertion order* unless ``ORDER
BY`` says otherwise — a list preserves order for free. Second, the list
index is already a perfectly good rowid for positioned DML, so
:class:`ListCursor` can mutate rows in place without a separate id-to-row
map.

**Constraint enforcement.** Done lazily, per insert/update. We don't keep
per-column uniqueness indexes — a linear scan of the rows list is
correct, simple, and fast enough for the scale this backend targets
(thousands of rows, not millions). Trading speed for clarity here is a
deliberate choice.

**Transactions.** Snapshot-and-restore. ``begin_transaction`` deep-clones
``self._tables`` and stashes the clone in ``self._snapshot``. ``commit``
just drops the snapshot. ``rollback`` replaces ``self._tables`` with the
snapshot. This is O(total data size) per transaction, which would be
disastrous at scale but is fine for a pedagogical in-memory backend.

**Thread safety.** None. This backend is single-threaded. The ``Backend``
interface does not require concurrency; backends that need it (SQLite WAL,
a remote server) handle it internally.

Why this is longer than it looks
--------------------------------

Constraint checking has a lot of cases: NOT NULL on insert, NOT NULL on
update (only if the column is in the assignment), UNIQUE on insert (must
scan all rows), UNIQUE on update (must scan all rows *except* the one being
updated), default application (present vs absent vs NULL-with-default).
Each case has its own error path. The helper methods ``_apply_defaults`` /
``_check_not_null`` / ``_check_unique`` isolate each concern.
"""

from __future__ import annotations

import copy
from collections.abc import Iterator
from typing import Final

from .backend import Backend, TransactionHandle
from .errors import (
    ColumnAlreadyExists,
    ColumnNotFound,
    ConstraintViolation,
    IndexAlreadyExists,
    IndexNotFound,
    TableAlreadyExists,
    TableNotFound,
    TriggerAlreadyExists,
    TriggerNotFound,
    Unsupported,
)
from .index import IndexDef
from .row import Cursor, ListCursor, ListRowIterator, Row, RowIterator
from .schema import ColumnDef, TriggerDef
from .values import SqlValue

# ---------------------------------------------------------------------------
# SQLite-compatible sort key for in-memory index scans
# ---------------------------------------------------------------------------


def _sql_sort_key(v: SqlValue) -> tuple[int, object]:
    """Map a SQL value to a comparable Python key using SQLite ordering.

    SQLite BINARY collation orders values as::

        NULL (0) < INTEGER / REAL (1) < TEXT (2) < BLOB (3)

    Integers and floats are compared numerically across types (``2.0 == 2``).
    Text is compared by UTF-8 byte values (case-sensitive).  Blobs compare
    by raw byte value.

    Returns a ``(class, value)`` tuple that Python's ``<`` operator sorts
    correctly for each class.  Within class 1 (numeric) the raw Python value
    is used — Python handles mixed int/float comparisons natively.

    Examples::

        _sql_sort_key(None)    # (0, None)   — smallest
        _sql_sort_key(42)      # (1, 42)
        _sql_sort_key(3.14)    # (1, 3.14)
        _sql_sort_key("hi")    # (2, b"hi")  — text sorted as UTF-8 bytes
        _sql_sort_key(b"\\x00") # (3, b"\\x00") — blob last
    """
    if v is None:
        return (0, b"")  # sentinel: None < everything; b"" avoids cross-type compare
    if isinstance(v, bool):
        # bool is a subclass of int in Python; treat as integer.
        return (1, int(v))
    if isinstance(v, (int, float)):
        return (1, v)
    if isinstance(v, str):
        return (2, v.encode("utf-8"))
    if isinstance(v, (bytes, bytearray)):
        return (3, bytes(v))
    # SqlValue is a closed union — if we reach here the caller passed a
    # value that is not part of the type contract.  Raise immediately rather
    # than silently leaking the repr() of an arbitrary object (which could
    # contain secrets or cause non-deterministic sort behaviour).
    raise TypeError(  # noqa: TRY301
        f"_sql_sort_key: unsupported value type {type(v).__name__!r}"
    )


class _Table:
    """Storage for one table — schema plus rows, in insertion order."""

    def __init__(self, columns: list[ColumnDef]) -> None:
        self.columns: list[ColumnDef] = list(columns)
        self.rows: list[Row] = []

    def column_def(self, name: str) -> ColumnDef | None:
        """Return the ColumnDef for ``name``, or None if no such column."""
        for col in self.columns:
            if col.name == name:
                return col
        return None


class InMemoryBackend(Backend):
    """Reference Backend implementation — stores all data in Python dicts and lists.

    Construct empty, then populate via ``create_table`` + ``insert``, or use
    :meth:`from_tables` to preload schema and rows in one call (useful in
    test fixtures).
    """

    def __init__(self) -> None:
        self._tables: dict[str, _Table] = {}
        # Index store: name → IndexDef.  The actual index data is not
        # maintained incrementally (inserts/updates/deletes don't update
        # in-memory index structures); scan_index does a linear scan at
        # call time.  This is fine for a pedagogical reference backend.
        self._indexes: dict[str, IndexDef] = {}
        # Trigger stores: name → TriggerDef (for uniqueness checks and DROP),
        # and table → ordered list of TriggerDef (for firing order).
        self._triggers: dict[str, TriggerDef] = {}
        self._triggers_by_table: dict[str, list[TriggerDef]] = {}
        # Snapshot for the currently-active transaction, if any. ``None``
        # means no transaction is open. We store a full deep copy rather
        # than a diff log because it is dramatically simpler and the data
        # volumes targeted by this backend are small.
        self._snapshot: dict[str, _Table] | None = None
        self._index_snapshot: dict[str, IndexDef] | None = None
        # We hand out handles as monotonically increasing integers. Reusing
        # handles across transactions would make stale-handle bugs silent;
        # this way an old handle will never match a new transaction.
        self._next_handle: int = 1
        self._active_handle: int | None = None
        # Savepoint stack: list of (name, tables_snapshot, indexes_snapshot).
        # Each SAVEPOINT pushes a deep-copy; RELEASE pops; ROLLBACK TO
        # restores from a snapshot but keeps the entry so it can be reused.
        self._savepoint_stack: list[tuple[str, dict[str, _Table], dict[str, IndexDef]]] = []

    # --- Construction helpers ---------------------------------------------

    @classmethod
    def from_tables(
        cls,
        tables: dict[str, tuple[list[ColumnDef], list[Row]]],
    ) -> InMemoryBackend:
        """Build a backend pre-populated with ``tables``.

        ``tables`` maps table name → (column defs, rows). Rows are inserted
        directly into the backing list *without* constraint checks — this
        is a fixture helper, not a public API. If you want constraint
        checking, call ``create_table`` + ``insert`` explicitly.
        """
        backend = cls()
        for name, (cols, rows) in tables.items():
            t = _Table(cols)
            t.rows = [dict(r) for r in rows]
            backend._tables[name] = t
        return backend

    # --- Schema -----------------------------------------------------------

    def tables(self) -> list[str]:
        return list(self._tables.keys())

    def columns(self, table: str) -> list[ColumnDef]:
        return list(self._require_table(table).columns)

    # --- Read -------------------------------------------------------------

    def scan(self, table: str) -> RowIterator:
        t = self._require_table(table)
        # Return a snapshot view — hand out shallow copies of rows so the
        # VM can mutate freely without corrupting our state. ListRowIterator
        # handles that copy on each next() call.
        return ListRowIterator(t.rows)

    def _open_cursor(self, table: str) -> ListCursor:
        """Internal helper: produce a ListCursor the VM can use for UPDATE/DELETE.

        Not on the public Backend interface, but documented here because
        tests for update/delete need a way to get a cursor. The VM's normal
        flow is: open a scan, iterate with next(), then pass the iterator
        to ``update`` or ``delete`` — so in practice the VM never needs
        this helper. Tests do.
        """
        t = self._require_table(table)
        return ListCursor(t.rows)

    # --- Write ------------------------------------------------------------

    def insert(self, table: str, row: Row) -> None:
        t = self._require_table(table)
        full_row = self._apply_defaults(t, row)
        self._check_unknown_columns(table, t, full_row)
        self._check_not_null(table, t, full_row)
        self._check_unique(table, t, full_row, ignore_index=None)
        t.rows.append(full_row)

    def update(
        self,
        table: str,
        cursor: Cursor,
        assignments: dict[str, SqlValue],
    ) -> None:
        t = self._require_table(table)
        # We require our own ListCursor for positioned DML because we need
        # to know which index to mutate. Foreign cursors (from other
        # backends) don't make sense here — a backend can only update rows
        # it owns.
        if not isinstance(cursor, ListCursor):
            raise Unsupported(operation="update with non-native cursor")
        idx = cursor.current_index()
        if idx < 0 or idx >= len(t.rows):
            raise Unsupported(operation="update without current row")

        # Validate column names *before* applying any assignment — partial
        # updates would corrupt constraint invariants.
        for col_name in assignments:
            if t.column_def(col_name) is None:
                raise ColumnNotFound(table=table, column=col_name)

        # Build the proposed new row, then re-check constraints against the
        # new values. We must ignore the row at ``idx`` during the UNIQUE
        # check — a row never conflicts with itself.
        proposed = dict(t.rows[idx])
        proposed.update(assignments)
        self._check_not_null(table, t, proposed)
        self._check_unique(table, t, proposed, ignore_index=idx)

        t.rows[idx] = proposed

    def delete(self, table: str, cursor: Cursor) -> None:
        t = self._require_table(table)
        if not isinstance(cursor, ListCursor):
            raise Unsupported(operation="delete with non-native cursor")
        idx = cursor.current_index()
        if idx < 0 or idx >= len(t.rows):
            raise Unsupported(operation="delete without current row")

        del t.rows[idx]
        # After deletion the cursor no longer has a valid current row. We
        # also shift the cursor's index back by one so that the next call
        # to next() returns what *used to be* idx+1 (now at idx after the
        # del). Without this adjustment we'd skip a row.
        cursor._idx -= 1  # noqa: SLF001 — tight coupling with ListCursor is intentional
        cursor._current = None  # noqa: SLF001

    # --- DDL --------------------------------------------------------------

    def create_table(
        self,
        table: str,
        columns: list[ColumnDef],
        if_not_exists: bool,
    ) -> None:
        if table in self._tables:
            if if_not_exists:
                return
            raise TableAlreadyExists(table=table)
        self._tables[table] = _Table(columns)

    def drop_table(self, table: str, if_exists: bool) -> None:
        if table not in self._tables:
            if if_exists:
                return
            raise TableNotFound(table=table)
        del self._tables[table]

    def add_column(self, table: str, column: ColumnDef) -> None:
        if table not in self._tables:
            raise TableNotFound(table=table)
        tbl = self._tables[table]
        if any(c.name == column.name for c in tbl.columns):
            raise ColumnAlreadyExists(table=table, column=column.name)
        tbl.columns.append(column)
        # Backfill existing rows: default value if specified, NULL otherwise.
        fill_value: SqlValue = column.default if column.has_default() else None
        for row in tbl.rows:
            row[column.name] = fill_value

    # --- Transactions -----------------------------------------------------

    def begin_transaction(self) -> TransactionHandle:
        if self._active_handle is not None:
            raise Unsupported(operation="nested transactions")
        # Deep-copy the whole table map. copy.deepcopy handles the nested
        # list-of-dicts correctly — each row dict is cloned, each value
        # inside is immutable (SqlValue is scalar) so we don't need to
        # recurse deeper.
        self._snapshot = copy.deepcopy(self._tables)
        # Also snapshot the index definitions so that create_index /
        # drop_index inside a rolled-back transaction leave no trace.
        self._index_snapshot = copy.deepcopy(self._indexes)
        handle = self._next_handle
        self._next_handle += 1
        self._active_handle = handle
        return TransactionHandle(handle)

    def commit(self, handle: TransactionHandle) -> None:
        self._require_active(handle)
        # Changes are already applied to self._tables — we just discard the
        # rollback snapshot.
        self._snapshot = None
        self._index_snapshot = None
        self._active_handle = None

    def rollback(self, handle: TransactionHandle) -> None:
        self._require_active(handle)
        # _require_active guarantees _snapshot is set whenever _active_handle is.
        assert self._snapshot is not None
        self._tables = self._snapshot
        # Restore index definitions from the snapshot.
        assert self._index_snapshot is not None
        self._indexes = self._index_snapshot
        self._snapshot = None
        self._index_snapshot = None
        self._active_handle = None

    def current_transaction(self) -> TransactionHandle | None:
        """Return the active transaction handle, or ``None`` if no transaction
        is currently open.

        Because the InMemoryBackend stores the handle internally, this method
        can bridge the gap between separate :func:`~sql_vm.vm.execute` calls:
        ``BeginTransaction`` stores the handle; subsequent ``CommitTransaction``
        / ``RollbackTransaction`` calls retrieve it here rather than relying on
        the (by then discarded) VM state object.
        """
        if self._active_handle is None:
            return None
        return TransactionHandle(self._active_handle)

    def create_savepoint(self, name: str) -> None:
        """Push a deep-copy snapshot of the current tables and indexes.

        Each SAVEPOINT call appends to the stack; multiple savepoints with the
        same name stack independently (SQLite allows this).  If a transaction
        is not already active, ``create_savepoint`` implicitly begins one so
        the savepoint has something to anchor to.

        Deep-copying is O(data size) but acceptable for the pedagogical scale
        this backend targets.
        """
        if self._active_handle is None:
            # Implicitly begin a transaction so the savepoint is anchored.
            self.begin_transaction()
        snap_tables = copy.deepcopy(self._tables)
        snap_indexes = copy.deepcopy(self._indexes)
        self._savepoint_stack.append((name, snap_tables, snap_indexes))

    def release_savepoint(self, name: str) -> None:
        """Remove the named savepoint (and all savepoints after it).

        Finds the *last* entry in the stack with the given name, removes it
        and every entry that was pushed after it.  The current table state is
        not changed — this is a "partial commit" up to the release point.

        Raises :class:`~sql_backend.errors.Unsupported` if no savepoint with
        that name exists.
        """
        idx = self._find_savepoint(name)
        if idx is None:
            raise Unsupported(operation=f"RELEASE {name!r}: no such savepoint")
        del self._savepoint_stack[idx:]

    def rollback_to_savepoint(self, name: str) -> None:
        """Restore the database to the state it was in when *name* was created.

        Finds the *last* savepoint with the given name, restores tables and
        indexes from its snapshot, and removes all savepoints pushed after it.
        The named savepoint itself is kept in the stack so the caller may roll
        back to it again or release it later.

        Raises :class:`~sql_backend.errors.Unsupported` if no savepoint with
        that name exists.
        """
        idx = self._find_savepoint(name)
        if idx is None:
            raise Unsupported(operation=f"ROLLBACK TO {name!r}: no such savepoint")
        _name, snap_tables, snap_indexes = self._savepoint_stack[idx]
        # Restore state from the snapshot.
        self._tables = copy.deepcopy(snap_tables)
        self._indexes = copy.deepcopy(snap_indexes)
        # Drop all savepoints created after this one; keep this one alive.
        del self._savepoint_stack[idx + 1:]

    def _find_savepoint(self, name: str) -> int | None:
        """Return the index of the *last* savepoint named *name*, or ``None``."""
        for i in range(len(self._savepoint_stack) - 1, -1, -1):
            if self._savepoint_stack[i][0] == name:
                return i
        return None

    # --- Triggers ----------------------------------------------------------

    def create_trigger(self, defn: TriggerDef) -> None:
        """Store a trigger definition.

        Raises :class:`TriggerAlreadyExists` if a trigger with the same name
        already exists.
        """
        if defn.name in self._triggers:
            raise TriggerAlreadyExists(name=defn.name)
        self._triggers[defn.name] = defn
        self._triggers_by_table.setdefault(defn.table, []).append(defn)

    def drop_trigger(self, name: str, if_exists: bool = False) -> None:
        """Remove a trigger definition by name.

        Raises :class:`TriggerNotFound` when *name* is absent and
        ``if_exists=False``.
        """
        if name not in self._triggers:
            if if_exists:
                return
            raise TriggerNotFound(name=name)
        defn = self._triggers.pop(name)
        table_list = self._triggers_by_table.get(defn.table, [])
        self._triggers_by_table[defn.table] = [t for t in table_list if t.name != name]

    def list_triggers(self, table: str) -> list[TriggerDef]:
        """Return all triggers for *table* in creation order."""
        return list(self._triggers_by_table.get(table, []))

    # --- Indexes ----------------------------------------------------------

    def create_index(self, index: IndexDef) -> None:
        """Store an index definition and validate it against the schema.

        The in-memory backend does not build a sorted data structure for
        the index at creation time — :meth:`scan_index` performs a linear
        scan of the table rows instead.  This is correct (though O(n)) and
        appropriate for a pedagogical reference backend.

        Raises
        ------
        IndexAlreadyExists
            If an index named ``index.name`` already exists.
        TableNotFound
            If ``index.table`` is not a known table.
        ColumnNotFound
            If any column in ``index.columns`` is not a column of
            ``index.table``.
        """
        if index.name in self._indexes:
            raise IndexAlreadyExists(index=index.name)
        t = self._require_table(index.table)
        col_names = {col.name for col in t.columns}
        for col in index.columns:
            if col not in col_names:
                raise ColumnNotFound(table=index.table, column=col)
        self._indexes[index.name] = index

    def drop_index(self, name: str, *, if_exists: bool = False) -> None:
        """Remove an index definition.

        Raises :class:`IndexNotFound` when *name* is absent and
        ``if_exists=False``.
        """
        if name not in self._indexes:
            if if_exists:
                return
            raise IndexNotFound(index=name)
        del self._indexes[name]

    def list_indexes(self, table: str | None = None) -> list[IndexDef]:
        """Return all stored index definitions, optionally filtered by table.

        Returns indexes in creation order.
        """
        if table is None:
            return list(self._indexes.values())
        return [idx for idx in self._indexes.values() if idx.table == table]

    def scan_index(
        self,
        index_name: str,
        lo: list[SqlValue] | None,
        hi: list[SqlValue] | None,
        *,
        lo_inclusive: bool = True,
        hi_inclusive: bool = True,
    ) -> Iterator[int]:
        """Yield list-indices of matching rows from the indexed table.

        For the in-memory backend the "rowid" exposed by ``scan_index`` is
        the 0-based position of the row in the table's row list — the same
        value used internally by :class:`ListCursor`.  This is consistent
        within the backend but is not comparable to the integer rowids used
        by file-backed backends.

        The scan is O(n): all rows are examined, key values are extracted
        and compared, then matching rows are yielded in ascending key order.
        This is correct for a pedagogical backend; file-backed backends do
        this in O(log n + k) via the B-tree index.

        Raises :class:`IndexNotFound` if *index_name* does not exist.
        """
        idx_def = self._indexes.get(index_name)
        if idx_def is None:
            raise IndexNotFound(index=index_name)

        t = self._require_table(idx_def.table)
        col_names = idx_def.columns

        # Build (sort_key, original_row_idx) pairs for all rows.
        # sort_key is a tuple of _sql_sort_key(v) values — one per index column.
        keyed: list[tuple[tuple[tuple[int, object], ...], int]] = []
        for i, row in enumerate(t.rows):
            key_vals = [row.get(col) for col in col_names]
            sort_key = tuple(_sql_sort_key(v) for v in key_vals)
            keyed.append((sort_key, i))

        # Sort by key — Python's tuple comparison does the right thing since
        # all elements are (int, comparable) pairs.
        keyed.sort(key=lambda kv: kv[0])

        lo_sort = tuple(_sql_sort_key(v) for v in lo) if lo is not None else None
        hi_sort = tuple(_sql_sort_key(v) for v in hi) if hi is not None else None

        for sort_key, row_idx in keyed:
            # Trim to the minimum length for partial-key comparison.
            if lo_sort is not None:
                cmp_lo = sort_key[: len(lo_sort)]
                if cmp_lo < lo_sort or (cmp_lo == lo_sort and not lo_inclusive):
                    continue
            if hi_sort is not None:
                cmp_hi = sort_key[: len(hi_sort)]
                if cmp_hi > hi_sort or (cmp_hi == hi_sort and not hi_inclusive):
                    return
            yield row_idx

    def scan_by_rowids(self, table: str, rowids: list[int]) -> RowIterator:
        """Return a RowIterator over the rows at the given list indices.

        For the in-memory backend every "rowid" is the 0-based position of a row
        in the table's internal list — exactly what :meth:`scan_index` yields.
        Rows are returned in the order the rowids are given; caller should sort
        if ascending order is required.

        Out-of-range indices are silently skipped.
        """
        t = self._require_table(table)
        rows = [t.rows[i] for i in rowids if 0 <= i < len(t.rows)]
        return ListRowIterator(rows)

    # --- Private helpers --------------------------------------------------

    def _require_table(self, table: str) -> _Table:
        t = self._tables.get(table)
        if t is None:
            raise TableNotFound(table=table)
        return t

    def _require_active(self, handle: TransactionHandle) -> None:
        if self._active_handle is None:
            raise Unsupported(operation="no active transaction")
        if int(handle) != self._active_handle:
            raise Unsupported(operation="stale transaction handle")

    def _apply_defaults(self, t: _Table, row: Row) -> Row:
        """Return ``row`` with any missing columns filled in from defaults.

        If a column is absent from the row and it has a DEFAULT, we insert
        the default value. If the column is absent and has no default, we
        leave it absent — NOT NULL / UNIQUE checks downstream will decide
        whether that's an error. (Absent columns produce NULL on read.)
        """
        out: Row = dict(row)
        for col in t.columns:
            if col.name not in out and col.has_default():
                # col.default is ColumnDefault = SqlValue | _NoDefault.
                # has_default() ruled out the sentinel, so this cast is safe.
                out[col.name] = col.default  # type: ignore[assignment]
            elif col.name not in out:
                # Missing + no default → NULL, so NOT NULL checks can see it.
                out[col.name] = None
        return out

    def _check_unknown_columns(self, table: str, t: _Table, row: Row) -> None:
        """Reject inserts that mention columns not in the schema."""
        known = {col.name for col in t.columns}
        for key in row:
            if key not in known:
                raise ColumnNotFound(table=table, column=key)

    def _check_not_null(self, table: str, t: _Table, row: Row) -> None:
        """Enforce NOT NULL (including implicit NOT NULL from PRIMARY KEY)."""
        for col in t.columns:
            if col.effective_not_null() and row.get(col.name) is None:
                raise ConstraintViolation(
                    table=table,
                    column=col.name,
                    message=f"NOT NULL constraint failed: {table}.{col.name}",
                )

    def _check_unique(
        self,
        table: str,
        t: _Table,
        row: Row,
        ignore_index: int | None,
    ) -> None:
        """Enforce UNIQUE (including implicit UNIQUE from PRIMARY KEY).

        NULL never conflicts with anything — SQL semantics. A UNIQUE column
        may contain many NULLs. ``ignore_index`` is the row being updated,
        which must not conflict with itself.
        """
        for col in t.columns:
            if not col.effective_unique():
                continue
            new_val = row.get(col.name)
            if new_val is None:
                continue
            for i, existing in enumerate(t.rows):
                if i == ignore_index:
                    continue
                if existing.get(col.name) == new_val:
                    label: Final[str] = "PRIMARY KEY" if col.primary_key else "UNIQUE"
                    raise ConstraintViolation(
                        table=table,
                        column=col.name,
                        message=f"{label} constraint failed: {table}.{col.name}",
                    )
