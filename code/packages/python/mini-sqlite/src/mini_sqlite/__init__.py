"""mini-sqlite — PEP 249 DB-API 2.0 facade over the SQL pipeline."""

from __future__ import annotations

from .advisor import IndexAdvisor
from .connection import Connection, connect
from .cursor import Cursor
from .engine import QueryEvent
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
from .policy import HitCountPolicy, IndexPolicy

# ------------------------------------------------------------------
# PEP 249 module-level attributes.
# ------------------------------------------------------------------

apilevel = "2.0"
"""PEP 249: the DB-API version this module implements."""

threadsafety = 1
"""PEP 249: 1 = threads may share the module but not connections."""

paramstyle = "qmark"
"""PEP 249: declared paramstyle is ``qmark`` (``?`` placeholders).

The driver also accepts ``:name`` placeholders when *parameters* is a
mapping — matching the stdlib ``sqlite3`` module's behaviour, which also
declares ``qmark`` while accepting both styles at runtime.  See
``binding.py`` for substitution rules.
"""


__all__ = [
    "Connection",
    "Cursor",
    "DataError",
    "DatabaseError",
    "Error",
    "HitCountPolicy",
    "IndexAdvisor",
    "IndexPolicy",
    "IntegrityError",
    "InterfaceError",
    "InternalError",
    "NotSupportedError",
    "OperationalError",
    "ProgrammingError",
    "QueryEvent",
    "Warning",
    "apilevel",
    "connect",
    "paramstyle",
    "threadsafety",
]
