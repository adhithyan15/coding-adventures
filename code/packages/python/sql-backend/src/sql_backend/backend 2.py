"""
The Backend interface
=====================

:class:`Backend` is the pluggable interface every data source implements. The
VM holds a single ``Backend`` reference and calls into it for every operation
that touches actual data — schema lookup, scans, inserts, updates, deletes,
DDL, and transactions. Everything above the backend (planner, optimizer,
codegen, VM) works purely in terms of this interface, which is what allows
us to swap in a CSV backend, a SQLite backend, or a remote HTTP backend
without touching the rest of the pipeline.

The interface is deliberately shaped like the 80/20 subset of a real database
engine:

- **Read:** ``tables``, ``columns``, ``scan`` — sufficient to execute
  SELECT against any data source.
- **Write:** ``insert``, ``update``, ``delete`` — take a cursor for
  positioned DML rather than a WHERE-clause evaluator, because the VM
  already knows how to evaluate WHERE; the backend's job is just to
  apply the change to the row the cursor is sitting on.
- **DDL:** ``create_table``, ``drop_table`` — the two DDL operations
  every real application needs. ALTER TABLE is deliberately omitted; it
  can be added later as an optional operation.
- **Transactions:** ``begin_transaction``, ``commit``, ``rollback`` —
  optional. Read-only backends return :class:`Unsupported`.

Optional operations
-------------------

We follow the same pattern as PostgreSQL FDWs and SQLite virtual tables:
any operation a backend can't support raises :class:`Unsupported` with a
descriptive ``operation`` string. The VM surfaces this unchanged; the facade
translates it into ``NotSupportedError`` (PEP 249). Application code sees a
clear, typed failure — never a silent no-op.

Why an ABC and not a Protocol?
------------------------------

Both would work. We use ``ABC`` for three reasons:

1. ``ABC`` raises ``TypeError`` at instantiation time if a subclass forgets
   a method. Protocols only catch that at type-check time, which is
   optional in Python. An explicit base class gives us a runtime safety
   net.

2. Conformance test helpers (see :mod:`sql_backend.conformance`) can
   ``isinstance()``-check against the base class. Protocols can do that too
   via ``@runtime_checkable``, but at a higher cost and without exactness
   guarantees.

3. Readable error messages: "Can't instantiate abstract class FooBackend
   with abstract methods insert, update" is exactly what a backend author
   wants to see.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import NewType

from .row import Cursor, Row, RowIterator
from .schema import ColumnDef
from .values import SqlValue

# Transactions are represented as opaque tokens — an integer handle. Backends
# that support transactions issue a fresh handle on ``begin_transaction`` and
# validate it on ``commit``/``rollback``. Using ``NewType`` gives us type
# safety (``TransactionHandle`` isn't interchangeable with a stray ``int``)
# without runtime cost.
TransactionHandle = NewType("TransactionHandle", int)


class Backend(ABC):
    """Abstract base class for all backends.

    See module docstring for the design rationale. Subclass this and
    implement every abstract method; raise :class:`Unsupported` from any
    operation you can't support.
    """

    # --- Schema ------------------------------------------------------------

    @abstractmethod
    def tables(self) -> list[str]:
        """Return the list of table names known to this backend.

        Order is backend-defined. The planner treats this as a set for name
        resolution; tests should not assume insertion order unless the
        specific backend documents it.
        """

    @abstractmethod
    def columns(self, table: str) -> list[ColumnDef]:
        """Return the columns of ``table`` in declaration order.

        Raises :class:`TableNotFound` if ``table`` is not known.
        """

    # --- Read --------------------------------------------------------------

    @abstractmethod
    def scan(self, table: str) -> RowIterator:
        """Return a RowIterator over all rows in ``table``.

        The VM typically scans to completion or to a LIMIT. Either way it
        calls ``close()`` on the iterator when done.

        Raises :class:`TableNotFound` if ``table`` is not known.
        """

    # --- Write -------------------------------------------------------------

    @abstractmethod
    def insert(self, table: str, row: Row) -> None:
        """Insert ``row`` into ``table``.

        Constraint enforcement (NOT NULL / UNIQUE / PRIMARY KEY) is the
        backend's responsibility — the VM does not pre-check. Missing
        columns are filled with their ``default`` if one is defined, else
        NULL.

        Raises :class:`TableNotFound` or :class:`ConstraintViolation`.
        """

    @abstractmethod
    def update(
        self,
        table: str,
        cursor: Cursor,
        assignments: dict[str, SqlValue],
    ) -> None:
        """Apply ``assignments`` to the row ``cursor`` is currently on.

        The cursor must have been obtained from an earlier operation on the
        same backend (typically a scan that the VM iterated). Constraint
        re-checks (NOT NULL / UNIQUE) apply to the updated values.

        Raises :class:`TableNotFound`, :class:`ColumnNotFound`, or
        :class:`ConstraintViolation`.
        """

    @abstractmethod
    def delete(self, table: str, cursor: Cursor) -> None:
        """Delete the row ``cursor`` is currently on.

        After ``delete``, the cursor's ``current_row()`` returns ``None``
        (there is no current row — we just removed it). The cursor may still
        be advanced with ``next()`` to continue iteration.
        """

    # --- DDL ---------------------------------------------------------------

    @abstractmethod
    def create_table(
        self,
        table: str,
        columns: list[ColumnDef],
        if_not_exists: bool,
    ) -> None:
        """Create a new table.

        If ``if_not_exists`` is True and the table already exists, this is a
        no-op. Otherwise, raise :class:`TableAlreadyExists`.
        """

    @abstractmethod
    def drop_table(self, table: str, if_exists: bool) -> None:
        """Drop an existing table.

        If ``if_exists`` is True and the table does not exist, this is a
        no-op. Otherwise, raise :class:`TableNotFound`.
        """

    # --- Transactions ------------------------------------------------------

    @abstractmethod
    def begin_transaction(self) -> TransactionHandle:
        """Begin a transaction. Return a handle to identify it later.

        Read-only or simple backends raise :class:`Unsupported` here.
        """

    @abstractmethod
    def commit(self, handle: TransactionHandle) -> None:
        """Commit the transaction identified by ``handle``."""

    @abstractmethod
    def rollback(self, handle: TransactionHandle) -> None:
        """Roll back the transaction identified by ``handle``."""


class SchemaProvider(ABC):
    """Minimal schema interface consumed by the planner.

    The planner uses this to resolve column references and detect ambiguity.
    Any full :class:`Backend` satisfies it via its ``columns`` method — the
    helper :func:`backend_as_schema_provider` wraps a Backend into a
    SchemaProvider for planner tests that don't want a full backend.
    """

    @abstractmethod
    def columns(self, table: str) -> list[str]:
        """Return the column *names* of ``table`` (not full ColumnDefs)."""


class _BackendSchemaProvider(SchemaProvider):
    """Adapter: expose a Backend as a SchemaProvider.

    Returns just the column names. Private — callers use
    :func:`backend_as_schema_provider`.
    """

    def __init__(self, backend: Backend) -> None:
        self._backend = backend

    def columns(self, table: str) -> list[str]:
        return [col.name for col in self._backend.columns(table)]


def backend_as_schema_provider(backend: Backend) -> SchemaProvider:
    """Wrap a :class:`Backend` to satisfy the :class:`SchemaProvider` interface.

    Useful when feeding a backend to the planner during integration tests.
    """
    return _BackendSchemaProvider(backend)
