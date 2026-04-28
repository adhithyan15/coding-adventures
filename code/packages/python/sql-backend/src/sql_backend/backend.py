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
from collections.abc import Iterator
from typing import NewType

from .index import IndexDef
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

    @abstractmethod
    def add_column(self, table: str, column: ColumnDef) -> None:
        """Add a new column to an existing table (ALTER TABLE … ADD COLUMN).

        Existing rows gain the column with the value ``column.default`` if a
        DEFAULT was specified, or NULL otherwise.  Raises
        :class:`TableNotFound` if the table does not exist and
        :class:`ColumnAlreadyExists` if a column with that name already exists.
        """

    # --- Indexes -----------------------------------------------------------

    @abstractmethod
    def create_index(self, index: IndexDef) -> None:
        """Create a new B-tree index and backfill it from existing table rows.

        The backend must:

        1. Reject the call with :class:`~sql_backend.errors.IndexAlreadyExists`
           if an index with ``index.name`` already exists.
        2. Reject with :class:`~sql_backend.errors.TableNotFound` if
           ``index.table`` is not a known table.
        3. Reject with :class:`~sql_backend.errors.ColumnNotFound` if any name
           in ``index.columns`` is not a column of ``index.table``.
        4. Allocate storage for the new index.
        5. Backfill the index with all existing rows from ``index.table`` so
           that the index is immediately consistent with the table data.

        Backfill must be atomic — either the index is fully built and visible,
        or nothing changes.  File-backed implementations should wrap the
        backfill in a transaction.

        Parameters
        ----------
        index:
            Full description of the index to build.  See :class:`IndexDef`.
        """

    @abstractmethod
    def drop_index(self, name: str, *, if_exists: bool = False) -> None:
        """Drop an existing index by name.

        After a successful call, the index's storage is reclaimed and
        ``list_indexes`` no longer returns an entry with this name.

        Parameters
        ----------
        name:
            The index name to drop.
        if_exists:
            When ``True``, silently succeed if no index named *name* exists.
            When ``False`` (the default), raise
            :class:`~sql_backend.errors.IndexNotFound` if absent.
        """

    @abstractmethod
    def list_indexes(self, table: str | None = None) -> list[IndexDef]:
        """Return all indexes, optionally filtered to a single table.

        Returns a list of :class:`IndexDef` descriptors in creation order.
        When *table* is ``None``, all indexes across all tables are returned.
        When *table* is given, only indexes whose ``IndexDef.table`` matches
        are included.

        The list is empty (not an error) when no indexes exist.

        Parameters
        ----------
        table:
            Optional table name to filter by.  Pass ``None`` to list all
            indexes in the database.
        """

    @abstractmethod
    def scan_index(
        self,
        index_name: str,
        lo: list[SqlValue] | None,
        hi: list[SqlValue] | None,
        *,
        lo_inclusive: bool = True,
        hi_inclusive: bool = True,
    ) -> Iterator[int]:
        """Yield rowids from the named index within the given key range.

        Iterates the index in ascending key order and yields the integer
        rowid of each matching table row.  The caller uses these rowids to
        fetch full rows (e.g. via a positioned scan or a rowid lookup).

        Parameters
        ----------
        index_name:
            The name of the index to scan.  Raises
            :class:`~sql_backend.errors.IndexNotFound` if the index does
            not exist.
        lo:
            Lower bound on the key values (one element per index column).
            ``None`` means unbounded — start from the first entry.
        hi:
            Upper bound on the key values.  ``None`` means unbounded.
        lo_inclusive:
            When ``True`` (default), entries whose key equals *lo* are
            included.
        hi_inclusive:
            When ``True`` (default), entries whose key equals *hi* are
            included.

        Yields
        ------
        int
            Rowids in ascending key order.
        """

    @abstractmethod
    def scan_by_rowids(self, table: str, rowids: list[int]) -> RowIterator:
        """Fetch the rows identified by *rowids* from *table*.

        Used by the VM after an index scan: ``scan_index`` yields integer rowids;
        this method materialises the corresponding full rows in the order the
        rowids are given.

        For the in-memory backend "rowid" is the 0-based list index.
        For the SQLite file backend "rowid" is the B-tree integer key.

        Raises :class:`~sql_backend.errors.TableNotFound` if the table does not
        exist.
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

    def current_transaction(self) -> TransactionHandle | None:
        """Return the active transaction handle, or ``None`` if no transaction
        is currently open.

        The default implementation returns ``None`` — suitable for read-only
        or stateless backends that do not track transaction state externally.
        Backends that support transactions should override this to return the
        handle they issued in ``begin_transaction``.

        This method exists so that multi-statement transaction sequences can
        be executed as separate :func:`~sql_vm.vm.execute` calls (each of
        which creates a fresh VM state) while still being able to carry the
        handle through to ``commit`` / ``rollback`` by consulting the backend
        directly. The VM's ``BeginTransaction`` instruction stores the handle
        on the backend; ``CommitTransaction`` / ``RollbackTransaction``
        retrieve it here.
        """
        return None


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

    Returns just the column names, plus proxies ``list_indexes`` so the
    planner can perform IX-6 index-scan substitution when a backend
    exposes indexes.  Private — callers use
    :func:`backend_as_schema_provider`.
    """

    def __init__(self, backend: Backend) -> None:
        self._backend = backend

    def columns(self, table: str) -> list[str]:
        return [col.name for col in self._backend.columns(table)]

    def list_indexes(self, table: str) -> list[IndexDef]:
        """Proxy ``Backend.list_indexes`` filtered to *table*."""
        return self._backend.list_indexes(table)


def backend_as_schema_provider(backend: Backend) -> SchemaProvider:
    """Wrap a :class:`Backend` to satisfy the :class:`SchemaProvider` interface.

    Useful when feeding a backend to the planner during integration tests.
    """
    return _BackendSchemaProvider(backend)
