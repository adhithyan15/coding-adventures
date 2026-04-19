"""
LogicalPlan tree — the planner's output interface
=================================================

This is what every downstream stage consumes. The optimizer rewrites a
LogicalPlan into another LogicalPlan (same type, improved shape). The
codegen reads a LogicalPlan and emits IR bytecode.

Each node is a frozen dataclass with structural equality — tests assert on
equality of two trees, not on rendering or string comparison, which keeps
assertions tight and unambiguous.

Naming: we reuse the spec's names exactly (Scan, Filter, Project, ...). A
direct correspondence between spec and code pays off every time someone
debugs a plan shape by flipping back to the spec.

Why separate node types instead of one node with a kind enum?
-------------------------------------------------------------

Same argument as ``Expr``: different nodes carry different fields. ``Scan``
has a table name; ``Limit`` has a count. Packing them together forces every
field to be optional and every consumer to defensively check. Separating
them makes the structure a type-system property instead of a runtime
invariant.

Shape
-----

Read the tree bottom-up. Leaves are :class:`Scan` (read from a table) or
DDL nodes (:class:`CreateTable`, :class:`DropTable`). Internal nodes
transform their ``input`` field, which is itself a :class:`LogicalPlan`.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from sql_backend.schema import ColumnDef

from .expr import AggFunc, Expr, FuncArg

# ---- Leaf: table scan -----------------------------------------------------


@dataclass(frozen=True, slots=True)
class Scan:
    """Read all rows from a named table. Leaf node.

    ``alias`` is preserved because the planner has already resolved column
    references using it — the codegen uses the alias (or table name if none)
    to look up column values on the row the scan yields.

    Optimizer-added annotations (always None from the planner):

    - ``required_columns`` — if set, the column subset the query actually
      needs. Backends may use this to avoid materializing other columns.
    - ``scan_limit`` — if set, a hint that at most this many rows will be
      consumed. Backends may short-circuit; the VM still applies the true
      Limit higher in the tree.
    """

    table: str
    alias: str | None = None
    required_columns: tuple[str, ...] | None = None
    scan_limit: int | None = None


@dataclass(frozen=True, slots=True)
class EmptyResult:
    """Leaf node produced by DeadCodeElimination. Yields zero rows at runtime.

    ``columns`` preserves the output schema so downstream nodes (and the
    codegen) can still type-check. The field is a tuple of column names; the
    optimizer populates it from the schema of the replaced subtree when
    possible.
    """

    columns: tuple[str, ...] = ()


# ---- Transform nodes ------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Filter:
    """Keep only rows where predicate evaluates to TRUE.

    Rows where the predicate evaluates to FALSE or NULL are discarded. This
    is SQL semantics — the VM's three-valued logic ensures NULL predicates
    behave identically to FALSE predicates at this boundary, even though
    they mean different things elsewhere.
    """

    input: LogicalPlan
    predicate: Expr


@dataclass(frozen=True, slots=True)
class ProjectionItem:
    """One output column in a Project node."""

    expr: Expr
    alias: str | None = None


@dataclass(frozen=True, slots=True)
class Project:
    """Select / rename output columns.

    ``SELECT *`` produces a single item whose expression is :class:`Wildcard`.
    The codegen expands the Wildcard by querying the backend's schema for
    the scanned table's columns. Doing the expansion at codegen rather than
    planning time means intermediate plan rewrites don't have to re-expand
    the wildcard after the fact.
    """

    input: LogicalPlan
    items: tuple[ProjectionItem, ...]


@dataclass(frozen=True, slots=True)
class Join:
    """Combine rows from two inputs.

    Inner / Left / Right / Full / Cross correspond directly to the SQL join
    forms. Cross join has ``condition = None``; all others have a non-None
    condition.
    """

    left: LogicalPlan
    right: LogicalPlan
    kind: str  # JoinKind.*
    condition: Expr | None = None


@dataclass(frozen=True, slots=True)
class AggregateItem:
    """One aggregate function call in an Aggregate node.

    ``alias`` is the output column name. If the user wrote
    ``COUNT(*) AS n`` the alias is ``n``; otherwise the planner derives
    a default alias (``count``, ``sum_salary``) using standard SQL rules.
    """

    func: AggFunc
    arg: FuncArg
    alias: str
    distinct: bool = False


@dataclass(frozen=True, slots=True)
class Aggregate:
    """Group by expressions, compute aggregates per group.

    If ``group_by`` is empty but ``aggregates`` is non-empty, the whole
    input is one group (SQL's implicit aggregation). If both are empty,
    the planner doesn't emit an Aggregate node at all — it would be a no-op.
    """

    input: LogicalPlan
    group_by: tuple[Expr, ...]
    aggregates: tuple[AggregateItem, ...]


@dataclass(frozen=True, slots=True)
class Having:
    """Post-aggregation filter.

    Always appears above an :class:`Aggregate` node. Separating Having from
    Filter is deliberate: optimizers treat them differently — a Filter can
    be pushed below a Project, a Having cannot because it may reference
    aggregate results that don't exist below the Aggregate.
    """

    input: LogicalPlan
    predicate: Expr


@dataclass(frozen=True, slots=True)
class SortKey:
    """One key in an ORDER BY clause."""

    expr: Expr
    descending: bool = False
    nulls_first: bool | None = None  # None = backend default


@dataclass(frozen=True, slots=True)
class Sort:
    """Order rows by one or more sort keys."""

    input: LogicalPlan
    keys: tuple[SortKey, ...]


@dataclass(frozen=True, slots=True)
class Limit:
    """Take at most ``count`` rows, skipping the first ``offset``.

    Both count and offset are optional. If neither is set, Limit is a no-op
    — and the planner doesn't emit such a node. It exists only if at least
    one bound is finite.
    """

    input: LogicalPlan
    count: int | None = None
    offset: int | None = None


@dataclass(frozen=True, slots=True)
class Distinct:
    """Remove duplicate rows.

    Two rows are duplicates if every column value compares equal, with NULL
    treated as equal to NULL for deduplication (this is SQL-standard
    DISTINCT semantics — distinct from the three-valued logic used in
    predicates).
    """

    input: LogicalPlan


@dataclass(frozen=True, slots=True)
class Union:
    """Combine two result sets. ``all=True`` keeps duplicates (UNION ALL)."""

    left: LogicalPlan
    right: LogicalPlan
    all: bool = False


# ---- DML nodes ------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class InsertSource:
    """Source of INSERT rows — either literal VALUES tuples or a sub-query plan.

    We model this as a single class with either ``values`` OR ``query`` set
    (exclusive-or), enforced by ``__post_init__``. A union type would be
    cleaner, but Python's pattern-matching on literal classes is more
    awkward than a small invariant check here.
    """

    values: tuple[tuple[Expr, ...], ...] | None = None
    query: LogicalPlan | None = None

    def __post_init__(self) -> None:
        if (self.values is None) == (self.query is None):
            raise ValueError("InsertSource must set exactly one of values or query")


@dataclass(frozen=True, slots=True)
class Insert:
    """INSERT INTO t (cols) VALUES (...) or INSERT INTO t SELECT ...."""

    table: str
    columns: tuple[str, ...] | None  # None = implicit column list
    source: InsertSource


@dataclass(frozen=True, slots=True)
class Assignment:
    """One column assignment in UPDATE."""

    column: str
    value: Expr


@dataclass(frozen=True, slots=True)
class Update:
    """UPDATE t SET col = expr, ... WHERE predicate."""

    table: str
    assignments: tuple[Assignment, ...]
    predicate: Expr | None = None  # None = update every row


@dataclass(frozen=True, slots=True)
class Delete:
    """DELETE FROM t WHERE predicate."""

    table: str
    predicate: Expr | None = None


# ---- DDL nodes ------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class CreateTable:
    """CREATE TABLE. Column defs come from sql-backend unchanged."""

    table: str
    columns: tuple[ColumnDef, ...]
    if_not_exists: bool = False


@dataclass(frozen=True, slots=True)
class DropTable:
    """DROP TABLE [IF EXISTS] t."""

    table: str
    if_exists: bool = False


# The root union. Every plan function returns one of these.
LogicalPlan = (
    Scan
    | EmptyResult
    | Filter
    | Project
    | Join
    | Aggregate
    | Having
    | Sort
    | Limit
    | Distinct
    | Union
    | Insert
    | Update
    | Delete
    | CreateTable
    | DropTable
)


# ---- Tree-walking helpers -------------------------------------------------
#
# The optimizer and codegen both need to walk a plan tree. We centralize the
# per-node "children" mapping here so each consumer can write a single-line
# recursive walk instead of re-discriminating on every node type.


def children(node: LogicalPlan) -> tuple[LogicalPlan, ...]:
    """Return the immediate child plan nodes of ``node``.

    Scans / DDL nodes return an empty tuple; unary nodes return a 1-tuple;
    Join / Union return a 2-tuple. Insert's sub-query plan (if any) is
    included. Expressions are *not* children for this purpose — they aren't
    plan nodes.
    """
    match node:
        case Scan() | EmptyResult() | CreateTable() | DropTable():
            return ()
        case (
            Filter() | Project() | Aggregate() | Having()
            | Sort() | Limit() | Distinct()
        ):
            return (node.input,)
        case Join() | Union():
            return (node.left, node.right)
        case Insert(_, _, source):
            return (source.query,) if source.query is not None else ()
        case Update() | Delete():
            # UPDATE and DELETE don't have a plan-node input in this IR —
            # they implicitly scan their table. The backend's scan cursor
            # is wired up at codegen time.
            return ()
    # Exhaustiveness: every LogicalPlan variant is covered above. Reaching
    # here means a new node type was added without updating this helper.
    raise AssertionError(f"children() missing case for {type(node).__name__}")


@dataclass(frozen=True, slots=True)
class _Unused:
    """Prevent unused-import warnings from field() — imported only for type hints."""


# Make ``field`` reachable from this module so the ``__init__`` can re-export
# cleanly if we ever expose plan-node construction helpers.
_ = field
