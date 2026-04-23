"""Shared Prolog runtime model objects."""

from prolog_core.runtime import (
    OperatorAssociativity,
    OperatorSpec,
    OperatorTable,
    PredicateRegistry,
    PredicateSpec,
    PrologDirective,
    __version__,
    apply_op_directive,
    apply_predicate_directive,
    directive,
    empty_operator_table,
    empty_predicate_registry,
    iso_operator_table,
    operator,
    swi_operator_table,
)

__all__ = [
    "__version__",
    "OperatorAssociativity",
    "OperatorSpec",
    "OperatorTable",
    "PredicateRegistry",
    "PredicateSpec",
    "PrologDirective",
    "apply_predicate_directive",
    "apply_op_directive",
    "directive",
    "empty_operator_table",
    "empty_predicate_registry",
    "iso_operator_table",
    "operator",
    "swi_operator_table",
]
