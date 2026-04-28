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
    ColumnNotFound,
    ConstraintViolation,
    DivisionByZero,
    InternalError,
    InvalidLabel,
    StackUnderflow,
    TableAlreadyExists,
    TableNotFound,
    TypeMismatch,
    VmError,
)
from .result import QueryResult
from .vm import execute

__all__ = [
    "BackendError",
    "ColumnNotFound",
    "ConstraintViolation",
    "DivisionByZero",
    "InternalError",
    "InvalidLabel",
    "QueryResult",
    "StackUnderflow",
    "TableAlreadyExists",
    "TableNotFound",
    "TypeMismatch",
    "VmError",
    "execute",
]
