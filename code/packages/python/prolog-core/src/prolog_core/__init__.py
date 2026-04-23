"""Shared Prolog runtime model objects."""

from prolog_core.runtime import (
    OperatorAssociativity,
    OperatorSpec,
    OperatorTable,
    PrologDirective,
    __version__,
    directive,
    empty_operator_table,
    iso_operator_table,
    operator,
    swi_operator_table,
)

__all__ = [
    "__version__",
    "OperatorAssociativity",
    "OperatorSpec",
    "OperatorTable",
    "PrologDirective",
    "directive",
    "empty_operator_table",
    "iso_operator_table",
    "operator",
    "swi_operator_table",
]
