"""algol-type-checker — Type checker for the first ALGOL 60 compiler subset

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

from algol_type_checker.checker import (
    AlgolTypeChecker,
    Diagnostic,
    Scope,
    Symbol,
    TypeCheckError,
    TypeCheckResult,
    assert_algol_typed,
    check,
    check_algol,
)

__version__ = "0.1.0"

__all__ = [
    "AlgolTypeChecker",
    "Diagnostic",
    "Scope",
    "Symbol",
    "TypeCheckError",
    "TypeCheckResult",
    "assert_algol_typed",
    "check",
    "check_algol",
]
