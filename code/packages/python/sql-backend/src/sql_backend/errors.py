"""
Backend error hierarchy
=======================

Every backend — in-memory, CSV, file-on-disk, SQLite, HTTP — translates its
native failure modes into one of the six errors defined here. The VM only ever
sees :class:`BackendError`. That boundary is what lets us swap backends without
touching the VM or the facade layer.

Why a base class and not six unrelated exceptions?
--------------------------------------------------

Two reasons:

1. The facade (``mini-sqlite``) catches ``BackendError`` to translate it into
   the PEP 249 exception hierarchy. One ``except`` clause, six subclasses —
   instead of six ``except`` clauses that would need to be edited every time
   we add a new variant.

2. Tests can assert on the *category* of failure (was this a constraint
   violation? any kind?) or on the *specific* variant (was it UNIQUE or
   NOT NULL?). Both styles read naturally with ``isinstance``.

Why dataclasses?
----------------

Dataclasses give us ``__eq__`` for free — two ``TableNotFound("users")``
instances compare equal — which makes conformance tests pithy::

    with pytest.raises(TableNotFound) as exc_info:
        backend.columns("missing")
    assert exc_info.value == TableNotFound(table="missing")

They also give us a readable ``repr`` without boilerplate, which is what
shows up in failed assertions.
"""

from __future__ import annotations

from dataclasses import dataclass


class BackendError(Exception):
    """Base class for all backend-reported failures.

    Backends must not raise any other exception type for expected failure
    modes. Raising something else is a bug — the VM does not catch it, so it
    propagates all the way up to application code as an opaque crash.
    """


@dataclass(eq=True)
class TableNotFound(BackendError):
    """Raised when an operation references a table that does not exist."""

    table: str

    def __str__(self) -> str:
        return f"table not found: {self.table!r}"


@dataclass(eq=True)
class TableAlreadyExists(BackendError):
    """Raised by ``create_table(if_not_exists=False)`` when the table exists."""

    table: str

    def __str__(self) -> str:
        return f"table already exists: {self.table!r}"


@dataclass(eq=True)
class ColumnNotFound(BackendError):
    """Raised when an operation references a column that does not exist."""

    table: str
    column: str

    def __str__(self) -> str:
        return f"column not found: {self.table!r}.{self.column!r}"


@dataclass(eq=True)
class ConstraintViolation(BackendError):
    """Raised for NOT NULL / UNIQUE / PRIMARY KEY violations.

    The ``message`` field carries a human-readable explanation so the facade
    can surface it without reformatting. Typical messages::

        "NOT NULL constraint failed: users.name"
        "UNIQUE constraint failed: users.email"
        "PRIMARY KEY constraint failed: users.id"
    """

    table: str
    column: str
    message: str

    def __str__(self) -> str:
        return self.message


@dataclass(eq=True)
class Unsupported(BackendError):
    """Raised when a backend does not implement an optional operation.

    Read-only backends (CSV, HTTP) raise this from ``insert``/``update``/
    ``delete``. Backends with no transaction support raise it from the three
    transaction methods. The VM surfaces the error unchanged so application
    code sees an honest failure instead of a silent no-op.
    """

    operation: str

    def __str__(self) -> str:
        return f"operation not supported: {self.operation}"


@dataclass(eq=True)
class Internal(BackendError):
    """Escape hatch for backend bugs or unexpected I/O failures.

    Use sparingly. If you find yourself reaching for this, ask whether the
    failure deserves its own variant instead — the goal is that the VM and
    facade can make useful decisions based on the error type, which they
    can't do for a generic ``Internal``.
    """

    message: str

    def __str__(self) -> str:
        return self.message


@dataclass(eq=True)
class IndexAlreadyExists(BackendError):
    """Raised by ``create_index`` when an index with the same name already exists.

    Like :class:`TableAlreadyExists`, this is a *schema conflict* error — the
    caller asked for something the backend already has.  The ``index`` field
    holds the duplicate name so the error message and test assertions are
    self-explanatory::

        with pytest.raises(IndexAlreadyExists) as exc_info:
            backend.create_index(IndexDef(name="idx_users_email", ...))
        assert exc_info.value == IndexAlreadyExists(index="idx_users_email")
    """

    index: str

    def __str__(self) -> str:
        return f"index already exists: {self.index!r}"


@dataclass(eq=True)
class IndexNotFound(BackendError):
    """Raised when an operation references an index that does not exist.

    Symmetric to :class:`TableNotFound`.  Raised by ``drop_index`` (when
    ``if_exists=False``) and by ``scan_index`` when the named index is
    absent::

        with pytest.raises(IndexNotFound) as exc_info:
            backend.drop_index("nonexistent")
        assert exc_info.value == IndexNotFound(index="nonexistent")
    """

    index: str

    def __str__(self) -> str:
        return f"index not found: {self.index!r}"
