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

from collections.abc import Callable
from dataclasses import dataclass, field

import sql_backend.errors as be
from sql_backend.backend import Backend, TransactionHandle
from sql_backend.errors import IndexAlreadyExists, IndexNotFound
from sql_backend.index import IndexDef
from sql_backend.row import RowIterator
from sql_backend.values import SqlValue, sql_type_name
from sql_codegen import (
    AdvanceCursor,
    AdvanceGroupKey,
    AlterTable,
    BeginRow,
    BeginTransaction,
    Between,
    BinaryOp,
    CallScalar,
    CaptureLeftResult,
    CloseScan,
    Coalesce,
    CommitTransaction,
    CreateIndex,
    CreateTable,
    DeleteRows,
    Direction,
    DistinctResult,
    DropIndex,
    DropTable,
    EmitColumn,
    EmitRow,
    ExceptResult,
    FinalizeAgg,
    Halt,
    InitAgg,
    InList,
    InsertFromResult,
    InsertRow,
    Instruction,
    IntersectResult,
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
    OpenIndexScan,
    OpenScan,
    Pop,
    Program,
    RollbackTransaction,
    RunExistsSubquery,
    RunSubquery,
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
    ColumnAlreadyExists,
    ColumnNotFound,
    ConstraintViolation,
    InternalError,
    InvalidLabel,
    StackUnderflow,
    TableAlreadyExists,
    TableNotFound,
    TransactionError,
)
from .operators import apply_binary, apply_unary, like_match
from .result import QueryResult, _MutableResult
from .scalar_functions import call as _call_scalar

# --------------------------------------------------------------------------
# Query telemetry — emitted after each SELECT scan when a listener is set.
# --------------------------------------------------------------------------


@dataclass
class QueryEvent:
    """Telemetry emitted after each SELECT scan.

    Registered listeners receive one ``QueryEvent`` per ``execute()`` call
    that performed at least one table or index scan.  ``duration_us`` is the
    wall-clock time of the entire ``execute()`` call in microseconds.
    """

    table: str
    filtered_columns: list[str]
    rows_scanned: int
    rows_returned: int
    used_index: str | None
    duration_us: int


# Module-level event listener — None means events are suppressed.
_event_listener: Callable[[QueryEvent], None] | None = None


def set_event_listener(
    listener: Callable[[QueryEvent], None] | None,
) -> None:
    """Register a callback to receive :class:`QueryEvent` after each SELECT scan.

    Pass ``None`` to remove the listener.  The callback is called
    synchronously before :func:`execute` returns — it must be fast.
    """
    global _event_listener
    _event_listener = listener


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
# Subquery cursor — an in-memory RowIterator backed by materialised rows.
# --------------------------------------------------------------------------


class _SubqueryCursor:
    """RowIterator over pre-materialised rows from a derived-table sub-query.

    After ``RunSubquery`` executes the inner program and collects result rows,
    it wraps them in one of these so that the outer scan's ``AdvanceCursor`` /
    ``LoadColumn`` / ``CloseScan`` instructions work transparently — they go
    through ``st.cursors`` just like a normal backend scan.

    ``current_row()`` is a no-op stub because subquery rows are read-only:
    the VM never issues UPDATE / DELETE against a derived-table source.
    """

    __slots__ = ("_rows", "_pos", "_closed")

    def __init__(self, rows: list[dict[str, SqlValue]]) -> None:
        self._rows = rows
        self._pos = -1
        self._closed = False

    def next(self) -> dict[str, SqlValue] | None:
        if self._closed:
            return None
        self._pos += 1
        if self._pos >= len(self._rows):
            return None
        return dict(self._rows[self._pos])  # shallow copy — consistent with ListRowIterator

    def close(self) -> None:
        self._closed = True


# --------------------------------------------------------------------------
# VM state. Public API is only the ``execute`` entry point below.
# --------------------------------------------------------------------------


@dataclass(slots=True)
class _VmState:
    program: Program
    backend: Backend
    pc: int = 0
    stack: list[SqlValue] = field(default_factory=list)
    cursors: dict[int, RowIterator] = field(default_factory=dict)
    current_row: dict[int, dict[str, SqlValue]] = field(default_factory=dict)
    row_buffer: list[SqlValue] = field(default_factory=list)
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
    # Left-side result saved by CaptureLeftResult, consumed by
    # IntersectResult / ExceptResult for INTERSECT / EXCEPT set operations.
    left_result: list[tuple[SqlValue, ...]] = field(default_factory=list)
    # Handle returned by backend.begin_transaction(); None when no explicit
    # transaction is active.
    transaction_handle: TransactionHandle | None = None
    # Scan telemetry — populated during execution; used to build QueryEvent.
    # Only the *first* scan per execute() call is recorded (one event per call).
    scan_table: str = ""
    scan_index: str | None = None
    rows_scanned: int = 0
    rows_returned: int = 0

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


def execute(
    program: Program,
    backend: Backend,
    *,
    event_cb: Callable[[QueryEvent], None] | None = None,
    filtered_columns: list[str] | None = None,
) -> QueryResult:
    """Execute ``program`` against ``backend`` and return the result.

    This is the VM's single public entry point.  It creates a fresh
    :class:`_VmState`, runs the dispatch loop, and packages up the result.

    ``event_cb``:
        Optional callback invoked with a :class:`QueryEvent` after execution
        completes, if a scan was performed.  Takes precedence over any
        module-level listener registered via :func:`set_event_listener`.

    ``filtered_columns``:
        Optional list of column names that appeared in the WHERE predicate of
        the executing statement.  Passed through verbatim to the
        :class:`QueryEvent`.  When ``None`` the event carries an empty list.
    """
    import time

    _t0 = time.perf_counter()
    state = _VmState(program=program, backend=backend)
    instructions = program.instructions
    n = len(instructions)
    while state.pc < n:
        ins = instructions[state.pc]
        state.pc += 1
        if isinstance(ins, Halt):
            break
        _dispatch(ins, state)
    result = state.result.freeze()

    # Emit a QueryEvent when a scan was performed and a listener is registered.
    listener = event_cb or _event_listener
    if listener is not None and state.scan_table:
        duration_us = int((time.perf_counter() - _t0) * 1_000_000)
        event = QueryEvent(
            table=state.scan_table,
            filtered_columns=filtered_columns or [],
            rows_scanned=state.rows_scanned,
            rows_returned=state.rows_returned,
            used_index=state.scan_index,
            duration_us=duration_us,
        )
        listener(event)

    return result


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
    if isinstance(ins, CallScalar):
        _do_call_scalar(ins, st)
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
        # Positional append — the column name is used only for schema
        # bookkeeping (result_schema / SetResultSchema), not for row assembly.
        # Using a list instead of a dict correctly handles SELECT lists with
        # duplicate column names (e.g. SELECT da.v, db.v in a CROSS JOIN where
        # both sub-queries expose a column called "v").
        st.row_buffer.append(st.pop())
        return
    if isinstance(ins, EmitRow):
        # The row buffer is positional; just convert to a tuple.
        row = tuple(st.row_buffer)
        st.result.rows.append(row)
        st.row_buffer.clear()
        st.rows_returned += 1
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
    if isinstance(ins, InsertFromResult):
        _do_insert_from_result(ins, st)
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
    if isinstance(ins, CreateIndex):
        _do_create_index(ins, st)
        return
    if isinstance(ins, DropIndex):
        _do_drop_index(ins, st)
        return
    if isinstance(ins, AlterTable):
        _do_alter_table(ins, st)
        return
    if isinstance(ins, OpenIndexScan):
        _do_open_index_scan(ins, st)
        return

    # Set operations -------------------------------------------------------
    if isinstance(ins, CaptureLeftResult):
        _do_capture_left(st)
        return
    if isinstance(ins, IntersectResult):
        _do_intersect(ins, st)
        return
    if isinstance(ins, ExceptResult):
        _do_except(ins, st)
        return

    # Derived-table sub-queries -------------------------------------------
    if isinstance(ins, RunSubquery):
        _do_run_subquery(ins, st)
        return
    if isinstance(ins, RunExistsSubquery):
        _do_run_exists_subquery(ins, st)
        return

    # Transactions --------------------------------------------------------
    if isinstance(ins, BeginTransaction):
        _do_begin_transaction(st)
        return
    if isinstance(ins, CommitTransaction):
        _do_commit_transaction(st)
        return
    if isinstance(ins, RollbackTransaction):
        _do_rollback_transaction(st)
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


def _do_call_scalar(ins: CallScalar, st: _VmState) -> None:
    """Dispatch a scalar function call.

    Arguments are on the stack in push order (leftmost argument was pushed
    first).  ``pop_n`` retrieves them in that same order so the function
    receives its arguments left-to-right.

    ``UnsupportedFunction`` and ``WrongNumberOfArguments`` propagate directly
    to the caller — they are both :class:`VmError` subclasses.

    ``TypeMismatch`` may also propagate when a function validates argument
    types internally (e.g. ``SQRT`` on a non-numeric string).

    Examples (SQL → stack trace):

    ::

        -- ABS(-5)
        LoadConst -5
        CallScalar("abs", 1)    → pushes 5

        -- COALESCE(NULL, 42)
        LoadConst NULL
        LoadConst 42
        CallScalar("coalesce", 2)  → pushes 42
    """
    args = st.pop_n(ins.n_args)
    result = _call_scalar(ins.func, args)
    st.push(result)


def _do_run_subquery(ins: RunSubquery, st: _VmState) -> None:
    """Execute the derived-table sub-program and materialise its rows.

    Runs the inner program against the *same backend* as the outer query (so
    the sub-query sees the same data, transactions, and schema).  The result
    rows are wrapped in a :class:`_SubqueryCursor` and stored under
    ``cursor_id`` in the outer state's cursor table.

    After this instruction, the outer scan loop's ``AdvanceCursor`` /
    ``LoadColumn`` / ``CloseScan`` on the same ``cursor_id`` iterate over
    the materialised rows exactly like a normal backend scan — no special
    casing needed in those paths.

    The inner execution gets a fresh ``_VmState`` so the sub-query's stack,
    agg_table, and result buffer are fully isolated from the outer query's
    state.  The backend is shared (read-only access from the sub-query side
    is safe; the sub-query should not issue DML, but we don't enforce that
    here).
    """
    sub_result = execute(ins.sub_program, st.backend)
    cols = sub_result.columns
    # Convert the result rows (tuples keyed by position) back to dicts so
    # LoadColumn can look up by column name exactly as it does for a normal scan.
    rows: list[dict[str, SqlValue]] = [
        dict(zip(cols, row, strict=False)) for row in sub_result.rows
    ]
    st.cursors[ins.cursor_id] = _SubqueryCursor(rows=rows)


def _do_run_exists_subquery(ins: RunExistsSubquery, st: _VmState) -> None:
    """Execute the EXISTS sub-program and push TRUE iff it returned any rows.

    Runs the inner program against the same backend as the outer query and
    checks the row count.  Unlike :func:`_do_run_subquery`, no cursor is
    opened — the result is a single boolean pushed onto the expression stack.

    ``NOT EXISTS`` is handled by the caller: a :class:`~sql_codegen.UnaryOp`
    ``NOT`` instruction follows this one and inverts the boolean.
    """
    sub_result = execute(ins.sub_program, st.backend)
    st.push(len(sub_result.rows) > 0)


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
    # Record the first scan's table for QueryEvent telemetry.
    if not st.scan_table:
        st.scan_table = ins.table


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
        st.rows_scanned += 1


def _do_close(ins: CloseScan, st: _VmState) -> None:
    cursor = st.cursors.pop(ins.cursor_id, None)
    if cursor is not None:
        cursor.close()
    st.current_row.pop(ins.cursor_id, None)


def _do_scan_all_columns(ins: ScanAllColumns, st: _VmState) -> None:
    """Copy every column from the cursor's current row into the row buffer.

    The row buffer is positional (a list), so we append values in the
    same order as ``row.items()`` — which is insertion order (Python ≥ 3.7).
    The schema (``result.columns``) is derived from ``row.keys()`` in the
    same iteration order, so column positions are guaranteed to match.
    """
    row = st.current_row.get(ins.cursor_id)
    if row is None:
        return
    for value in row.values():
        st.row_buffer.append(value)
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
    if isinstance(e, be.ColumnAlreadyExists):
        return ColumnAlreadyExists(table=e.table, column=e.column)
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


def _do_create_index(ins: CreateIndex, st: _VmState) -> None:
    """Create a named index on the backend.

    ``IndexDef`` wraps all the parameters the backend needs: name, table,
    column list, and the ``unique`` flag. The ``auto`` flag is set to
    ``False`` for user-created indexes (auto-indexes are created by the
    backend itself when it detects a UNIQUE constraint).

    If ``if_not_exists=True`` and the index already exists, the error is
    silently suppressed — the SQL semantics of ``CREATE INDEX IF NOT EXISTS``
    guarantee idempotence. Otherwise, ``IndexAlreadyExists`` propagates.
    """
    idx = IndexDef(
        name=ins.name, table=ins.table, columns=list(ins.columns), unique=ins.unique, auto=False
    )
    try:
        st.backend.create_index(idx)
    except IndexAlreadyExists:
        if not ins.if_not_exists:
            raise
    st.result.rows_affected = 0


def _do_drop_index(ins: DropIndex, st: _VmState) -> None:
    """Drop a named index from the backend.

    The ``if_exists`` flag is forwarded directly to the backend: when
    ``True``, a missing index is silently ignored; when ``False``, the
    backend raises ``IndexNotFound`` which propagates to the caller.
    """
    try:
        st.backend.drop_index(ins.name, if_exists=ins.if_exists)
    except IndexNotFound:
        raise
    st.result.rows_affected = 0


def _do_alter_table(ins: AlterTable, st: _VmState) -> None:
    """Add a column to an existing table via ALTER TABLE … ADD COLUMN."""
    from sql_backend.schema import ColumnDef as BackendColumnDef

    col = BackendColumnDef(
        name=ins.column.name,
        type_name=ins.column.type,
        not_null=not ins.column.nullable,
    )
    try:
        st.backend.add_column(ins.table, col)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    st.result.rows_affected = 0


def _do_open_index_scan(ins: OpenIndexScan, st: _VmState) -> None:
    """Open a cursor backed by an index range scan.

    The VM asks the backend for the set of rowids that fall within the
    requested range, then fetches the full rows for those rowids.  The
    resulting ``RowIterator`` is stored in ``cursors[cursor_id]`` so that
    the normal ``AdvanceCursor`` / ``LoadColumn`` / ``CloseScan``
    instructions work transparently — no special-casing needed downstream.

    IX-8: ``ins.lo`` and ``ins.hi`` are now *tuples* of ``SqlValue``
    (matching the ``OpenIndexScan`` IR field change).  A single-column scan
    uses a 1-tuple; a composite two-column scan uses a 2-tuple.  We convert
    the tuples to lists before passing them to ``backend.scan_index``, whose
    type signature requires ``list[SqlValue] | None``.

    Example (single-column)::

        OpenIndexScan(cursor_id=0, table="orders", index_name="idx_orders_ts",
                      lo=(1_000_000,), hi=(2_000_000,),
                      lo_inclusive=True, hi_inclusive=False)
        # → backend.scan_index("idx_orders_ts", [1_000_000], [2_000_000],
        #                        lo_inclusive=True, hi_inclusive=False)
        # → rowids = [3, 7, 12, ...]
        # → backend.scan_by_rowids("orders", rowids)  →  RowIterator

    Example (composite two-column)::

        OpenIndexScan(cursor_id=0, table="orders", index_name="auto_orders_user_id_status",
                      lo=(1, "shipped"), hi=(1, "shipped"),
                      lo_inclusive=True, hi_inclusive=True)
        # → backend.scan_index("auto_orders_user_id_status", [1, "shipped"], [1, "shipped"],
        #                        lo_inclusive=True, hi_inclusive=True)
    """
    lo_list = list(ins.lo) if ins.lo is not None else None
    hi_list = list(ins.hi) if ins.hi is not None else None
    rowids = list(st.backend.scan_index(
        ins.index_name, lo_list, hi_list,
        lo_inclusive=ins.lo_inclusive, hi_inclusive=ins.hi_inclusive,
    ))
    row_iter = st.backend.scan_by_rowids(ins.table, rowids)
    st.cursors[ins.cursor_id] = row_iter  # type: ignore[assignment]
    # Record scan telemetry for QueryEvent.  An IndexScan overrides any
    # earlier plain OpenScan because it is more specific information.
    st.scan_table = ins.table
    st.scan_index = ins.index_name


# --------------------------------------------------------------------------
# INSERT … SELECT support.
# --------------------------------------------------------------------------


def _do_insert_from_result(ins: InsertFromResult, st: _VmState) -> None:
    """Drain the result buffer, inserting every row into ``ins.table``.

    Column mapping:
    - If ``ins.columns`` is non-empty, it overrides the result schema column
      names for the purpose of building the insert dict.  The cardinality
      must match — each result-column value is mapped to the corresponding
      target column in order.
    - If ``ins.columns`` is empty, we fall back to the result schema column
      names verbatim: ``result.columns[i] → row[i]``.

    After draining, the result buffer is cleared and ``rows_affected`` is
    set to the count of inserted rows.
    """
    schema = st.result.columns
    target_cols = ins.columns if ins.columns else schema
    affected = 0
    for row in st.result.rows:
        row_dict = dict(zip(target_cols, row, strict=False))
        try:
            st.backend.insert(ins.table, row_dict)
        except be.BackendError as e:
            raise _translate_backend_error(e) from e
        affected += 1
    st.result.rows.clear()
    st.result.rows_affected = (st.result.rows_affected or 0) + affected


# --------------------------------------------------------------------------
# Set operations: INTERSECT / EXCEPT.
# --------------------------------------------------------------------------


def _do_capture_left(st: _VmState) -> None:
    """Save the current result rows as the left side of a set operation.

    The result buffer is cleared afterwards so the right side's scan can
    accumulate into it from scratch. The result schema is *not* cleared —
    both sides must share the same schema (standard SQL constraint).
    """
    st.left_result = list(st.result.rows)
    st.result.rows.clear()


def _do_intersect(ins: IntersectResult, st: _VmState) -> None:
    """Compute INTERSECT [ALL] from ``left_result`` and the current result rows.

    INTERSECT (all=False)
    ---------------------
    Return each *distinct* row that appears in both sides.  We use a set of
    the right-side rows as a membership test, then deduplicate the output.

    INTERSECT ALL (all=True)
    ------------------------
    Return rows with multiplicity equal to the *minimum* count in each side.
    We count occurrences on both sides and emit min(left_count, right_count)
    copies for each distinct row.
    """
    left = st.left_result
    right = st.result.rows
    st.left_result = []

    if not ins.all:
        # INTERSECT (set semantics): distinct rows present in both sides.
        right_set = set(right)
        seen: set[tuple[SqlValue, ...]] = set()
        out: list[tuple[SqlValue, ...]] = []
        for row in left:
            if row in right_set and row not in seen:
                seen.add(row)
                out.append(row)
    else:
        # INTERSECT ALL (bag semantics): min multiplicity from each side.
        right_counts: dict[tuple[SqlValue, ...], int] = {}
        for row in right:
            right_counts[row] = right_counts.get(row, 0) + 1
        left_counts: dict[tuple[SqlValue, ...], int] = {}
        for row in left:
            left_counts[row] = left_counts.get(row, 0) + 1
        out = []
        for row, left_cnt in left_counts.items():
            right_cnt = right_counts.get(row, 0)
            out.extend([row] * min(left_cnt, right_cnt))

    st.result.rows = out


def _do_except(ins: ExceptResult, st: _VmState) -> None:
    """Compute EXCEPT [ALL] from ``left_result`` minus the current result rows.

    EXCEPT (all=False)
    ------------------
    Return distinct rows that appear in the left side but *not* in the right
    side.

    EXCEPT ALL (all=True)
    ---------------------
    For each distinct row, subtract the right-side count from the left-side
    count; emit max(0, left_count - right_count) copies.
    """
    left = st.left_result
    right = st.result.rows
    st.left_result = []

    if not ins.all:
        # EXCEPT (set semantics): distinct rows in left not in right.
        right_set = set(right)
        seen: set[tuple[SqlValue, ...]] = set()
        out: list[tuple[SqlValue, ...]] = []
        for row in left:
            if row not in right_set and row not in seen:
                seen.add(row)
                out.append(row)
    else:
        # EXCEPT ALL (bag semantics): left count minus right count.
        right_counts: dict[tuple[SqlValue, ...], int] = {}
        for row in right:
            right_counts[row] = right_counts.get(row, 0) + 1
        out = []
        remaining: dict[tuple[SqlValue, ...], int] = {}
        for row in left:
            remaining[row] = remaining.get(row, 0) + 1
        for row, cnt in remaining.items():
            take = max(0, cnt - right_counts.get(row, 0))
            out.extend([row] * take)

    st.result.rows = out


# --------------------------------------------------------------------------
# Transaction control.
# --------------------------------------------------------------------------


def _do_begin_transaction(st: _VmState) -> None:
    """Open an explicit transaction.

    Nested BEGIN calls are rejected: SQL standard does not allow nested
    transactions without SAVEPOINT. If a transaction is already active we
    raise ``TransactionError`` rather than silently ignoring it.

    Why we also check the backend
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    Each :func:`execute` call creates a fresh ``_VmState`` whose
    ``transaction_handle`` starts as ``None``.  If a user runs
    ``BEGIN`` in one execute call and then ``BEGIN`` again in a later call
    (with no intervening ``COMMIT``/``ROLLBACK``), the per-call check
    ``st.transaction_handle is not None`` would miss the first BEGIN.

    We therefore also consult ``backend.current_transaction()``: if the
    backend already has an active handle, the transaction was started by a
    previous execute call and a nested BEGIN must be rejected.
    """
    # Check both the per-call state and the backend's persistent state.
    if st.transaction_handle is not None or st.backend.current_transaction() is not None:
        raise TransactionError(
            message="cannot BEGIN: a transaction is already active"
        )
    try:
        handle = st.backend.begin_transaction()
    except be.BackendError as e:
        raise BackendError(message=str(e), original=e) from e
    st.transaction_handle = handle


def _do_commit_transaction(st: _VmState) -> None:
    """Commit the active transaction.

    The handle may come from the current VM state (if BEGIN and COMMIT are
    in the same execute call) or from the backend's persistent state (if
    they are in separate calls).  We prefer the per-call handle; if it is
    ``None`` we fall back to ``backend.current_transaction()``.

    Raises ``TransactionError`` if no transaction is active.
    """
    handle = st.transaction_handle or st.backend.current_transaction()
    if handle is None:
        raise TransactionError(
            message="cannot COMMIT: no active transaction"
        )
    st.transaction_handle = None
    try:
        st.backend.commit(handle)
    except be.BackendError as e:
        raise BackendError(message=str(e), original=e) from e


def _do_rollback_transaction(st: _VmState) -> None:
    """Roll back the active transaction.

    Same handle-lookup strategy as :func:`_do_commit_transaction`.

    Raises ``TransactionError`` if no transaction is active.
    """
    handle = st.transaction_handle or st.backend.current_transaction()
    if handle is None:
        raise TransactionError(
            message="cannot ROLLBACK: no active transaction"
        )
    st.transaction_handle = None
    try:
        st.backend.rollback(handle)
    except be.BackendError as e:
        raise BackendError(message=str(e), original=e) from e
