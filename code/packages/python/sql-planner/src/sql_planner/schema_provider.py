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
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

from .errors import UnknownTable


@runtime_checkable
class SchemaProvider(Protocol):
    """Read-only schema interface — the minimum the planner needs.

    Implementations raise :class:`sql_backend.TableNotFound` (or any
    subclass) when a table is unknown. The planner catches the backend
    error type and re-raises as :class:`UnknownTable` so the facade sees a
    pure planner error type.
    """

    def columns(self, table: str) -> list[str]: ...


class InMemorySchemaProvider:
    """Dict-backed SchemaProvider — used in planner unit tests.

    Example::

        sp = InMemorySchemaProvider({"users": ["id", "name", "email"]})
        sp.columns("users")  # ["id", "name", "email"]

    Uses its own ``UnknownTable`` to signal missing tables, so planner
    tests don't need to import backend error types.
    """

    def __init__(self, schema: dict[str, list[str]]) -> None:
        # Defensive copy so callers can't mutate the provider's state after
        # construction without going through the ctor.
        self._schema: dict[str, list[str]] = {k: list(v) for k, v in schema.items()}

    def columns(self, table: str) -> list[str]:
        if table not in self._schema:
            raise UnknownTable(table=table)
        return list(self._schema[table])

    def tables(self) -> list[str]:
        """Not required by the Protocol, but handy for error messages."""
        return list(self._schema.keys())
