"""
coding-adventures-type-checker-protocol
========================================

Generic type-checker protocol for the coding-adventures compiler stack.

This package defines the shared interface that all language type checkers in
this repository must implement.  Import from here rather than from the
internal ``protocol`` module so that the public API is stable even if we
reorganise the internals.

Public API
----------
TypeChecker
    ``typing.Protocol[ASTIn, ASTOut]`` — the interface all type checkers must
    implement.  Structural: no inheritance required.

TypeCheckResult
    Frozen dataclass carrying the typed AST and any errors from a single pass.
    Check ``.ok`` to see whether the pass succeeded.

TypeErrorDiagnostic
    Frozen dataclass representing one type error with ``message``, ``line``,
    and ``column``.

Example
-------
    from type_checker_protocol import TypeChecker, TypeCheckResult, TypeErrorDiagnostic

    class MyTypeChecker:
        def check(self, ast: MyNode) -> TypeCheckResult[TypedMyNode]:
            ...
"""

from type_checker_protocol.protocol import (
    TypeCheckResult,
    TypeChecker,
    TypeErrorDiagnostic,
)
from type_checker_protocol.generic import GenericTypeChecker

__all__ = [
    "GenericTypeChecker",
    "TypeChecker",
    "TypeCheckResult",
    "TypeErrorDiagnostic",
]
