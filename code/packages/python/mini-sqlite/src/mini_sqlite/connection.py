"""
PEP 249 Connection.

The Connection owns:

- the backend (in-memory today, file-backed later),
- transaction state: whether a transaction is currently open and the
  associated :class:`TransactionHandle` from the backend,
- the autocommit flag.

PEP 249 semantics (without autocommit, the default): a transaction is
*implicitly* begun when the first data-modifying statement runs after
a fresh connection or after a preceding ``commit``/``rollback``. The
user calls ``commit()`` or ``rollback()`` to end it. DDL is subtle —
sqlite3 implicitly commits before and after a DDL statement; we match
that behavior in v1 by treating DDL as autocommitted, which keeps the
semantics simple and predictable.

``autocommit=True`` mode is offered as an escape hatch: every statement
commits immediately. There is no implicit transaction to start or end.

Context-manager protocol: ``__exit__`` commits on success and rolls
back on exception — matching sqlite3.
"""

from __future__ import annotations

import contextlib
from collections.abc import Sequence
from types import TracebackType
from typing import Any

from sql_backend import Backend, InMemoryBackend, TransactionHandle
from storage_sqlite import SqliteFileBackend

from .cursor import Cursor
from .errors import ProgrammingError, translate

# Statements which, under sqlite3 semantics, implicitly commit the
# current transaction and run outside of any transaction. Keeping the
# list small and explicit avoids surprising users — if a statement is
# not in this set, it participates in the current transaction.
_DDL_KEYWORDS = ("CREATE", "DROP", "ALTER")


def _is_ddl(sql: str) -> bool:
    """Cheap first-token sniff to decide if a statement is DDL.

    We skip leading whitespace and comments, then look at the first word.
    A wrong answer here is not catastrophic: it just means the user sees
    a slightly different transaction boundary than sqlite3 would draw.
    """
    i = 0
    n = len(sql)
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
    word = []
    while i < n and sql[i].isalpha():
        word.append(sql[i])
        i += 1
    return "".join(word).upper() in _DDL_KEYWORDS


class Connection:
    """PEP 249 Connection object.

    Create via :func:`mini_sqlite.connect`. Owns a backend and a
    transaction lifecycle.
    """

    def __init__(self, backend: Backend, *, autocommit: bool = False) -> None:
        self._backend = backend
        self._autocommit = autocommit
        self._txn: TransactionHandle | None = None
        self._closed = False
        # True when _txn was opened for a DDL statement and should be
        # committed immediately after the statement runs.
        self._ddl_txn: bool = False

    # ------------------------------------------------------------------
    # Cursor + shortcut methods.
    # ------------------------------------------------------------------

    def cursor(self) -> Cursor:
        self._assert_open()
        return Cursor(self)

    def execute(self, sql: str, parameters: Sequence[Any] = ()) -> Cursor:
        """Shortcut: create a cursor, run ``execute``, return the cursor.

        Non-standard but universally expected — sqlite3 exposes it too.
        """
        cur = self.cursor()
        return cur.execute(sql, parameters)

    def executemany(self, sql: str, seq_of_parameters: Any) -> Cursor:
        cur = self.cursor()
        return cur.executemany(sql, seq_of_parameters)

    # ------------------------------------------------------------------
    # Transaction control.
    # ------------------------------------------------------------------

    def commit(self) -> None:
        self._assert_open()
        if self._txn is None:
            return  # nothing open — commit is a no-op
        try:
            self._backend.commit(self._txn)
        except Exception as e:  # noqa: BLE001
            raise translate(e) from e
        finally:
            self._txn = None

    def rollback(self) -> None:
        self._assert_open()
        if self._txn is None:
            return
        try:
            self._backend.rollback(self._txn)
        except Exception as e:  # noqa: BLE001
            raise translate(e) from e
        finally:
            self._txn = None

    # ------------------------------------------------------------------
    # Lifecycle.
    # ------------------------------------------------------------------

    def close(self) -> None:
        if self._closed:
            return
        if self._txn is not None:
            # PEP 249: closing with an open transaction should roll back.
            # Best-effort: if the backend is itself in a broken state,
            # we swallow so close() stays idempotent.
            with contextlib.suppress(Exception):
                self._backend.rollback(self._txn)
            self._txn = None
        self._closed = True

    def __enter__(self) -> Connection:
        self._assert_open()
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> None:
        if exc_type is None:
            self.commit()
        else:
            self.rollback()

    # ------------------------------------------------------------------
    # Called by Cursor on every execute to keep transaction state honest.
    # ------------------------------------------------------------------

    def _ensure_transaction_if_needed(self, sql: str) -> None:
        """Begin an implicit transaction if this DML needs one.

        Called from :meth:`Cursor.execute` before each statement.

        DDL semantics (matching sqlite3): any open DML transaction is
        committed first, then the DDL runs inside its own single-statement
        transaction.  That transaction is committed immediately after the
        statement completes by :meth:`_post_execute`.  This guarantees that
        DDL changes are persisted to disk regardless of whether any DML
        follows in the same session.
        """
        if self._autocommit:
            return
        if _is_ddl(sql):
            # DDL: autocommit mode for this statement. If a transaction is
            # already open we commit it first so the DDL is not rolled in.
            if self._txn is not None:
                try:
                    self._backend.commit(self._txn)
                except Exception as e:  # noqa: BLE001
                    raise translate(e) from e
                finally:
                    self._txn = None
            # Open a fresh transaction to wrap the DDL itself.  It will be
            # committed immediately after the statement runs (_post_execute).
            try:
                self._txn = self._backend.begin_transaction()
                self._ddl_txn = True
            except Exception as e:  # noqa: BLE001
                raise translate(e) from e
            return
        self._ddl_txn = False
        if self._txn is None:
            try:
                self._txn = self._backend.begin_transaction()
            except Exception as e:  # noqa: BLE001
                raise translate(e) from e

    def _post_execute(self) -> None:
        """Auto-commit a DDL transaction immediately after the statement runs.

        Called by :meth:`Cursor.execute` after every statement.  For non-DDL
        statements this is a no-op.  For DDL statements it commits the
        single-statement transaction that :meth:`_ensure_transaction_if_needed`
        opened, so the schema change is persisted to disk right away.
        """
        if not self._ddl_txn or self._txn is None:
            return
        try:
            self._backend.commit(self._txn)
        except Exception as e:  # noqa: BLE001
            raise translate(e) from e
        finally:
            self._txn = None
            self._ddl_txn = False

    # ------------------------------------------------------------------
    # Internals.
    # ------------------------------------------------------------------

    def _assert_open(self) -> None:
        if self._closed:
            raise ProgrammingError("connection is closed")


def connect(database: str, *, autocommit: bool = False) -> Connection:
    """Open a new :class:`Connection`.

    ``database``:

    - ``":memory:"`` — the in-memory backend (no persistence). Every
      connection gets its own fresh database; nothing is written to disk.
    - any other string — interpreted as a filesystem path to a SQLite
      ``.db`` file. The file is created if it does not exist. Files
      written here are byte-compatible with the real ``sqlite3`` CLI
      and Python's built-in ``sqlite3`` module.

    Both modes return a :class:`Connection` that implements the same
    PEP 249 DB-API 2.0 interface; only the persistence semantics differ.
    """
    if database == ":memory:":
        return Connection(InMemoryBackend(), autocommit=autocommit)
    return Connection(SqliteFileBackend(database), autocommit=autocommit)
