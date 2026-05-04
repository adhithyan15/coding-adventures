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
from collections.abc import Mapping, Sequence
from types import TracebackType
from typing import Any

from sql_backend import Backend, InMemoryBackend, TransactionHandle
from storage_sqlite import SqliteFileBackend

from .advisor import IndexAdvisor
from .cursor import Cursor
from .errors import OperationalError, ProgrammingError, translate
from .policy import IndexPolicy

# Statements which, under sqlite3 semantics, implicitly commit the
# current transaction and run outside of any transaction. Keeping the
# list small and explicit avoids surprising users — if a statement is
# not in this set, it participates in the current transaction.
_DDL_KEYWORDS = frozenset(["CREATE", "DROP", "ALTER"])

# Explicit transaction-control keywords.  Statements that begin with one
# of these are intercepted at the cursor level and handled directly by the
# connection — they never pass through the engine/VM.  This avoids a
# conflict between the connection's implicit-transaction management and
# the VM's own transaction instructions.
_TCL_KEYWORDS = frozenset(["BEGIN", "COMMIT", "ROLLBACK"])


def _first_keyword(sql: str) -> str:
    """Extract the first identifier (uppercase) from a SQL string.

    Skips leading whitespace and both ``--`` line comments and ``/* */``
    block comments before extracting the word.  Used to sniff whether a
    statement is DDL or a transaction-control statement (TCL) so the
    connection can handle it specially.
    """
    i = 0
    n = len(sql)
    while i < n:
        ch = sql[i]
        if ch.isspace():
            i += 1
            continue
        if ch == "-" and i + 1 < n and sql[i + 1] == "-":
            # Line comment — skip to end of line.
            while i < n and sql[i] != "\n":
                i += 1
            continue
        if ch == "/" and i + 1 < n and sql[i + 1] == "*":
            # Block comment — skip to matching close.
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
    return "".join(word).upper()


def _is_ddl(sql: str) -> bool:
    """Return True when the first keyword is a DDL keyword (CREATE/DROP/ALTER).

    A wrong answer here is not catastrophic: it just means the user sees
    a slightly different transaction boundary than sqlite3 would draw.
    """
    return _first_keyword(sql) in _DDL_KEYWORDS


def _is_tcl(sql: str) -> bool:
    """Return True when the first keyword is a TCL keyword (BEGIN/COMMIT/ROLLBACK).

    TCL statements are handled entirely by the connection, never by the VM.
    """
    return _first_keyword(sql) in _TCL_KEYWORDS


class Connection:
    """PEP 249 Connection object.

    Create via :func:`mini_sqlite.connect`. Owns a backend and a
    transaction lifecycle.
    """

    def __init__(
        self,
        backend: Backend,
        *,
        autocommit: bool = False,
        auto_index: bool = True,
    ) -> None:
        self._backend = backend
        self._autocommit = autocommit
        self._txn: TransactionHandle | None = None
        self._closed = False
        # True when _txn was opened for a DDL statement and should be
        # committed immediately after the statement runs.
        self._ddl_txn: bool = False
        # Optional index advisor — created by default, may be disabled via
        # auto_index=False or replaced via set_policy().
        self._advisor: IndexAdvisor | None = (
            IndexAdvisor(backend) if auto_index else None
        )
        # CHECK constraint registry persisted across execute() calls.
        # Populated by CREATE TABLE statements, consulted on INSERT/UPDATE.
        self._check_registry: dict = {}
        # FOREIGN KEY registries — child (forward) and parent (reverse).
        self._fk_child: dict = {}
        self._fk_parent: dict = {}
        # View definitions: name → SelectStmt. Populated by CREATE VIEW,
        # removed by DROP VIEW, and threaded through the adapter so that bare
        # view names in FROM/JOIN are expanded to DerivedTableRef at parse time.
        self._view_defs: dict = {}
        # Savepoint name stack. Kept in sync with backend.create_savepoint /
        # release_savepoint / rollback_to_savepoint by engine.run().
        # Cleared when the enclosing transaction commits or rolls back.
        self._savepoints: list[str] = []
        # User-defined scalar functions: lower-cased name → (nargs, callable).
        # nargs=-1 means variadic.  Registered via create_function().
        self._user_functions: dict[str, tuple[int, Any]] = {}

    # ------------------------------------------------------------------
    # Cursor + shortcut methods.
    # ------------------------------------------------------------------

    def cursor(self) -> Cursor:
        self._assert_open()
        return Cursor(self)

    def execute(
        self,
        sql: str,
        parameters: Sequence[Any] | Mapping[str, Any] = (),
    ) -> Cursor:
        """Shortcut: create a cursor, run ``execute``, return the cursor.

        Non-standard but universally expected — sqlite3 exposes it too.

        Accepts either a positional ``Sequence`` (qmark style, ``?``
        placeholders) or a ``Mapping`` (named style, ``:identifier``
        placeholders).  See :meth:`Cursor.execute` for details.
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
            self._savepoints.clear()

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
            self._savepoints.clear()

    # ------------------------------------------------------------------
    # Explicit TCL — called by Cursor when it detects BEGIN/COMMIT/ROLLBACK
    # ------------------------------------------------------------------

    def _tcl_begin(self) -> None:
        """Handle an explicit ``BEGIN [TRANSACTION]`` statement.

        Unlike the implicit transaction opened by :meth:`_ensure_transaction_if_needed`,
        an explicit BEGIN is an error if a transaction is already active (the
        DB-API does not support nested transactions without SAVEPOINT).

        If there is already an implicit DML transaction open we commit it
        first so the user's explicit BEGIN starts with a clean slate — this
        matches SQLite's behaviour.
        """
        self._assert_open()
        if self._txn is not None:
            # A transaction is already open — explicit BEGIN is a no-nested
            # rule violation.
            raise OperationalError(
                "cannot BEGIN: a transaction is already active"
            )
        try:
            self._txn = self._backend.begin_transaction()
            self._ddl_txn = False  # explicit; do NOT auto-commit it
        except Exception as e:  # noqa: BLE001
            raise translate(e) from e

    def _tcl_commit(self) -> None:
        """Handle an explicit ``COMMIT [TRANSACTION]`` statement.

        Raises :exc:`OperationalError` when there is no active transaction —
        committing without a prior BEGIN is a programming error.
        """
        self._assert_open()
        if self._txn is None:
            raise OperationalError(
                "cannot COMMIT: no active transaction"
            )
        try:
            self._backend.commit(self._txn)
        except Exception as e:  # noqa: BLE001
            raise translate(e) from e
        finally:
            self._txn = None
            self._savepoints.clear()

    def _tcl_rollback(self) -> None:
        """Handle an explicit ``ROLLBACK [TRANSACTION]`` statement.

        Raises :exc:`OperationalError` when there is no active transaction —
        rolling back without a prior BEGIN is a programming error.
        """
        self._assert_open()
        if self._txn is None:
            raise OperationalError(
                "cannot ROLLBACK: no active transaction"
            )
        try:
            self._backend.rollback(self._txn)
        except Exception as e:  # noqa: BLE001
            raise translate(e) from e
        finally:
            self._txn = None
            self._savepoints.clear()

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

    def set_policy(self, policy: IndexPolicy) -> None:
        """Replace the index-creation policy on the live advisor.

        If ``auto_index=False`` was passed at connection time the advisor is
        ``None`` and calling ``set_policy`` is a no-op (no advisor to
        configure).  Create the connection with ``auto_index=True`` — the
        default — if you want to use a custom policy.

        Hit counts accumulated by the previous policy are preserved; only the
        decision logic changes.

        Example::

            conn = mini_sqlite.connect(":memory:")
            conn.set_policy(HitCountPolicy(threshold=5))
        """
        if self._advisor is not None:
            self._advisor.policy = policy

    def create_function(self, name: str, nargs: int, fn: Any) -> None:
        """Register a user-defined scalar function.

        After registration, ``name(...)`` is callable from any SQL statement
        executed on this connection, exactly like a built-in function.

        Parameters
        ----------
        name:
            SQL function name (case-insensitive; stored in lower-case).
        nargs:
            Expected argument count.  Pass ``-1`` for a variadic function
            that accepts any number of arguments.
        fn:
            A Python callable.  It receives SQL values as positional
            arguments and must return a value of a type recognised by the
            backend (``int``, ``float``, ``str``, ``bytes``, ``bool``, or
            ``None``).

        Examples::

            conn.create_function("double", 1, lambda x: x * 2 if x is not None else None)
            conn.create_function("add3", 3, lambda a, b, c: a + b + c)
        """
        self._assert_open()
        self._user_functions[name.lower()] = (nargs, fn)

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


def connect(
    database: str,
    *,
    autocommit: bool = False,
    auto_index: bool = True,
) -> Connection:
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

    ``auto_index`` (default ``True``):

    When ``True``, an :class:`~mini_sqlite.advisor.IndexAdvisor` is attached
    to the connection.  It observes every query plan and automatically creates
    B-tree indexes for columns that appear repeatedly in ``WHERE`` predicates.
    The default policy (:class:`~mini_sqlite.policy.HitCountPolicy`) creates
    an index after a column has been filtered three times.

    Set to ``False`` to disable automatic index management entirely.  You can
    also call :meth:`Connection.set_policy` at any time to swap in a custom
    policy.
    """
    if database == ":memory:":
        return Connection(InMemoryBackend(), autocommit=autocommit, auto_index=auto_index)
    return Connection(SqliteFileBackend(database), autocommit=autocommit, auto_index=auto_index)
