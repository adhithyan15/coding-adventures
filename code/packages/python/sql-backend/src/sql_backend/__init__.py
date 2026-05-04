"""
sql_backend — the pluggable data-source interface for the SQL pipeline.

Importing from this package gives you everything you need to implement a
backend, construct one, or write conformance tests against one:

    from sql_backend import (
        Backend,              # the ABC every backend subclasses
        InMemoryBackend,      # reference implementation
        ColumnDef,            # schema element
        Row, RowIterator,     # data in motion
        Cursor,               # positioned DML
        BackendError,         # base of the error hierarchy
        TableNotFound, ...    # specific errors
    )

See the package modules for docstrings with the design rationale.
"""

from .backend import (
    Backend,
    SchemaProvider,
    TransactionHandle,
    backend_as_schema_provider,
)
from .errors import (
    BackendError,
    ColumnAlreadyExists,
    ColumnNotFound,
    ConstraintViolation,
    IndexAlreadyExists,
    IndexNotFound,
    Internal,
    TableAlreadyExists,
    TableNotFound,
    TriggerAlreadyExists,
    TriggerNotFound,
    Unsupported,
)
from .in_memory import InMemoryBackend
from .index import IndexDef
from .row import Cursor, ListCursor, ListRowIterator, Row, RowIterator
from .schema import NO_DEFAULT, ColumnDef, ColumnDefault, TriggerDef
from .values import SqlValue, is_sql_value, sql_type_name

__all__ = [
    "NO_DEFAULT",
    "Backend",
    "BackendError",
    "ColumnAlreadyExists",
    "ColumnDef",
    "ColumnDefault",
    "ColumnNotFound",
    "ConstraintViolation",
    "Cursor",
    "IndexAlreadyExists",
    "IndexDef",
    "IndexNotFound",
    "InMemoryBackend",
    "Internal",
    "ListCursor",
    "ListRowIterator",
    "Row",
    "RowIterator",
    "SchemaProvider",
    "SqlValue",
    "TableAlreadyExists",
    "TableNotFound",
    "TransactionHandle",
    "TriggerAlreadyExists",
    "TriggerDef",
    "TriggerNotFound",
    "Unsupported",
    "backend_as_schema_provider",
    "is_sql_value",
    "sql_type_name",
]
