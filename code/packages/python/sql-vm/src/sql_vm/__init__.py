"""
sql-vm — dispatch-loop virtual machine for IR bytecode.

The VM executes a :class:`sql_codegen.Program` against a
:class:`sql_backend.Backend` and returns a :class:`QueryResult`. It is the
final stage of the SQL pipeline — after lexer, parser, planner, optimizer,
and codegen — and its only job is to move values through a stack while
honoring SQL's value semantics.

Public surface::

    from sql_vm import execute, QueryResult, VmError
    result = execute(program, backend)
"""

from __future__ import annotations

from .errors import (
    BackendError,
    ColumnAlreadyExists,
    ColumnNotFound,
    ConstraintViolation,
    DivisionByZero,
    InternalError,
    InvalidLabel,
    StackUnderflow,
    TableAlreadyExists,
    TableNotFound,
    TransactionError,
    TypeMismatch,
    UnsupportedFunction,
    VmError,
    WrongNumberOfArguments,
)
from .result import QueryResult
from .scalar_functions import call as call_scalar
from .vm import QueryEvent, execute, set_event_listener

__all__ = [
    "BackendError",
    "ColumnAlreadyExists",
    "ColumnNotFound",
    "ConstraintViolation",
    "DivisionByZero",
    "InternalError",
    "InvalidLabel",
    "QueryEvent",
    "QueryResult",
    "StackUnderflow",
    "TableAlreadyExists",
    "TableNotFound",
    "TransactionError",
    "TypeMismatch",
    "UnsupportedFunction",
    "VmError",
    "WrongNumberOfArguments",
    "call_scalar",
    "execute",
    "set_event_listener",
]
