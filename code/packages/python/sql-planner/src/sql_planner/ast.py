"""
Structured SQL AST — the planner's input interface
==================================================

This module defines a typed, structured AST that the planner consumes. It is
**intentionally distinct** from the raw parse tree produced by ``sql-parser``
(which is a generic ``ASTNode`` tree keyed by grammar rule names).

Why a separate structured AST?
------------------------------

Three reasons:

1. **Decoupling.** The parser's grammar can evolve — rules get renamed,
   alternatives get restructured — and the planner should not have to
   follow every change. By consuming a typed Statement hierarchy, the
   planner only cares about semantic shape, not syntactic rule names.

2. **Testability.** Tests can construct Statement trees directly, without
   going through tokenizer → parser → parse tree. A unit test for the
   WHERE-clause planner is a three-line ``SelectStmt(where=BinaryExpr(...))``
   rather than 50 lines of nested ASTNode construction.

3. **Clear pipeline boundary.** Later, we will add a ``parse_tree_adapter``
   module that walks the parser's raw ASTNode and produces these typed
   Statements. That adapter is the single place that knows about the
   parser's grammar rule names. Everything above it is grammar-agnostic.

Shape
-----

Every statement type is a frozen dataclass. Execution-irrelevant syntax
(commas, keywords, whitespace) is already gone; what remains is just the
semantic content. Clauses are represented as optional fields — ``where``,
``group_by``, ``having``, ``order_by``, ``limit`` — rather than as a flat
list, because every SELECT has *at most one* of each.

This is closer to what a hand-written AST in a compiler textbook looks like
than to what a grammar-driven parser produces, and that is deliberate: the
planner wants the textbook shape.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from sql_backend.schema import ColumnDef

from .expr import Expr

# ---- SELECT statement pieces ---------------------------------------------


@dataclass(frozen=True, slots=True)
class SelectItem:
    """One entry in a SELECT list. ``expr`` may be :class:`Wildcard` for ``*``."""

    expr: Expr
    alias: str | None = None


class JoinKind:
    """String constants instead of an Enum because join kinds appear in many
    places and the Enum import noise isn't worth the cost. Kept as a small
    namespace class for clarity."""

    INNER = "INNER"
    LEFT = "LEFT"
    RIGHT = "RIGHT"
    FULL = "FULL"
    CROSS = "CROSS"


@dataclass(frozen=True, slots=True)
class TableRef:
    """A reference to a base table in FROM, optionally aliased."""

    table: str
    alias: str | None = None


@dataclass(frozen=True, slots=True)
class DerivedTableRef:
    """A subquery used as a table source — ``(SELECT ...) AS alias``.

    The ``select`` is the inner query statement (already a typed SelectStmt,
    not a raw parse node).  The ``alias`` is mandatory — SQL requires an
    alias for every derived table.

    Example::

        SELECT dt.n FROM (SELECT COUNT(*) AS n FROM orders) AS dt
        ↓
        DerivedTableRef(
            select=SelectStmt(items=[SelectItem(AggregateExpr(COUNT, *), alias='n')],
                              from_=TableRef('orders')),
            alias='dt',
        )
    """

    select: SelectStmt
    alias: str


@dataclass(frozen=True, slots=True)
class JoinClause:
    """One JOIN appended to the FROM clause.

    The FROM clause is represented as a base :class:`TableRef` plus a list
    of join clauses, each describing how a new table attaches. This mirrors
    how most SQL grammars structure multi-table FROMs.
    """

    kind: str  # one of JoinKind.*
    right: TableRef | DerivedTableRef
    on: Expr | None = None  # None for CROSS JOIN


@dataclass(frozen=True, slots=True)
class SortKey:
    """One key in ORDER BY."""

    expr: Expr
    descending: bool = False
    nulls_first: bool | None = None  # None = backend default (nulls last for ASC)


@dataclass(frozen=True, slots=True)
class Limit:
    """LIMIT count OFFSET offset. Either may be None."""

    count: int | None = None
    offset: int | None = None


@dataclass(frozen=True, slots=True)
class SelectStmt:
    """A structured SELECT statement — the usual shape from a compiler textbook."""

    from_: TableRef | DerivedTableRef
    items: tuple[SelectItem, ...]
    joins: tuple[JoinClause, ...] = field(default_factory=tuple)
    where: Expr | None = None
    group_by: tuple[Expr, ...] = field(default_factory=tuple)
    having: Expr | None = None
    order_by: tuple[SortKey, ...] = field(default_factory=tuple)
    limit: Limit | None = None
    distinct: bool = False


# ---- DML statements -------------------------------------------------------


@dataclass(frozen=True, slots=True)
class InsertValuesStmt:
    """INSERT INTO t (cols) VALUES (v1, v2, ...), (w1, w2, ...)."""

    table: str
    columns: tuple[str, ...] | None  # None = implicit column list (all columns in order)
    rows: tuple[tuple[Expr, ...], ...]


@dataclass(frozen=True, slots=True)
class Assignment:
    """One SET clause of UPDATE."""

    column: str
    value: Expr


@dataclass(frozen=True, slots=True)
class UpdateStmt:
    """UPDATE t SET col = expr, ... WHERE predicate."""

    table: str
    assignments: tuple[Assignment, ...]
    where: Expr | None = None


@dataclass(frozen=True, slots=True)
class DeleteStmt:
    """DELETE FROM t WHERE predicate."""

    table: str
    where: Expr | None = None


# ---- Set-operation statements -----------------------------------------------


@dataclass(frozen=True, slots=True)
class UnionStmt:
    """SELECT … UNION [ALL] SELECT …

    ``left`` and ``right`` are the two sub-queries being combined.
    When ``all=True`` duplicate rows are preserved (UNION ALL); when
    ``all=False`` the result is deduplicated (UNION).

    ``left`` may itself be a set-operation statement to support
    left-associative chaining: ``A UNION B UNION C`` becomes
    ``UnionStmt(UnionStmt(A, B), C)``.
    """

    left: SelectStmt | UnionStmt | IntersectStmt | ExceptStmt
    right: SelectStmt
    all: bool = False


@dataclass(frozen=True, slots=True)
class IntersectStmt:
    """SELECT … INTERSECT [ALL] SELECT …

    Returns rows present in *both* result sets. ``all=True`` keeps
    duplicates up to the minimum multiplicity in each side.

    ``left`` may itself be a set-operation statement to support
    left-associative chaining.
    """

    left: SelectStmt | UnionStmt | IntersectStmt | ExceptStmt
    right: SelectStmt
    all: bool = False


@dataclass(frozen=True, slots=True)
class ExceptStmt:
    """SELECT … EXCEPT [ALL] SELECT …

    Returns rows present in the left set but not the right set.
    ``all=True`` subtracts multiplicities rather than sets.

    ``left`` may itself be a set-operation statement to support
    left-associative chaining.
    """

    left: SelectStmt | UnionStmt | IntersectStmt | ExceptStmt
    right: SelectStmt
    all: bool = False


# ---- INSERT … SELECT statement -------------------------------------------


@dataclass(frozen=True, slots=True)
class InsertSelectStmt:
    """INSERT INTO t (cols) SELECT …

    The ``select`` field is the sub-query whose result rows are inserted.
    ``columns`` is the explicit target column list; ``None`` means the
    table's natural column order is used (same semantics as VALUES INSERT).
    """

    table: str
    columns: tuple[str, ...] | None
    select: SelectStmt


# ---- Transaction-control statements ----------------------------------------


@dataclass(frozen=True, slots=True)
class BeginStmt:
    """BEGIN [TRANSACTION] — start an explicit transaction."""


@dataclass(frozen=True, slots=True)
class CommitStmt:
    """COMMIT [TRANSACTION] — commit the active transaction."""


@dataclass(frozen=True, slots=True)
class RollbackStmt:
    """ROLLBACK [TRANSACTION] — roll back the active transaction."""


# ---- DDL statements -------------------------------------------------------


@dataclass(frozen=True, slots=True)
class CreateTableStmt:
    """CREATE TABLE. Column defs use the same :class:`ColumnDef` as the backend.

    This reuse is why the planner depends on sql-backend: the schema shape
    is defined once, in the leaf package. Backend-level constraint
    enforcement and planner-level statement planning agree by construction.
    """

    table: str
    columns: tuple[ColumnDef, ...]
    if_not_exists: bool = False


@dataclass(frozen=True, slots=True)
class DropTableStmt:
    """DROP TABLE [IF EXISTS] t."""

    table: str
    if_exists: bool = False


@dataclass(frozen=True, slots=True)
class CreateIndexStmt:
    """CREATE [UNIQUE] INDEX [IF NOT EXISTS] name ON table (col1, col2, ...).

    The ``unique`` flag is stored but not enforced in v2; it is reserved for
    future use.  ``columns`` is the ordered list of indexed column names.
    """

    name: str
    table: str
    columns: tuple[str, ...]
    unique: bool = False
    if_not_exists: bool = False


@dataclass(frozen=True, slots=True)
class DropIndexStmt:
    """DROP INDEX [IF EXISTS] name."""

    name: str
    if_exists: bool = False


# The type union every Statement consumer matches on.
Statement = (
    SelectStmt
    | UnionStmt
    | IntersectStmt
    | ExceptStmt
    | InsertValuesStmt
    | InsertSelectStmt
    | UpdateStmt
    | DeleteStmt
    | CreateTableStmt
    | DropTableStmt
    | CreateIndexStmt
    | DropIndexStmt
    | BeginStmt
    | CommitStmt
    | RollbackStmt
)
