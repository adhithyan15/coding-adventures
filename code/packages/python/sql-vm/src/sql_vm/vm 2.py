"""
The dispatch-loop VM
====================

Execution is a very small loop::

    while pc < len(instructions):
        ins = instructions[pc]; pc += 1
        dispatch(ins, state)

``dispatch`` is a single ``match`` statement over every ``Instruction``
variant. Each arm performs the effect described in the spec:

- Stack ops push/pop values
- Scan ops open, advance, and close backend iterators
- Row ops manipulate ``row_buffer`` and append finalized rows to
  ``result_buffer``
- Aggregate ops maintain a per-group-key table of accumulators
- Post-processing ops (sort / limit / distinct) run on the finalized result
  buffer, in-place
- DML ops call into the backend and increment ``rows_affected``
- Jumps rewrite ``pc`` to a labeled index

That's the whole story. Every "tricky" piece of SQL — NULL semantics,
aggregate initialization, LIKE matching, three-valued logic — is factored
out into helpers so that this file reads like pseudocode.
"""

from __future__ import annotations

from dataclasses import dataclass, field

import sql_backend.errors as be
from sql_backend.backend import Backend
from sql_backend.row import Cursor
from sql_backend.values import SqlValue, sql_type_name
from sql_codegen import (
    AdvanceCursor,
    AdvanceGroupKey,
    BeginRow,
    Between,
    BinaryOp,
    CloseScan,
    Coalesce,
    CreateTable,
    DeleteRows,
    Direction,
    DistinctResult,
    DropTable,
    EmitColumn,
    EmitRow,
    FinalizeAgg,
    Halt,
    InitAgg,
    InList,
    InsertRow,
    Instruction,
    IsNotNull,
    IsNull,
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    Label,
    Like,
    LimitResult,
    LoadColumn,
    LoadConst,
    LoadGroupKey,
    NullsOrder,
    OpenScan,
    Pop,
    Program,
    SaveGroupKey,
    ScanAllColumns,
    SetResultSchema,
    SortResult,
    UnaryOp,
    UpdateAgg,
    UpdateRows,
)
from sql_codegen import IrAggFunc as AggFunc

from .errors import (
    BackendError,
    ColumnNotFound,
    ConstraintViolation,
    InternalError,
    InvalidLabel,
    StackUnderflow,
    TableAlreadyExists,
    TableNotFound,
)
from .operators import apply_binary, apply_unary, like_match
from .result import QueryResult, _MutableResult

# --------------------------------------------------------------------------
# Aggregate state — one of these lives in each slot of ``agg_table[key]``.
# --------------------------------------------------------------------------


@dataclass(slots=True)
class _AggState:
    """Mutable accumulator for a single aggregate slot in a single group.

    The field that matters depends on ``func``:

    - COUNT / COUNT(*)  — count
    - SUM               — sum (Null until first non-null input)
    - AVG               — sum + count
    - MIN / MAX         — extremum
    """

    func: AggFunc
    count: int = 0
    acc: SqlValue = None  # sum / min / max carrier


# --------------------------------------------------------------------------
# VM state. Public API is only the ``execute`` entry point below.
# --------------------------------------------------------------------------


@dataclass(slots=True)
class _VmState:
    program: Program
    backend: Backend
    pc: int = 0
    stack: list[SqlValue] = field(default_factory=list)
    cursors: dict[int, Cursor] = field(default_factory=dict)
    current_row: dict[int, dict[str, SqlValue]] = field(default_factory=dict)
    row_buffer: dict[str, SqlValue] = field(default_factory=dict)
    result: _MutableResult = field(default_factory=_MutableResult)
    # Aggregate state, keyed by the tuple of group key values.
    agg_table: dict[tuple[SqlValue, ...], list[_AggState]] = field(default_factory=dict)
    # The VM's notion of "current group key". During the scan loop it carries
    # the values of the GROUP BY expressions for the current row; during the
    # emit loop it's rewritten by LoadGroupKey/SaveGroupKey as we iterate.
    group_key: tuple[SqlValue, ...] = ()
    # Ordered list of groups we've seen, so iteration is deterministic.
    group_order: list[tuple[SqlValue, ...]] = field(default_factory=list)
    # Cursor into ``group_order`` for the per-group emit loop. -1 = not yet
    # started; AdvanceGroupKey increments it each iteration.
    group_iter: int = -1

    def push(self, v: SqlValue) -> None:
        self.stack.append(v)

    def pop(self) -> SqlValue:
        if not self.stack:
            raise StackUnderflow()
        return self.stack.pop()

    def pop_n(self, n: int) -> list[SqlValue]:
        """Pop n values; return them in push order (oldest first)."""
        if len(self.stack) < n:
            raise StackUnderflow()
        result = self.stack[-n:]
        del self.stack[-n:]
        return result


# --------------------------------------------------------------------------
# Public API.
# --------------------------------------------------------------------------


def execute(program: Program, backend: Backend) -> QueryResult:
    """Execute ``program`` against ``backend`` and return the result.

    This is the VM's single public entry point. It creates a fresh
    :class:`_VmState`, runs the dispatch loop, and packages up the result.
    """
    state = _VmState(program=program, backend=backend)
    instructions = program.instructions
    n = len(instructions)
    while state.pc < n:
        ins = instructions[state.pc]
        state.pc += 1
        if isinstance(ins, Halt):
            break
        _dispatch(ins, state)
    return state.result.freeze()


# --------------------------------------------------------------------------
# Dispatch. One match statement over the Instruction type hierarchy.
# --------------------------------------------------------------------------


def _dispatch(ins: Instruction, st: _VmState) -> None:  # noqa: PLR0912, C901
    # Stack / constants ---------------------------------------------------
    if isinstance(ins, LoadConst):
        st.push(ins.value)
        return
    if isinstance(ins, LoadColumn):
        _load_column(ins, st)
        return
    if isinstance(ins, Pop):
        st.pop()
        return

    # Arithmetic / logic --------------------------------------------------
    if isinstance(ins, BinaryOp):
        right = st.pop()
        left = st.pop()
        st.push(apply_binary(ins.op, left, right))
        return
    if isinstance(ins, UnaryOp):
        st.push(apply_unary(ins.op, st.pop()))
        return
    if isinstance(ins, IsNull):
        st.push(st.pop() is None)
        return
    if isinstance(ins, IsNotNull):
        st.push(st.pop() is not None)
        return
    if isinstance(ins, Between):
        _do_between(st)
        return
    if isinstance(ins, InList):
        _do_in_list(ins, st)
        return
    if isinstance(ins, Like):
        _do_like(ins, st)
        return
    if isinstance(ins, Coalesce):
        _do_coalesce(ins, st)
        return

    # Scans ----------------------------------------------------------------
    if isinstance(ins, OpenScan):
        _do_open(ins, st)
        return
    if isinstance(ins, AdvanceCursor):
        _do_advance(ins, st)
        return
    if isinstance(ins, CloseScan):
        _do_close(ins, st)
        return
    if isinstance(ins, ScanAllColumns):
        _do_scan_all_columns(ins, st)
        return

    # Row emission ---------------------------------------------------------
    if isinstance(ins, BeginRow):
        st.row_buffer.clear()
        return
    if isinstance(ins, EmitColumn):
        st.row_buffer[ins.name] = st.pop()
        return
    if isinstance(ins, EmitRow):
        row = tuple(st.row_buffer.get(c) for c in st.result.columns)
        st.result.rows.append(row)
        st.row_buffer.clear()
        return
    if isinstance(ins, SetResultSchema):
        st.result.columns = ins.columns
        return

    # Aggregates ----------------------------------------------------------
    if isinstance(ins, InitAgg):
        _do_init_agg(ins, st)
        return
    if isinstance(ins, UpdateAgg):
        _do_update_agg(ins, st)
        return
    if isinstance(ins, FinalizeAgg):
        _do_finalize_agg(ins, st)
        return
    if isinstance(ins, SaveGroupKey):
        values = st.pop_n(ins.n)
        st.group_key = tuple(values)
        return
    if isinstance(ins, LoadGroupKey):
        st.push(st.group_key[ins.i])
        return
    if isinstance(ins, AdvanceGroupKey):
        st.group_iter += 1
        if st.group_iter >= len(st.group_order):
            st.pc = _resolve(st, ins.on_exhausted)
        else:
            st.group_key = st.group_order[st.group_iter]
        return

    # Post-processing -----------------------------------------------------
    if isinstance(ins, SortResult):
        _do_sort(ins, st)
        return
    if isinstance(ins, LimitResult):
        _do_limit(ins, st)
        return
    if isinstance(ins, DistinctResult):
        _do_distinct(st)
        return

    # DML / DDL -----------------------------------------------------------
    if isinstance(ins, InsertRow):
        _do_insert(ins, st)
        return
    if isinstance(ins, UpdateRows):
        _do_update(ins, st)
        return
    if isinstance(ins, DeleteRows):
        _do_delete(ins, st)
        return
    if isinstance(ins, CreateTable):
        _do_create_table(ins, st)
        return
    if isinstance(ins, DropTable):
        _do_drop_table(ins, st)
        return

    # Control flow --------------------------------------------------------
    if isinstance(ins, Label):
        return  # runtime no-op
    if isinstance(ins, Jump):
        st.pc = _resolve(st, ins.label)
        return
    if isinstance(ins, JumpIfFalse):
        v = st.pop()
        if v is False or v is None:
            st.pc = _resolve(st, ins.label)
        return
    if isinstance(ins, JumpIfTrue):
        v = st.pop()
        if v is True:
            st.pc = _resolve(st, ins.label)
        return

    raise InternalError(message=f"unknown instruction: {type(ins).__name__}")


# --------------------------------------------------------------------------
# Individual instruction implementations. Kept out of _dispatch to keep the
# big match readable.
# --------------------------------------------------------------------------


def _resolve(st: _VmState, label: str) -> int:
    idx = st.program.labels.get(label)
    if idx is None:
        raise InvalidLabel(label=label)
    return idx


def _load_column(ins: LoadColumn, st: _VmState) -> None:
    row = st.current_row.get(ins.cursor_id)
    if row is None:
        st.push(None)
        return
    st.push(row.get(ins.column))


def _do_between(st: _VmState) -> None:
    high = st.pop()
    low = st.pop()
    value = st.pop()
    # NULL propagation: any NULL input yields NULL.
    if value is None or low is None or high is None:
        st.push(None)
        return
    from sql_codegen import BinaryOpCode

    ge = apply_binary(BinaryOpCode.GTE, value, low)
    le = apply_binary(BinaryOpCode.LTE, value, high)
    # 3VL AND: NULL only if no FALSE seen.
    if ge is False or le is False:
        st.push(False)
        return
    if ge is None or le is None:
        st.push(None)
        return
    st.push(True)


def _do_in_list(ins: InList, st: _VmState) -> None:
    values = st.pop_n(ins.n)
    value = st.pop()
    if value is None:
        st.push(None)
        return
    found_null = False
    for item in values:
        if item is None:
            found_null = True
            continue
        if item == value and isinstance(item, bool) == isinstance(value, bool):
            st.push(True)
            return
    st.push(None if found_null else False)


def _do_like(ins: Like, st: _VmState) -> None:
    pattern = st.pop()
    value = st.pop()
    if value is None or pattern is None:
        st.push(None)
        return
    if not isinstance(value, str) or not isinstance(pattern, str):
        from .errors import TypeMismatch

        raise TypeMismatch(
            expected="text",
            got=f"{sql_type_name(value)}, {sql_type_name(pattern)}",
            context="Like",
        )
    matched = like_match(value, pattern)
    st.push(not matched if ins.negated else matched)


def _do_coalesce(ins: Coalesce, st: _VmState) -> None:
    # Pop n values; earliest arg is first in push-order. The spec says return
    # the first non-NULL (leftmost). ``pop_n`` returns in push order.
    values = st.pop_n(ins.n)
    for v in values:
        if v is not None:
            st.push(v)
            return
    st.push(None)


def _do_open(ins: OpenScan, st: _VmState) -> None:
    # Prefer a positioned cursor when the backend offers one — UPDATE and
    # DELETE paths need ``current_row`` semantics. Falling back to ``scan``
    # keeps us correct for read-only backends. This duck-typing check keeps
    # the VM uncoupled from any specific backend class.
    opener = getattr(st.backend, "_open_cursor", None)
    try:
        it = opener(ins.table) if opener is not None else st.backend.scan(ins.table)
    except be.TableNotFound as e:
        raise TableNotFound(table=ins.table) from e
    except be.BackendError as e:
        raise BackendError(message=str(e), original=e) from e
    st.cursors[ins.cursor_id] = it  # type: ignore[assignment]


def _do_advance(ins: AdvanceCursor, st: _VmState) -> None:
    cursor = st.cursors.get(ins.cursor_id)
    if cursor is None:
        raise InternalError(message=f"advance of unknown cursor {ins.cursor_id}")
    row = cursor.next()
    if row is None:
        st.current_row.pop(ins.cursor_id, None)
        st.pc = _resolve(st, ins.on_exhausted)
    else:
        st.current_row[ins.cursor_id] = row


def _do_close(ins: CloseScan, st: _VmState) -> None:
    cursor = st.cursors.pop(ins.cursor_id, None)
    if cursor is not None:
        cursor.close()
    st.current_row.pop(ins.cursor_id, None)


def _do_scan_all_columns(ins: ScanAllColumns, st: _VmState) -> None:
    # Copy every column from the cursor's current row into the row buffer.
    row = st.current_row.get(ins.cursor_id)
    if row is None:
        return
    for name, value in row.items():
        st.row_buffer[name] = value
    # Ensure the schema covers every column we just dumped — if the outer
    # query had no explicit schema, derive one from the row's keys in order.
    if not st.result.columns:
        st.result.columns = tuple(row.keys())


# --------------------------------------------------------------------------
# Aggregates.
# --------------------------------------------------------------------------


def _ensure_group(st: _VmState) -> list[_AggState]:
    if st.group_key not in st.agg_table:
        st.agg_table[st.group_key] = []
        st.group_order.append(st.group_key)
    return st.agg_table[st.group_key]


def _do_init_agg(ins: InitAgg, st: _VmState) -> None:
    # Idempotent: the codegen emits InitAgg once per input row, but the
    # semantic meaning is "ensure this slot exists for this group". Only
    # allocate on first encounter; on subsequent calls the existing
    # accumulator is preserved. MIN/MAX/SUM start at NULL; AVG tracks
    # sum and count.
    slots = _ensure_group(st)
    while len(slots) <= ins.slot:
        slots.append(_AggState(func=ins.func))


def _do_update_agg(ins: UpdateAgg, st: _VmState) -> None:
    value = st.pop()
    slots = _ensure_group(st)
    if ins.slot >= len(slots):
        raise InternalError(message=f"update_agg: slot {ins.slot} not initialized")
    agg = slots[ins.slot]
    if agg.func is AggFunc.COUNT_STAR:
        agg.count += 1
        return
    if value is None:
        return  # SQL: NULL inputs ignored for everything except COUNT(*)
    if agg.func is AggFunc.COUNT:
        agg.count += 1
        return
    if agg.func is AggFunc.SUM:
        agg.acc = value if agg.acc is None else agg.acc + value  # type: ignore[operator]
        return
    if agg.func is AggFunc.AVG:
        agg.acc = value if agg.acc is None else agg.acc + value  # type: ignore[operator]
        agg.count += 1
        return
    if agg.func is AggFunc.MIN:
        if agg.acc is None or value < agg.acc:  # type: ignore[operator]
            agg.acc = value
        return
    if agg.func is AggFunc.MAX:
        if agg.acc is None or value > agg.acc:  # type: ignore[operator]
            agg.acc = value
        return


def _do_finalize_agg(ins: FinalizeAgg, st: _VmState) -> None:
    slots = _ensure_group(st)
    if ins.slot >= len(slots):
        raise InternalError(message=f"finalize_agg: slot {ins.slot} not initialized")
    agg = slots[ins.slot]
    if agg.func in (AggFunc.COUNT, AggFunc.COUNT_STAR):
        st.push(agg.count)
        return
    if agg.func is AggFunc.AVG:
        if agg.count == 0:
            st.push(None)
            return
        st.push(agg.acc / agg.count)  # type: ignore[operator]
        return
    # SUM / MIN / MAX — the accumulator *is* the result (may be NULL for empty).
    st.push(agg.acc)


# --------------------------------------------------------------------------
# Post-processing. These mutate the result buffer directly.
# --------------------------------------------------------------------------


def _null_lt(a: SqlValue, b: SqlValue, nulls: NullsOrder) -> int:
    """Return -1/0/+1 for sort ordering, respecting NULL placement.

    NULLs FIRST → NULL < anything; LAST → NULL > anything. Equal NULLs → 0.
    """
    if a is None and b is None:
        return 0
    if a is None:
        return -1 if nulls is NullsOrder.FIRST else 1
    if b is None:
        return 1 if nulls is NullsOrder.FIRST else -1
    if a < b:  # type: ignore[operator]
        return -1
    if a > b:  # type: ignore[operator]
        return 1
    return 0


def _do_sort(ins: SortResult, st: _VmState) -> None:
    columns = st.result.columns

    def key_fn(row: tuple[SqlValue, ...]) -> tuple[object, ...]:
        # We compose a sortable tuple by wrapping each key-value in a
        # (rank, value) pair where ``rank`` handles NULL placement and the
        # Direction. Python's default tuple comparison does the rest.
        out: list[object] = []
        for k in ins.keys:
            idx = columns.index(k.column)
            v = row[idx]
            is_null = v is None
            # NULLs first/last encoded as 0 / 2 around non-null = 1.
            rank = (
                (0 if is_null else 1)
                if k.nulls is NullsOrder.FIRST
                else (2 if is_null else 1)
            )
            if k.direction is Direction.DESC:
                # Invert: larger values sort first. Trick: use _Reversed wrappers.
                out.append((-rank, _Rev(v)))
            else:
                out.append((rank, _NoneLast(v)))
        return tuple(out)

    st.result.rows.sort(key=key_fn)


class _Rev:
    """Helper: reversed ordering for DESC sort. ``_Rev(x) < _Rev(y)`` iff ``x > y``."""

    __slots__ = ("v",)

    def __init__(self, v: SqlValue) -> None:
        self.v = v

    def __lt__(self, other: _Rev) -> bool:
        if self.v is None or other.v is None:
            return False
        return self.v > other.v  # type: ignore[operator]

    def __eq__(self, other: object) -> bool:
        return isinstance(other, _Rev) and self.v == other.v


class _NoneLast:
    """Helper: treat None as greater than everything else for ASC sort."""

    __slots__ = ("v",)

    def __init__(self, v: SqlValue) -> None:
        self.v = v

    def __lt__(self, other: _NoneLast) -> bool:
        if self.v is None:
            return False
        if other.v is None:
            return True
        return self.v < other.v  # type: ignore[operator]

    def __eq__(self, other: object) -> bool:
        return isinstance(other, _NoneLast) and self.v == other.v


def _do_limit(ins: LimitResult, st: _VmState) -> None:
    start = ins.offset or 0
    if ins.count is None:
        st.result.rows = st.result.rows[start:]
    else:
        st.result.rows = st.result.rows[start : start + ins.count]


def _do_distinct(st: _VmState) -> None:
    seen: set[tuple[SqlValue, ...]] = set()
    out: list[tuple[SqlValue, ...]] = []
    for row in st.result.rows:
        if row not in seen:
            seen.add(row)
            out.append(row)
    st.result.rows = out


# --------------------------------------------------------------------------
# DML + DDL.
# --------------------------------------------------------------------------


def _translate_backend_error(e: be.BackendError) -> Exception:
    if isinstance(e, be.TableNotFound):
        return TableNotFound(table=e.table)
    if isinstance(e, be.TableAlreadyExists):
        return TableAlreadyExists(table=e.table)
    if isinstance(e, be.ColumnNotFound):
        return ColumnNotFound(cursor_id=-1, column=e.column)
    if isinstance(e, be.ConstraintViolation):
        return ConstraintViolation(table=e.table, column=e.column, message=e.message)
    return BackendError(message=str(e), original=e)


def _do_insert(ins: InsertRow, st: _VmState) -> None:
    values = st.pop_n(len(ins.columns))
    row = dict(zip(ins.columns, values, strict=True))
    try:
        st.backend.insert(ins.table, row)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    st.result.rows_affected = (st.result.rows_affected or 0) + 1


def _do_update(ins: UpdateRows, st: _VmState) -> None:
    values = st.pop_n(len(ins.assignments))
    assignments = dict(zip(ins.assignments, values, strict=True))
    cursor = st.cursors.get(ins.cursor_id)
    if cursor is None:
        raise InternalError(message=f"update: cursor {ins.cursor_id} not open")
    try:
        st.backend.update(ins.table, cursor, assignments)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    # Keep the local current_row in sync so subsequent LoadColumn in the loop
    # sees the updated values.
    if ins.cursor_id in st.current_row:
        st.current_row[ins.cursor_id].update(assignments)
    st.result.rows_affected = (st.result.rows_affected or 0) + 1


def _do_delete(ins: DeleteRows, st: _VmState) -> None:
    cursor = st.cursors.get(ins.cursor_id)
    if cursor is None:
        raise InternalError(message=f"delete: cursor {ins.cursor_id} not open")
    try:
        st.backend.delete(ins.table, cursor)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    st.current_row.pop(ins.cursor_id, None)
    st.result.rows_affected = (st.result.rows_affected or 0) + 1


def _do_create_table(ins: CreateTable, st: _VmState) -> None:
    from sql_backend.schema import ColumnDef as BackendColumnDef

    col_defs = [
        BackendColumnDef(name=c.name, type_name=c.type, not_null=not c.nullable)
        for c in ins.columns
    ]
    try:
        st.backend.create_table(ins.table, col_defs, ins.if_not_exists)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    st.result.rows_affected = 0


def _do_drop_table(ins: DropTable, st: _VmState) -> None:
    try:
        st.backend.drop_table(ins.table, ins.if_exists)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    st.result.rows_affected = 0
