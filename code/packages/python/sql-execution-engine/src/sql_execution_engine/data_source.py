"""DataSource — the pluggable data interface for the SQL execution engine.

The engine is decoupled from any particular storage system via the
``DataSource`` abstract base class.  To execute SQL against your data,
you subclass ``DataSource`` and implement two methods:

- ``schema(table_name)`` — return the list of column names for the table.
- ``scan(table_name)``   — return all rows in the table as dicts.

The engine calls ``scan()`` once per referenced table and then does all
filtering, joining, and sorting in-memory using Python.  For production
systems you'd push predicates down to the storage layer, but for an
educational engine the simplicity of in-memory processing is the point.

Example Implementation
-----------------------

.. code-block:: python

    class InMemorySource(DataSource):
        _data = {
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
            ]
        }

        def schema(self, table_name: str) -> list[str]:
            if table_name not in self._data:
                raise TableNotFoundError(table_name)
            return list(self._data[table_name][0].keys())

        def scan(self, table_name: str) -> list[dict[str, Any]]:
            if table_name not in self._data:
                raise TableNotFoundError(table_name)
            return list(self._data[table_name])  # defensive copy

Why an ABC?
-----------

Making ``DataSource`` an abstract base class (``ABC``) means Python will
raise ``TypeError`` if someone instantiates it directly or forgets to
implement one of the required methods.  This gives clear error messages
rather than mysterious ``AttributeError`` surprises at runtime.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class DataSource(ABC):
    """Abstract interface for table data providers.

    Subclass this and implement ``schema()`` and ``scan()`` to connect
    the SQL execution engine to any data store.

    The engine calls these methods in this order:

    1. ``schema(table_name)`` — to discover column names for each table
       referenced in the FROM and JOIN clauses.  The engine uses this to
       validate column references and to build the star-expansion for
       ``SELECT *``.

    2. ``scan(table_name)`` — to retrieve all rows.  The engine will then
       apply WHERE, JOIN, GROUP BY, etc. on top of these rows in memory.

    Raising Errors
    --------------

    Both methods should raise ``TableNotFoundError`` (from
    ``sql_execution_engine.errors``) when given an unknown ``table_name``.
    """

    @abstractmethod
    def schema(self, table_name: str) -> list[str]:
        """Return the column names of a table.

        Args:
            table_name: The bare table name (no schema prefix).

        Returns:
            A list of column name strings in their natural order.

        Raises:
            TableNotFoundError: If the table does not exist.
        """

    @abstractmethod
    def scan(self, table_name: str) -> list[dict[str, Any]]:
        """Return all rows of a table as a list of dicts.

        Each dict maps column name → value.  Values may be Python
        ``None`` (representing SQL NULL), ``int``, ``float``, ``str``,
        or ``bool``.

        The engine does not modify the returned list, but implementations
        should return a defensive copy if the underlying storage is mutable.

        Args:
            table_name: The bare table name.

        Returns:
            A list of row dicts. May be empty if the table has no rows.

        Raises:
            TableNotFoundError: If the table does not exist.
        """
