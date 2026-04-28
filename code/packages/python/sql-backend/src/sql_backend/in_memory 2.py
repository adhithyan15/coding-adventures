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
from typing import Final

from .backend import Backend, TransactionHandle
from .errors import (
    ColumnNotFound,
    ConstraintViolation,
    TableAlreadyExists,
    TableNotFound,
    Unsupported,
)
from .row import Cursor, ListCursor, ListRowIterator, Row, RowIterator
from .schema import ColumnDef
from .values import SqlValue


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
        # Snapshot for the currently-active transaction, if any. ``None``
        # means no transaction is open. We store a full deep copy rather
        # than a diff log because it is dramatically simpler and the data
        # volumes targeted by this backend are small.
        self._snapshot: dict[str, _Table] | None = None
        # We hand out handles as monotonically increasing integers. Reusing
        # handles across transactions would make stale-handle bugs silent;
        # this way an old handle will never match a new transaction.
        self._next_handle: int = 1
        self._active_handle: int | None = None

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

    # --- Transactions -----------------------------------------------------

    def begin_transaction(self) -> TransactionHandle:
        if self._active_handle is not None:
            raise Unsupported(operation="nested transactions")
        # Deep-copy the whole table map. copy.deepcopy handles the nested
        # list-of-dicts correctly — each row dict is cloned, each value
        # inside is immutable (SqlValue is scalar) so we don't need to
        # recurse deeper.
        self._snapshot = copy.deepcopy(self._tables)
        handle = self._next_handle
        self._next_handle += 1
        self._active_handle = handle
        return TransactionHandle(handle)

    def commit(self, handle: TransactionHandle) -> None:
        self._require_active(handle)
        # Changes are already applied to self._tables — we just discard the
        # rollback snapshot.
        self._snapshot = None
        self._active_handle = None

    def rollback(self, handle: TransactionHandle) -> None:
        self._require_active(handle)
        # _require_active guarantees _snapshot is set whenever _active_handle is.
        assert self._snapshot is not None
        self._tables = self._snapshot
        self._snapshot = None
        self._active_handle = None

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
