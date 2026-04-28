"""mini-sqlite — PEP 249 DB-API 2.0 facade over the SQL pipeline."""

from __future__ import annotations

from .connection import Connection, connect
from .cursor import Cursor
from .errors import (
    DatabaseError,
    DataError,
    Error,
    IntegrityError,
    InterfaceError,
    InternalError,
    NotSupportedError,
    OperationalError,
    ProgrammingError,
    Warning,
)

# ------------------------------------------------------------------
# PEP 249 module-level attributes.
# ------------------------------------------------------------------

apilevel = "2.0"
"""PEP 249: the DB-API version this module implements."""

threadsafety = 1
"""PEP 249: 1 = threads may share the module but not connections."""

paramstyle = "qmark"
"""PEP 249: ``?``-style placeholders. See ``binding.py`` for substitution."""


__all__ = [
    "Connection",
    "Cursor",
    "DataError",
    "DatabaseError",
    "Error",
    "IntegrityError",
    "InterfaceError",
    "InternalError",
    "NotSupportedError",
    "OperationalError",
    "ProgrammingError",
    "Warning",
    "apilevel",
    "connect",
    "paramstyle",
    "threadsafety",
]
