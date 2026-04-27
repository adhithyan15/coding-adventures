"""
SchemaProvider — the planner's read-only view of the schema
===========================================================

The planner needs to know which columns each table has so it can:

- Qualify bare column references (``salary`` → ``employees.salary``).
- Detect ambiguity (``id`` referring to both ``users.id`` and
  ``orders.id``).
- Expand ``SELECT *`` into the concrete column list at projection time.

It does **not** need to read or write rows. So we pass in a minimal
:class:`SchemaProvider` Protocol instead of a full
``sql_backend.Backend``. The sql_backend package ships an adapter
(:func:`sql_backend.backend_as_schema_provider`) that wraps any Backend.

This shape means:

- Planner tests can supply a plain Python dict as the schema.
- Backend integration is one call away (``backend_as_schema_provider(b)``).
- The planner doesn't accidentally call any mutating method.

InMemorySchemaProvider
----------------------

A dict-backed implementation is included for tests. Every test that
exercises column resolution builds one of these and hands it to ``plan()``.

Index awareness (IX-6)
----------------------

The planner optionally uses index information to substitute
``Filter(Scan)`` with :class:`~sql_planner.plan.IndexScan` when a
predicate column is covered by a B-tree index.  This is an *optional*
capability: :class:`SchemaProvider` only requires ``columns``; the
``list_indexes`` method is discovered via duck-typing so that existing
schema providers (tests, etc.) that don't know about indexes continue to
work unchanged.

Backend-backed providers that wrap a :class:`sql_backend.Backend` expose
``list_indexes`` automatically via :class:`_BackendSchemaProvider`.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol, runtime_checkable

from .errors import UnknownTable

if TYPE_CHECKING:
    from sql_backend.index import IndexDef


@runtime_checkable
class SchemaProvider(Protocol):
    """Read-only schema interface — the minimum the planner needs.

    Implementations raise :class:`sql_backend.TableNotFound` (or any
    subclass) when a table is unknown. The planner catches the backend
    error type and re-raises as :class:`UnknownTable` so the facade sees a
    pure planner error type.

    Optional: ``list_indexes(table: str) -> list[IndexDef]``
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    Providers that implement ``list_indexes`` enable the planner's
    :class:`~sql_planner.plan.IndexScan` substitution (IX-6).  Providers
    that don't have this method are treated as "no indexes known" and the
    planner emits plain ``Filter(Scan)`` nodes instead.
    """

    def columns(self, table: str) -> list[str]: ...


class InMemorySchemaProvider:
    """Dict-backed SchemaProvider — used in planner unit tests.

    Example (columns only)::

        sp = InMemorySchemaProvider({"users": ["id", "name", "email"]})
        sp.columns("users")  # ["id", "name", "email"]

    Example (with index metadata for IX-6 tests)::

        from sql_backend.index import IndexDef
        sp = InMemorySchemaProvider(
            {"orders": ["id", "user_id", "total"]},
            indexes=[IndexDef(name="idx_orders_user_id", table="orders",
                              columns=["user_id"])],
        )
        sp.list_indexes("orders")  # [IndexDef(...)]

    Uses its own ``UnknownTable`` to signal missing tables, so planner
    tests don't need to import backend error types.
    """

    def __init__(
        self,
        schema: dict[str, list[str]],
        indexes: list[IndexDef] | None = None,
    ) -> None:
        # Defensive copy so callers can't mutate the provider's state after
        # construction without going through the ctor.
        self._schema: dict[str, list[str]] = {k: list(v) for k, v in schema.items()}
        # Defensive copy of indexes list.  An empty list means "index-aware but
        # no indexes exist" — still enables index-scan path (no match found →
        # planner emits Filter(Scan)).  None is only used internally as a
        # sentinel; the public ``list_indexes`` always returns a list.
        self._indexes: list[IndexDef] = list(indexes) if indexes is not None else []

    def columns(self, table: str) -> list[str]:
        if table not in self._schema:
            raise UnknownTable(table=table)
        return list(self._schema[table])

    def tables(self) -> list[str]:
        """Not required by the Protocol, but handy for error messages."""
        return list(self._schema.keys())

    def list_indexes(self, table: str) -> list[IndexDef]:
        """Return all indexes whose ``table`` field matches *table*.

        Returns an empty list when no indexes are registered.  The planner
        calls this via ``getattr(schema, 'list_indexes', None)`` so providers
        that don't expose this method are treated as "no indexes" instead of
        raising AttributeError.
        """
        return [idx for idx in self._indexes if idx.table == table]
