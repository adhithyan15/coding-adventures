"""
Planner error hierarchy
=======================

Every failure mode during planning raises one of these. The facade
(``mini-sqlite``) maps them to PEP 249 exception classes; the user sees a
clear, typed failure.

Same design as ``sql_backend.errors``: a base class, frozen dataclasses for
structural equality in tests, one docstring per variant explaining when to
raise it.
"""

from __future__ import annotations

from dataclasses import dataclass


class PlanError(Exception):
    """Base class for all planning failures."""


@dataclass(eq=True)
class AmbiguousColumn(PlanError):
    """Raised when a bare column reference matches more than one table.

    Example: ``SELECT id FROM a JOIN b ON a.k = b.k`` — both tables have an
    ``id`` column. The user must qualify: ``a.id`` or ``b.id``.
    """

    column: str
    tables: list[str]

    def __str__(self) -> str:
        return f"ambiguous column {self.column!r}: present in {', '.join(self.tables)}"


@dataclass(eq=True)
class UnknownTable(PlanError):
    """Raised when a FROM clause names a table the SchemaProvider doesn't know about."""

    table: str

    def __str__(self) -> str:
        return f"unknown table: {self.table!r}"


@dataclass(eq=True)
class UnknownColumn(PlanError):
    """Raised when a column reference names a column no in-scope table contains."""

    table: str | None
    column: str

    def __str__(self) -> str:
        if self.table:
            return f"unknown column: {self.table}.{self.column}"
        return f"unknown column: {self.column!r}"


@dataclass(eq=True)
class InvalidAggregate(PlanError):
    """Raised when an aggregate function appears somewhere it can't — e.g. in WHERE.

    SQL semantics: WHERE runs per-row before grouping. Aggregates evaluate
    across groups. So an aggregate in WHERE is always an error. Likewise
    nested aggregates (SUM(COUNT(x))) are not allowed in basic SQL.
    """

    message: str

    def __str__(self) -> str:
        return self.message


@dataclass(eq=True)
class UnsupportedStatement(PlanError):
    """Raised when the planner sees a statement kind it does not yet handle.

    Examples in v1: ``UNION``, ``WITH``, subqueries in FROM clauses. These
    are on the roadmap but not blocking for the first integration.
    """

    kind: str

    def __str__(self) -> str:
        return f"unsupported statement: {self.kind}"


@dataclass(eq=True)
class InternalError(PlanError):
    """Escape hatch for planner bugs. Prefer a more specific variant if possible."""

    message: str

    def __str__(self) -> str:
        return self.message
