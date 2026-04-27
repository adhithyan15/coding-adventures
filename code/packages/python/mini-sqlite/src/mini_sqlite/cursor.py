"""
PEP 249 Cursor.

A Cursor is the object through which SQL is actually executed and rows
are read. It carries the transient state of a single in-flight statement:

- ``description``: seven-tuple per output column (PEP 249 layout).
- ``rowcount``: number of rows produced / affected by the last execute.
- ``arraysize``: default batch size for fetchmany().
- a buffered iterator over the last result's rows.

The cursor does *not* own transaction state — that lives on the
Connection. ``commit`` and ``rollback`` are not cursor methods.
"""

from __future__ import annotations

from collections.abc import Iterable, Iterator, Sequence
from typing import TYPE_CHECKING, Any

from .engine import run
from .errors import ProgrammingError

# Transaction-control keywords — these must be handled at the connection
# level (not by the VM) so the connection's transaction handle stays in
# sync.  Keep this set consistent with connection._TCL_KEYWORDS.
_TCL_KEYWORDS = frozenset(["BEGIN", "COMMIT", "ROLLBACK"])


def _tcl_keyword(sql: str) -> str | None:
    """Return the first keyword of a TCL statement, or None.

    Parses the first non-whitespace, non-comment token from *sql* and
    returns it (uppercase) if it is a transaction-control keyword
    (BEGIN/COMMIT/ROLLBACK), otherwise returns ``None``.

    This mirrors the comment-skipping logic in
    :func:`~mini_sqlite.connection._first_keyword` so the two always
    agree — keep them in sync if the logic changes.
    """
    i = 0
    n = len(sql)
    # Skip whitespace and comments.
    while i < n:
        ch = sql[i]
        if ch.isspace():
            i += 1
            continue
        if ch == "-" and i + 1 < n and sql[i + 1] == "-":
            while i < n and sql[i] != "\n":
                i += 1
            continue
        if ch == "/" and i + 1 < n and sql[i + 1] == "*":
            i += 2
            while i + 1 < n and not (sql[i] == "*" and sql[i + 1] == "/"):
                i += 1
            i = min(i + 2, n)
            continue
        break
    word: list[str] = []
    while i < n and sql[i].isalpha():
        word.append(sql[i])
        i += 1
    kw = "".join(word).upper()
    return kw if kw in _TCL_KEYWORDS else None

if TYPE_CHECKING:
    from .connection import Connection


def _coerce_value(v: Any) -> Any:
    """Match sqlite3's boolean → int convention on output.

    The planner/VM preserve ``True``/``False`` end to end, but sqlite3
    returns them as ``1``/``0``. We do the same so test code written
    against sqlite3 keeps passing.
    """
    if v is True:
        return 1
    if v is False:
        return 0
    return v


def _coerce_row(row: Sequence[Any]) -> tuple[Any, ...]:
    return tuple(_coerce_value(v) for v in row)


class Cursor:
    """PEP 249 Cursor object.

    Obtained via :meth:`Connection.cursor`. Not thread-safe — PEP 249
    threadsafety level 1 means connections may be shared between threads
    but cursors must not.
    """

    def __init__(self, connection: Connection) -> None:
        self._connection = connection
        self._closed = False
        self._rows: list[tuple[Any, ...]] = []
        self._row_iter: Iterator[tuple[Any, ...]] = iter(())
        self.description: tuple[tuple[str, None, None, None, None, None, None], ...] | None = None
        self.rowcount: int = -1
        self.arraysize: int = 1
        self.lastrowid: int | None = None

    # ------------------------------------------------------------------
    # Execute.
    # ------------------------------------------------------------------

    def execute(self, sql: str, parameters: Sequence[Any] = ()) -> Cursor:
        """Execute a single SQL statement. Returns ``self`` for chaining.

        TCL fast-path
        ~~~~~~~~~~~~~
        ``BEGIN``, ``COMMIT``, and ``ROLLBACK`` (with optional ``TRANSACTION``
        suffix) are intercepted *before* the engine is invoked.  They are
        delegated to the connection's ``_tcl_*`` methods so the connection's
        ``_txn`` handle stays in sync with the backend.

        Without this fast-path the connection would open an implicit DML
        transaction just before the VM tries to open an explicit one, causing
        a "transaction already active" error on the very first ``BEGIN``.
        """
        self._assert_open()
        tcl = _tcl_keyword(sql)
        if tcl is not None:
            # Dispatch to the connection's TCL methods — no engine involved.
            if tcl == "BEGIN":
                self._connection._tcl_begin()  # noqa: SLF001
            elif tcl == "COMMIT":
                self._connection._tcl_commit()  # noqa: SLF001
            else:  # ROLLBACK
                self._connection._tcl_rollback()  # noqa: SLF001
            # TCL statements produce no result set.
            self.description = None
            self._rows = []
            self._row_iter = iter(())
            self.rowcount = -1
            return self

        self._connection._ensure_transaction_if_needed(sql)  # noqa: SLF001

        result = run(  # noqa: SLF001
            self._connection._backend,
            sql,
            parameters,
            advisor=self._connection._advisor,
        )

        # For DDL (CREATE/DROP/ALTER), auto-commit the single-statement
        # transaction that _ensure_transaction_if_needed opened.  This
        # ensures schema changes are persisted even when no DML follows.
        self._connection._post_execute()  # noqa: SLF001

        if result.columns:
            self.description = tuple(
                (name, None, None, None, None, None, None) for name in result.columns
            )
            self._rows = [_coerce_row(r) for r in result.rows]
            self._row_iter = iter(self._rows)
            self.rowcount = len(self._rows)
        else:
            # DML/DDL — no result set.
            self.description = None
            self._rows = []
            self._row_iter = iter(())
            self.rowcount = result.rows_affected if result.rows_affected is not None else -1
        return self

    def executemany(self, sql: str, seq_of_parameters: Iterable[Sequence[Any]]) -> Cursor:
        """Run the same SQL once for each parameter row.

        Per PEP 249, ``executemany`` is not required to expose a result set
        — we accumulate ``rowcount`` across iterations and clear any prior
        result so ``fetchone`` on this cursor after ``executemany`` returns
        nothing.
        """
        self._assert_open()
        total = 0
        for params in seq_of_parameters:
            self.execute(sql, params)
            total += max(self.rowcount, 0)
        self.rowcount = total
        self.description = None
        self._rows = []
        self._row_iter = iter(())
        return self

    # ------------------------------------------------------------------
    # Fetch.
    # ------------------------------------------------------------------

    def fetchone(self) -> tuple[Any, ...] | None:
        self._assert_open()
        return next(self._row_iter, None)

    def fetchmany(self, size: int | None = None) -> list[tuple[Any, ...]]:
        self._assert_open()
        n = self.arraysize if size is None else size
        out: list[tuple[Any, ...]] = []
        for _ in range(n):
            row = next(self._row_iter, None)
            if row is None:
                break
            out.append(row)
        return out

    def fetchall(self) -> list[tuple[Any, ...]]:
        self._assert_open()
        return list(self._row_iter)

    # ------------------------------------------------------------------
    # Iteration protocol — ``for row in cursor``.
    # ------------------------------------------------------------------

    def __iter__(self) -> Cursor:
        return self

    def __next__(self) -> tuple[Any, ...]:
        self._assert_open()
        row = next(self._row_iter, None)
        if row is None:
            raise StopIteration
        return row

    # ------------------------------------------------------------------
    # PEP 249 methods we don't meaningfully implement but must expose.
    # ------------------------------------------------------------------

    def setinputsizes(self, sizes: Sequence[Any]) -> None:
        """No-op — PEP 249 requires this method, not its effect."""
        self._assert_open()

    def setoutputsize(self, size: int, column: int | None = None) -> None:
        """No-op — PEP 249 requires this method, not its effect."""
        self._assert_open()

    def close(self) -> None:
        self._closed = True
        self._rows = []
        self._row_iter = iter(())

    # ------------------------------------------------------------------
    # Internals.
    # ------------------------------------------------------------------

    def _assert_open(self) -> None:
        if self._closed:
            raise ProgrammingError("cursor is closed")
        if self._connection._closed:  # noqa: SLF001
            raise ProgrammingError("connection is closed")
