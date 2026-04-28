"""
Intermediate Representation — the bytecode the VM executes.

Why a register-free stack machine?
----------------------------------

The VM is a loop: fetch, dispatch, advance. Instructions are tiny and
self-contained. Values live on a single stack; operations pop inputs and
push outputs. This is the same model as Python's bytecode compiler, the
JVM, and SQLite's VDBE.

Three supporting state spaces live beside the stack:

- ``cursor_table``: open iterators over tables, keyed by cursor_id.
- ``row_buffer`` / ``result_buffer``: the row currently being assembled
  and the list of completed rows.
- ``agg_state``: a hash map from group-key tuples to lists of aggregate
  accumulators. Managed by the VM; the codegen emits ``InitAgg`` /
  ``UpdateAgg`` / ``FinalizeAgg`` instructions that address slots.

Labels
------

``Label("name")`` is a zero-cost marker. Jump targets are strings during
generation; a single resolution pass after codegen turns every string
target into an integer instruction index stored on the ``Program``.
This two-phase approach lets the generator emit code without knowing
the final offset of forward jump targets — the same trick compilers
use for branches.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum

# --------------------------------------------------------------------------
# SQL values — what the stack and result buffer hold at runtime.
# --------------------------------------------------------------------------

SqlValue = None | bool | int | float | str | bytes
"""The runtime value type. NULL is represented as Python ``None``.

We keep ``bool`` distinct from ``int`` even though ``bool`` is a subclass
of ``int`` in Python: the three-valued logic used by SQL comparisons
cares about the difference (``True == 1`` but ``isinstance(True, bool)``
distinguishes booleans for NULL-propagation decisions).
"""


# --------------------------------------------------------------------------
# Operator enumerations — shared between the AST (planner) and IR (codegen).
# The IR uses its own enums so the VM does not import the planner. The
# compiler maps from planner BinaryOp / UnaryOp / AggFunc into these.
# --------------------------------------------------------------------------


class BinaryOpCode(Enum):
    ADD = "ADD"
    SUB = "SUB"
    MUL = "MUL"
    DIV = "DIV"
    MOD = "MOD"
    EQ = "EQ"
    NEQ = "NEQ"
    LT = "LT"
    LTE = "LTE"
    GT = "GT"
    GTE = "GTE"
    AND = "AND"
    OR = "OR"
    CONCAT = "CONCAT"


class UnaryOpCode(Enum):
    NEG = "NEG"
    NOT = "NOT"


class AggFunc(Enum):
    COUNT = "COUNT"
    COUNT_STAR = "COUNT_STAR"
    SUM = "SUM"
    AVG = "AVG"
    MIN = "MIN"
    MAX = "MAX"


class Direction(Enum):
    ASC = "ASC"
    DESC = "DESC"


class NullsOrder(Enum):
    FIRST = "FIRST"
    LAST = "LAST"


# --------------------------------------------------------------------------
# Instruction variants. Every instruction is a frozen dataclass so two
# equivalent programs are ``==``-comparable, which keeps tests clean.
# --------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class LoadConst:
    """Push a literal onto the value stack."""
    value: SqlValue


@dataclass(frozen=True, slots=True)
class LoadColumn:
    """Push the value of ``column`` from the current row of ``cursor_id``."""
    cursor_id: int
    column: str


@dataclass(frozen=True, slots=True)
class Pop:
    """Discard the top of the value stack."""


@dataclass(frozen=True, slots=True)
class BinaryOp:
    """Pop right, pop left, apply ``op``, push result."""
    op: BinaryOpCode


@dataclass(frozen=True, slots=True)
class UnaryOp:
    """Pop operand, apply ``op``, push result."""
    op: UnaryOpCode


@dataclass(frozen=True, slots=True)
class IsNull:
    """Pop value; push TRUE iff value is NULL."""


@dataclass(frozen=True, slots=True)
class IsNotNull:
    """Pop value; push FALSE iff value is NULL."""


@dataclass(frozen=True, slots=True)
class Between:
    """Pop high, low, value; push low <= value <= high (three-valued)."""


@dataclass(frozen=True, slots=True)
class InList:
    """Pop n list values then needle; push needle IN (list) using 3VL."""
    n: int


@dataclass(frozen=True, slots=True)
class Like:
    """Pop pattern, value; push TRUE iff value matches the SQL LIKE pattern."""
    negated: bool = False  # True for NOT LIKE


@dataclass(frozen=True, slots=True)
class Coalesce:
    """Pop n values; push the first non-NULL, else NULL.

    .. deprecated::
        New code should use ``CallScalar(func="coalesce", n_args=n)`` instead.
        This instruction is kept for backwards compatibility and is still
        dispatched by the VM.
    """
    n: int


@dataclass(frozen=True, slots=True)
class CallScalar:
    """Pop ``n_args`` values, call the named scalar function, push the result.

    ``func`` is the lower-cased SQL function name (e.g. ``"upper"``, ``"abs"``).
    The VM holds a registry of built-in implementations keyed by this name.
    Unrecognised names raise ``UnsupportedFunction`` at runtime.

    The call convention is **positional**: the last argument pushed is the
    first element popped, so :meth:`_VmState.pop_n` returns args in
    left-to-right (push) order.

    Built-in functions implemented in the VM:

    *NULL-handling*: ``coalesce``, ``nullif``, ``ifnull``, ``iif``

    *Type inspection*: ``typeof``

    *Numeric*: ``abs``, ``round``, ``ceil``, ``floor``, ``sign``, ``mod``,
    ``max`` (scalar 2-arg variant), ``min`` (scalar 2-arg variant)

    *String*: ``upper``, ``lower``, ``length``, ``trim``, ``ltrim``,
    ``rtrim``, ``substr`` / ``substring``, ``replace``, ``instr``,
    ``hex``, ``unhex``, ``quote``, ``printf`` / ``format``,
    ``char``, ``unicode``, ``zeroblob``, ``soundex``

    *Math*: ``sqrt``, ``pow`` / ``power``, ``log``, ``log2``, ``log10``,
    ``exp``, ``pi``, ``sin``, ``cos``, ``tan``, ``asin``, ``acos``, ``atan``,
    ``atan2``, ``degrees``, ``radians``

    *Utility*: ``random``, ``randomblob``, ``last_insert_rowid``
    (``last_insert_rowid`` always returns NULL in this implementation)
    """

    func: str      # lower-cased function name
    n_args: int    # number of arguments already on the stack


# ---- Scan instructions --------------------------------------------------


@dataclass(frozen=True, slots=True)
class OpenScan:
    """Ask the backend to open an iterator over ``table`` on ``cursor_id``."""
    cursor_id: int
    table: str


@dataclass(frozen=True, slots=True)
class AdvanceCursor:
    """Advance ``cursor_id`` one row; jump to ``on_exhausted`` when empty."""
    cursor_id: int
    on_exhausted: str


@dataclass(frozen=True, slots=True)
class CloseScan:
    """Release the cursor."""
    cursor_id: int


# ---- Row output ---------------------------------------------------------


@dataclass(frozen=True, slots=True)
class BeginRow:
    """Clear the row buffer; start assembling an output row."""


@dataclass(frozen=True, slots=True)
class EmitColumn:
    """Pop value; store it in the row buffer under ``name``."""
    name: str


@dataclass(frozen=True, slots=True)
class EmitRow:
    """Finalize the current row buffer and append to the result buffer."""


@dataclass(frozen=True, slots=True)
class SetResultSchema:
    """Declare output column names. Emitted once at program start."""
    columns: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class ScanAllColumns:
    """Pseudo-instruction: expand SELECT * at runtime via the cursor's schema."""
    cursor_id: int


# ---- Aggregates ---------------------------------------------------------


@dataclass(frozen=True, slots=True)
class InitAgg:
    """Initialize aggregate slot to its zero state for a new group."""
    slot: int
    func: AggFunc


@dataclass(frozen=True, slots=True)
class UpdateAgg:
    """Pop value; feed it to the aggregate at ``slot``."""
    slot: int


@dataclass(frozen=True, slots=True)
class FinalizeAgg:
    """Compute the final value of ``slot`` and push it."""
    slot: int


@dataclass(frozen=True, slots=True)
class SaveGroupKey:
    """Pop ``n`` values and save them as the current group key."""
    n: int


@dataclass(frozen=True, slots=True)
class LoadGroupKey:
    """Push the i-th value of the saved group key."""
    i: int


@dataclass(frozen=True, slots=True)
class AdvanceGroupKey:
    """Advance the VM's group iterator to the next group.

    Pre-execution: the aggregate accumulator holds one entry per distinct
    group. The per-group emit block begins with this instruction; the VM
    sets ``group_key`` to the next group in insertion order. When every
    group has been emitted, the VM jumps to ``on_exhausted`` — the
    label after the emit block — so the single-pass loop ends cleanly.

    This mirrors ``AdvanceCursor`` for scans: the same loop-with-exit-label
    pattern, but iterating over groups instead of rows.
    """
    on_exhausted: str


# ---- Result-buffer post-processing -------------------------------------


@dataclass(frozen=True, slots=True)
class SortKey:
    """One sort key: column name + direction + NULL ordering."""
    column: str
    direction: Direction = Direction.ASC
    nulls: NullsOrder = NullsOrder.LAST


@dataclass(frozen=True, slots=True)
class SortResult:
    """Sort the result buffer by ``keys``."""
    keys: tuple[SortKey, ...]


@dataclass(frozen=True, slots=True)
class LimitResult:
    """Skip ``offset`` rows, then keep at most ``count``."""
    count: int | None
    offset: int | None = None


@dataclass(frozen=True, slots=True)
class DistinctResult:
    """Deduplicate rows in the result buffer."""


# ---- Mutation -----------------------------------------------------------


@dataclass(frozen=True, slots=True)
class InsertRow:
    """Pop one value per column (last first); backend inserts the row."""
    table: str
    columns: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class InsertFromResult:
    """Drain the current result buffer, inserting every row into ``table``.

    Used for INSERT INTO … SELECT. After this instruction the result buffer
    is empty; ``rows_affected`` is set to the number of rows inserted.

    ``columns`` is the explicit target column list. If empty the VM uses
    the result schema's column order as the target column names. This
    mirrors the INSERT VALUES semantics: explicit column list wins, falling
    back to the table's natural order.
    """

    table: str
    columns: tuple[str, ...]  # empty = use result schema


@dataclass(frozen=True, slots=True)
class UpdateRows:
    """For the row under the current cursor, update the named assignments."""
    table: str
    assignments: tuple[str, ...]
    cursor_id: int


@dataclass(frozen=True, slots=True)
class DeleteRows:
    """Delete the row under the current cursor."""
    table: str
    cursor_id: int


@dataclass(frozen=True, slots=True)
class CaptureLeftResult:
    """Save the current result buffer as the "left side" of a set operation.

    After this instruction the result buffer is cleared and the saved rows
    live in VM state as ``left_result``. The right side's scan then fills
    the result buffer normally; the subsequent Intersect/ExceptResult
    instruction performs the final set arithmetic.

    Used by INTERSECT and EXCEPT compilation, which follows the pattern::

        <compile left side>
        CaptureLeftResult
        <compile right side>
        IntersectResult / ExceptResult
    """


@dataclass(frozen=True, slots=True)
class IntersectResult:
    """Compute the intersection of ``left_result`` and the current result buffer.

    ``all=False`` (INTERSECT): each distinct row appears once iff it is
    present in both sides.

    ``all=True`` (INTERSECT ALL): a row appears min(left_count, right_count)
    times, where the counts are the number of occurrences in each side.

    After this instruction ``left_result`` is cleared and the result buffer
    holds the intersection.
    """

    all: bool = False


@dataclass(frozen=True, slots=True)
class ExceptResult:
    """Compute the difference of ``left_result`` minus the current result buffer.

    ``all=False`` (EXCEPT): a row from the left side is excluded if it
    appears anywhere in the right side.

    ``all=True`` (EXCEPT ALL): each occurrence in the right side cancels one
    occurrence in the left side.

    After this instruction ``left_result`` is cleared and the result buffer
    holds the difference.
    """

    all: bool = False


# ---- Transaction control ------------------------------------------------


@dataclass(frozen=True, slots=True)
class BeginTransaction:
    """Call ``backend.begin_transaction()`` and store the handle.

    The handle is saved in VM state so subsequent Commit/Rollback can
    reference it. Nested BEGIN calls are not supported in v1 — a second
    BEGIN while a transaction is active raises ``TransactionError``.
    """


@dataclass(frozen=True, slots=True)
class CommitTransaction:
    """Call ``backend.commit(handle)`` and clear the stored handle."""


@dataclass(frozen=True, slots=True)
class RollbackTransaction:
    """Call ``backend.rollback(handle)`` and clear the stored handle."""


# Sentinel cursor_id used when the VM evaluates CHECK constraint expressions.
# Normal cursors are allocated starting from 0, so -1 is guaranteed distinct.
CHECK_CURSOR_ID: int = -1


@dataclass(frozen=True, slots=True)
class ColumnDef:
    """One column in a CREATE TABLE — mirrors planner's ColumnDef.

    ``check_instrs`` is a pre-compiled sequence of IR instructions that
    leaves a boolean (or NULL) on the stack when executed. The VM evaluates
    these against a new/updated row using the sentinel cursor id
    ``CHECK_CURSOR_ID``.  Empty tuple means no CHECK constraint.
    """
    name: str
    type: str
    nullable: bool = True
    check_instrs: tuple[Instruction, ...] = ()
    # (ref_table, ref_col_or_None) where None means "reference the parent PK".
    foreign_key: tuple[str, str | None] | None = None


@dataclass(frozen=True, slots=True)
class CreateTable:
    """Ask the backend to create a table."""
    table: str
    columns: tuple[ColumnDef, ...]
    if_not_exists: bool = False


@dataclass(frozen=True, slots=True)
class DropTable:
    """Ask the backend to drop a table."""
    table: str
    if_exists: bool = False


@dataclass(frozen=True, slots=True)
class AlterTable:
    """Ask the backend to add a column to an existing table."""
    table: str
    column: ColumnDef


@dataclass(frozen=True, slots=True)
class CreateIndex:
    """Ask the backend to create an index and backfill existing rows.

    ``columns`` is the ordered list of column names to index.
    The backend allocates a B-tree page, builds the index from all existing
    rows, and registers the index in ``sqlite_schema``.

    If ``if_not_exists=True`` and an index with the same name already exists,
    the instruction is silently skipped. Otherwise ``IndexAlreadyExists`` is
    raised.
    """
    name: str
    table: str
    columns: tuple[str, ...]
    unique: bool = False
    if_not_exists: bool = False


@dataclass(frozen=True, slots=True)
class DropIndex:
    """Ask the backend to drop an index by name.

    If ``if_exists=True`` and the index does not exist, the instruction is
    silently skipped. Otherwise ``IndexNotFound`` is raised.
    """
    name: str
    if_exists: bool = False


@dataclass(frozen=True, slots=True)
class OpenIndexScan:
    """Materialise rowids from an index range scan into ``cursor_id``.

    The VM calls ``backend.scan_index(index_name, lo, hi, lo_inclusive,
    hi_inclusive)`` to get matching rowids, then calls
    ``backend.scan_by_rowids(table, rowids)`` to materialise full rows.
    The resulting ``RowIterator`` is stored in ``cursor_table[cursor_id]``
    so that normal ``AdvanceCursor`` / ``LoadColumn`` / ``CloseScan``
    instructions can iterate over it without change.

    This single instruction replaces the ``OpenScan`` + filter-in-VM loop
    for index-covered predicates — it is semantically equivalent to
    ``OpenScan(cursor_id, table)`` followed by a WHERE filter, just faster.

    IX-8 change: ``lo`` and ``hi`` are now *tuples* of ``SqlValue`` rather
    than bare scalars.  A single-column scan uses a 1-tuple; a composite
    two-column scan uses a 2-tuple.  ``None`` still means "unbounded on
    that side".  The VM converts these to lists when passing them to
    ``backend.scan_index``, which requires ``list[SqlValue] | None``.
    """
    cursor_id: int
    table: str
    index_name: str
    lo: tuple[object, ...] | None   # SqlValue tuple or None (unbounded)
    hi: tuple[object, ...] | None   # SqlValue tuple or None (unbounded)
    lo_inclusive: bool = True
    hi_inclusive: bool = True


# ---- Derived-table (subquery in FROM) -----------------------------------


@dataclass(frozen=True, slots=True)
class RunSubquery:
    """Execute the inner sub-program and materialise its result rows under ``cursor_id``.

    Used for derived tables — ``(SELECT …) AS alias`` in a FROM clause.  The
    outer query then iterates over the materialised rows using the normal
    ``AdvanceCursor`` / ``LoadColumn`` / ``CloseScan`` instructions on the
    same ``cursor_id``.  The VM checks a per-state subquery-cursor dict before
    delegating to the backend so the outer loop is transparent to changes here.

    ``sub_program`` is a fully resolved inner :class:`Program` compiled
    independently with its own cursor/label namespace.  The VM runs it in a
    temporary child state, collects the result rows, and stores them indexed
    by ``cursor_id``.
    """

    cursor_id: int
    sub_program: Program   # Program is defined below; forward-ref resolved by PEP 563


@dataclass(frozen=True, slots=True)
class RunExistsSubquery:
    """Execute the inner sub-program; push TRUE iff it returns at least one row.

    Used for ``EXISTS (subquery)`` in WHERE, HAVING, and SELECT projection.
    Unlike :class:`RunSubquery` (which materialises rows for cursor-based
    iteration), this instruction is a pure boolean test — it executes the inner
    plan in a temporary child state, checks the row count, and pushes a boolean
    onto the outer expression stack.

    ``NOT EXISTS`` is handled by the caller: a :class:`UnaryOp` ``NOT``
    instruction is emitted after this one, inverting the boolean result.
    """

    sub_program: Program   # fully compiled inner SELECT program


@dataclass(frozen=True, slots=True)
class RunRecursiveCTE:
    """Execute a WITH RECURSIVE CTE via fixed-point iteration.

    Algorithm (see VM ``_do_run_recursive_cte``):

    1. Run ``anchor_program`` once → initial working set.
    2. Loop while working set is non-empty:
       a. Run ``recursive_program`` with cursor ``working_cursor_id``
          pre-populated from the current working set.
       b. Relabel result rows with the anchor's output column names
          (SQL rule: UNION ALL names come from the left/anchor side).
       c. If ``union_all`` is False, discard rows already in the
          accumulated set (UNION semantics — cycle prevention).
       d. Extend the accumulated result; update working set.
    3. Wrap the accumulated rows in a :class:`~sql_vm.vm._SubqueryCursor`
       and store under ``cursor_id`` for the outer scan loop.

    ``working_cursor_id`` is the cursor slot pre-allocated inside
    ``recursive_program`` for the working-set scan (always cursor 0 in the
    recursive sub-program, because the recursive FROM source is compiled
    first).
    """

    cursor_id: int
    anchor_program: Program
    recursive_program: Program
    working_cursor_id: int
    union_all: bool = True


@dataclass(frozen=True, slots=True)
class OpenWorkingSetScan:
    """Open a fresh cursor over the recursive working-set rows.

    Used inside recursive sub-programs so that ``WorkingSetScan`` loops work
    correctly when the CTE self-reference appears inside a JOIN.  Each time
    this instruction executes, it materialises a brand-new
    :class:`~sql_vm.vm._SubqueryCursor` from ``vm_state.working_set_data``
    and stores it under ``cursor_id``.  The subsequent ``AdvanceCursor`` /
    ``CloseScan`` loop iterates that fresh cursor — so even if the outer JOIN
    loop calls this many times, each iteration starts from row zero.
    """

    cursor_id: int


# ---- Control flow -------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Label:
    """A named jump target. No-op at runtime."""
    name: str


@dataclass(frozen=True, slots=True)
class Jump:
    """Unconditional branch to ``label``."""
    label: str


@dataclass(frozen=True, slots=True)
class JumpIfFalse:
    """Pop value; branch if FALSE or NULL."""
    label: str


@dataclass(frozen=True, slots=True)
class JumpIfTrue:
    """Pop value; branch if TRUE."""
    label: str


@dataclass(frozen=True, slots=True)
class Halt:
    """Stop the VM. Result buffer holds the output."""


# --------------------------------------------------------------------------
# Window functions
# --------------------------------------------------------------------------


class WinFunc(Enum):
    """Supported window (analytic) functions.

    Ranking functions
    -----------------
    ROW_NUMBER   — 1-based sequential integer within the ordered partition.
                   Ties get different numbers depending on order.
    RANK         — like ROW_NUMBER but identical ORDER BY values share the same
                   rank; the next rank after a tie jumps (1, 1, 3, …).
    DENSE_RANK   — like RANK but without gaps (1, 1, 2, …).

    Aggregate-style functions
    -------------------------
    SUM / COUNT / COUNT_STAR / AVG / MIN / MAX
        — cumulate over the *entire* partition (the full-partition frame is
          used, which is the default when no ROWS/RANGE clause is given).
          ``COUNT_STAR`` is ``COUNT(*)`` — no arg column, always counts rows.

    Value functions
    ---------------
    FIRST_VALUE  — the value of ``arg_col`` in the first row of the partition.
    LAST_VALUE   — the value of ``arg_col`` in the last row of the partition.
    """

    ROW_NUMBER = "ROW_NUMBER"
    RANK = "RANK"
    DENSE_RANK = "DENSE_RANK"
    SUM = "SUM"
    COUNT = "COUNT"
    COUNT_STAR = "COUNT_STAR"
    AVG = "AVG"
    MIN = "MIN"
    MAX = "MAX"
    FIRST_VALUE = "FIRST_VALUE"
    LAST_VALUE = "LAST_VALUE"


@dataclass(frozen=True, slots=True)
class WinFuncSpec:
    """Column-level specification for one window function call.

    The IR represents all arguments as column names (strings) because at
    codegen time every expression has already been compiled into the inner
    projection's output schema.  The VM looks up ``arg_col`` /
    ``partition_cols`` / ``order_cols`` in ``result.columns`` to find the
    correct positional index.

    Fields
    ------
    func:
        Which window function to evaluate.
    arg_col:
        Name of the column in the result buffer that holds the function's
        argument (e.g. ``"salary"`` for ``SUM(salary)``).  ``None`` for
        arg-free functions (``ROW_NUMBER``, ``RANK``, ``DENSE_RANK``) and
        ``COUNT_STAR``.
    partition_cols:
        Ordered tuple of column names to partition by.  An empty tuple means
        the whole result is one partition.
    order_cols:
        Ordered tuple of ``(column_name, descending)`` pairs that define the
        ordering within each partition.  Required for ranking functions.
    result_col:
        The output column name for this window function (the SELECT alias).
    """

    func: WinFunc
    arg_col: str | None
    partition_cols: tuple[str, ...]
    order_cols: tuple[tuple[str, bool], ...]
    result_col: str


@dataclass(frozen=True, slots=True)
class ComputeWindowFunctions:
    """Post-process the result buffer to evaluate window functions.

    Execution model
    ---------------
    By the time this instruction executes, the inner scan loop has finished
    and ``vm_state.result.rows`` holds every output row as a positional
    tuple.  ``ComputeWindowFunctions`` works entirely on that buffer:

    1. Convert each row tuple to a dict keyed by ``result.columns``.
    2. For each :class:`WinFuncSpec` in ``specs``:
       a. Group rows into partitions by the values of ``partition_cols``.
       b. Sort each partition by ``order_cols`` (stable sort).
       c. Evaluate the window function over the sorted partition, assigning
          a value to each row.
       d. Store the value in ``result_col`` on each dict.
    3. Project each dict down to ``output_cols`` (in order) and convert back
       to tuples.
    4. Replace ``result.rows`` and set ``result.columns = output_cols``.

    This single-pass design avoids materialising multiple intermediate
    buffers and is correct for non-nested window functions.
    """

    specs: tuple[WinFuncSpec, ...]
    output_cols: tuple[str, ...]  # final column projection after all specs


# --------------------------------------------------------------------------
# Discriminated union — every Instruction variant.
# --------------------------------------------------------------------------

Instruction = (
    LoadConst | LoadColumn | Pop
    | BinaryOp | UnaryOp | IsNull | IsNotNull | Between | InList | Like | Coalesce | CallScalar
    | OpenScan | AdvanceCursor | CloseScan
    | BeginRow | EmitColumn | EmitRow | SetResultSchema | ScanAllColumns
    | InitAgg | UpdateAgg | FinalizeAgg | SaveGroupKey | LoadGroupKey | AdvanceGroupKey
    | SortResult | LimitResult | DistinctResult
    | ComputeWindowFunctions
    | InsertRow | InsertFromResult | UpdateRows | DeleteRows | CreateTable | DropTable | AlterTable
    | CreateIndex | DropIndex | OpenIndexScan
    | CaptureLeftResult | IntersectResult | ExceptResult
    | BeginTransaction | CommitTransaction | RollbackTransaction
    | RunSubquery
    | RunExistsSubquery
    | RunRecursiveCTE
    | OpenWorkingSetScan
    | Label | Jump | JumpIfFalse | JumpIfTrue | Halt
)


@dataclass(frozen=True, slots=True)
class Program:
    """The final compiled bytecode.

    ``instructions`` is the flat stream. ``labels`` maps every label name
    to its instruction index — computed once by the resolver so the VM
    can jump in O(1). ``result_schema`` is the ordered output column
    list, set by the single ``SetResultSchema`` at the program's start
    (or empty for DDL / DML programs).
    """

    instructions: tuple[Instruction, ...]
    labels: dict[str, int] = field(default_factory=dict)
    result_schema: tuple[str, ...] = ()
