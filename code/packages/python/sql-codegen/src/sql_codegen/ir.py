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
    """Pop n values; push the first non-NULL, else NULL."""
    n: int


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
class ColumnDef:
    """One column in a CREATE TABLE — mirrors planner's ColumnDef."""
    name: str
    type: str
    nullable: bool = True


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
# Discriminated union — every Instruction variant.
# --------------------------------------------------------------------------

Instruction = (
    LoadConst | LoadColumn | Pop
    | BinaryOp | UnaryOp | IsNull | IsNotNull | Between | InList | Like | Coalesce
    | OpenScan | AdvanceCursor | CloseScan
    | BeginRow | EmitColumn | EmitRow | SetResultSchema | ScanAllColumns
    | InitAgg | UpdateAgg | FinalizeAgg | SaveGroupKey | LoadGroupKey | AdvanceGroupKey
    | SortResult | LimitResult | DistinctResult
    | InsertRow | UpdateRows | DeleteRows | CreateTable | DropTable
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
