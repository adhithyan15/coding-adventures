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
from sql_backend.schema import TriggerDef as BackendTriggerDef
from sql_backend.values import SqlValue, sql_type_name
from sql_codegen import (
    CHECK_CURSOR_ID,
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
    ComputeWindowFunctions,
    CreateIndex,
    CreateTable,
    CreateTriggerDef,
    DeleteRows,
    Direction,
    DistinctResult,
    DropIndex,
    DropTable,
    DropTriggerDef,
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
    JoinBeginRow,
    JoinIfMatched,
    JoinSetMatched,
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    Label,
    Like,
    LimitResult,
    LoadColumn,
    LoadConst,
    LoadGroupKey,
    LoadLastInsertedColumn,
    LoadOuterColumn,
    NullsOrder,
    OpenIndexScan,
    OpenScan,
    OpenWorkingSetScan,
    Pop,
    Program,
    RollbackTransaction,
    RunExistsSubquery,
    RunInSubquery,
    RunRecursiveCTE,
    RunScalarSubquery,
    RunSubquery,
    SaveGroupKey,
    ScanAllColumns,
    SetResultSchema,
    SortResult,
    UnaryOp,
    UpdateAgg,
    UpdateRows,
    WinFunc,
)
from sql_codegen import IrAggFunc as AggFunc

from .errors import (
    BackendError,
    CardinalityError,
    ColumnAlreadyExists,
    ColumnNotFound,
    ConstraintViolation,
    InternalError,
    InvalidLabel,
    StackUnderflow,
    TableAlreadyExists,
    TableNotFound,
    TransactionError,
    TriggerDepthError,
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
    # Per-table CHECK constraint registry: table → [(col_name, instrs)].
    # Populated at CreateTable time; consulted before every INSERT/UPDATE.
    check_registry: dict[str, list[tuple[str, tuple[Instruction, ...]]]] = field(
        default_factory=dict
    )
    # FOREIGN KEY registries — both populated at CreateTable time.
    # fk_child: child_table → [(child_col, parent_table, parent_col_or_None)]
    # fk_parent: parent_table → [(child_table, child_col, parent_col_or_None)]
    # parent_col=None means "the parent's PRIMARY KEY column".
    fk_child: dict[str, list[tuple[str, str, str | None]]] = field(default_factory=dict)
    fk_parent: dict[str, list[tuple[str, str, str | None]]] = field(default_factory=dict)
    # Working-set rows for recursive CTEs.  Populated by _execute_with_cursors
    # before running the recursive sub-program; read by OpenWorkingSetScan to
    # create a fresh _SubqueryCursor on each loop entry (handles JOIN context).
    working_set_data: list[dict[str, SqlValue]] = field(default_factory=list)
    # Trigger executor callback.  When provided, the DML handlers call this
    # for each trigger that should fire.  The callable signature is:
    #   trigger_executor(defn, new_row, old_row, current_depth) -> None
    # where defn is a TriggerDef, new_row/old_row are dicts or None, and
    # current_depth is the nesting level of the current VM invocation.
    # The executor is responsible for the depth check (raises TriggerDepthError
    # when current_depth + 1 > 16) and for setting up NEW/OLD pseudo-tables.
    trigger_executor: Callable | None = None
    # Nesting depth of the current VM invocation within trigger bodies.
    # 0 = top-level statement; > 0 = inside a trigger body.
    trigger_depth: int = 0
    # User-registered scalar functions: lower-cased name → (nargs, callable).
    # nargs=-1 means variadic.  Checked before the built-in registry so users
    # can override built-ins (e.g. to shim behaviour in tests).
    user_functions: dict[str, tuple[int, Callable]] | None = None
    # Outer-join match tracking.  Each JoinBeginRow pushes False; JoinSetMatched
    # sets the top to True; JoinIfMatched pops and conditionally jumps.
    # The stack depth equals the number of currently-open outer-join levels.
    join_match_stack: list[bool] = field(default_factory=list)
    # INSERT … RETURNING support — the most recently inserted row is saved here
    # by ``_do_insert`` so that ``LoadLastInsertedColumn`` can read it back.
    # Keyed by column name; empty dict when no INSERT has been executed yet.
    last_inserted_row: dict[str, SqlValue] = field(default_factory=dict)
    # Correlated subquery support — a snapshot of the enclosing query's
    # ``current_row`` at the time the sub-program was launched.  Used by
    # ``LoadOuterColumn`` to read values from the outer scan without sharing
    # any mutable state with the inner execution.  ``None`` (the default)
    # means this program is the top-level query (not a correlated sub-program).
    outer_current_row: dict[int, dict[str, SqlValue]] = field(default_factory=dict)
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
    check_registry: dict[str, list[tuple[str, tuple[Instruction, ...]]]] | None = None,
    fk_child: dict[str, list[tuple[str, str, str | None]]] | None = None,
    fk_parent: dict[str, list[tuple[str, str, str | None]]] | None = None,
    event_cb: Callable[[QueryEvent], None] | None = None,
    filtered_columns: list[str] | None = None,
    trigger_executor: Callable | None = None,
    trigger_depth: int = 0,
    user_functions: dict[str, tuple[int, Callable]] | None = None,
    outer_current_row: dict[int, dict[str, SqlValue]] | None = None,
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

    ``user_functions``:
        Optional dict mapping lower-cased SQL function names to
        ``(nargs, callable)`` pairs registered via
        :meth:`~mini_sqlite.Connection.create_function`.  Checked before the
        built-in scalar registry so users can shadow built-in functions.

    ``outer_current_row``:
        Optional snapshot of the enclosing query's ``current_row`` table —
        ``{cursor_id: {col: value, …}, …}``.  Provided only when this program
        is a correlated sub-program; ``LoadOuterColumn`` reads from it.
        ``None`` for top-level programs (the default).
    """
    import time

    _t0 = time.perf_counter()
    # Use the caller-supplied registries so constraints registered by a prior
    # CREATE TABLE statement persist across execute() calls.
    registry: dict[str, list[tuple[str, tuple[Instruction, ...]]]] = (
        check_registry if check_registry is not None else {}
    )
    fk_c: dict[str, list[tuple[str, str, str | None]]] = fk_child if fk_child is not None else {}
    fk_p: dict[str, list[tuple[str, str, str | None]]] = fk_parent if fk_parent is not None else {}
    state = _VmState(
        program=program,
        backend=backend,
        check_registry=registry,
        fk_child=fk_c,
        fk_parent=fk_p,
        trigger_executor=trigger_executor,
        trigger_depth=trigger_depth,
        user_functions=user_functions,
        outer_current_row=outer_current_row if outer_current_row is not None else {},
    )
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
    if isinstance(ins, LoadOuterColumn):
        _load_outer_column(ins, st)
        return
    if isinstance(ins, LoadLastInsertedColumn):
        st.push(st.last_inserted_row.get(ins.col))
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
    if isinstance(ins, ComputeWindowFunctions):
        _do_compute_window(ins, st)
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
    if isinstance(ins, CreateTriggerDef):
        _do_create_trigger(ins, st)
        return
    if isinstance(ins, DropTriggerDef):
        _do_drop_trigger(ins, st)
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
    if isinstance(ins, RunScalarSubquery):
        _do_run_scalar_subquery(ins, st)
        return
    if isinstance(ins, RunInSubquery):
        _do_run_in_subquery(ins, st)
        return
    if isinstance(ins, RunRecursiveCTE):
        _do_run_recursive_cte(ins, st)
        return
    if isinstance(ins, OpenWorkingSetScan):
        st.cursors[ins.cursor_id] = _SubqueryCursor(rows=st.working_set_data)
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

    # Outer-join match tracking -------------------------------------------
    if isinstance(ins, JoinBeginRow):
        st.join_match_stack.append(False)
        return
    if isinstance(ins, JoinSetMatched):
        if st.join_match_stack:
            st.join_match_stack[-1] = True
        return
    if isinstance(ins, JoinIfMatched):
        matched = st.join_match_stack.pop() if st.join_match_stack else False
        if matched:
            st.pc = _resolve(st, ins.label)
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


def _load_outer_column(ins: LoadOuterColumn, st: _VmState) -> None:
    """Push a column value from the outer query's current row snapshot.

    ``st.outer_current_row`` is a frozen copy of the enclosing scan's
    ``current_row`` at the time the inner sub-program was invoked.

    If the outer cursor ID is not present (e.g. the subquery was invoked
    from an uncorrelated context — which should not happen if the planner and
    codegen are correct), we push ``None`` as a safe fallback.
    """
    row = st.outer_current_row.get(ins.cursor_id)
    if row is None:
        st.push(None)
        return
    st.push(row.get(ins.col))


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
    # User-registered functions take precedence over built-ins, allowing
    # callers to shadow or extend the function set at the connection level.
    if st.user_functions is not None:
        entry = st.user_functions.get(ins.func.lower())
        if entry is not None:
            nargs, fn = entry
            if nargs != -1 and nargs != len(args):
                from .errors import WrongNumberOfArguments
                raise WrongNumberOfArguments(
                    name=ins.func, expected=str(nargs), got=len(args)
                )
            st.push(fn(*args))
            return
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

    Correlated subqueries
    ---------------------
    If the inner program contains ``LoadOuterColumn`` instructions, it needs
    the outer scan's current row.  We pass ``st.current_row`` as the inner
    program's ``outer_current_row`` snapshot so those reads resolve correctly.
    The inner execution gets its own isolated ``_VmState``; sharing
    ``current_row`` as a **dict reference** (not a deep copy) is safe because
    the inner program does not modify outer cursor rows.
    """
    sub_result = execute(ins.sub_program, st.backend, outer_current_row=st.current_row)
    st.push(len(sub_result.rows) > 0)


def _do_run_scalar_subquery(ins: RunScalarSubquery, st: _VmState) -> None:
    """Execute the scalar sub-program and push the single result value.

    Runs the inner program against the same backend as the outer query.

    - Zero rows: ``NULL`` is pushed.
    - One row: the first column's value is pushed.
    - Two or more rows: :class:`~sql_vm.errors.CardinalityError` is raised.

    The inner program always returns exactly one column — the optimizer
    and codegen ensure this at compile time.

    Correlated subqueries: passes ``st.current_row`` as ``outer_current_row``
    so ``LoadOuterColumn`` instructions in the inner program resolve correctly.
    """
    sub_result = execute(ins.sub_program, st.backend, outer_current_row=st.current_row)
    rows = sub_result.rows
    if len(rows) == 0:
        st.push(None)
    elif len(rows) > 1:
        raise CardinalityError()
    else:
        st.push(rows[0][0] if rows[0] else None)


def _do_run_in_subquery(ins: RunInSubquery, st: _VmState) -> None:
    """Execute the IN-subquery and push a boolean (or NULL).

    Pops the test value from the top of the stack, executes the sub-program
    to materialise the result set, then tests membership.

    NULL semantics (SQL three-valued logic):
    - test_value is None  → push None (NULL IN ... = NULL)
    - test_value found in non-NULL members → push True (or False if negate)
    - test_value not found, result set contains NULL → push None (unknown)
    - test_value not found, no NULL in set → push False (or True if negate)

    Correlated subqueries: passes ``st.current_row`` as ``outer_current_row``
    so ``LoadOuterColumn`` instructions in the inner program resolve correctly.
    The inner program is re-executed once per outer row; each time it reads
    the correlated column values from the outer cursor's current row.
    """
    test_value = st.pop()
    if test_value is None:
        st.push(None)
        return
    sub_result = execute(ins.sub_program, st.backend, outer_current_row=st.current_row)
    # Build two sets: one of non-NULL first-column values, and a flag for NULL presence.
    non_null_values: set[object] = set()
    has_null = False
    for row in sub_result.rows:
        val = row[0] if row else None
        if val is None:
            has_null = True
        else:
            non_null_values.add(val)
    if test_value in non_null_values:
        # Definite match.
        st.push(ins.negate is False)
    elif has_null:
        # No match found, but NULL is in the set — result is UNKNOWN (NULL).
        st.push(None)
    else:
        # Definite non-match.
        st.push(ins.negate is True)


def _execute_with_cursors(
    program: Program,
    backend: Backend,
    working_set_rows: list[dict[str, SqlValue]],
) -> QueryResult:
    """Execute ``program`` with a pre-loaded working set.

    Used by the recursive CTE handler to supply the current working set before
    running the recursive step.  Rather than pre-populating a cursor (which
    would be exhausted after the first JOIN outer-loop iteration), the rows are
    stored in ``VmState.working_set_data``.  The compiled
    ``OpenWorkingSetScan`` instruction then creates a brand-new
    :class:`_SubqueryCursor` from that data on every entry into the
    WorkingSetScan loop, so even a JOIN-based recursive step works correctly.

    A fresh :class:`_VmState` is created so that the recursive program's stack,
    agg_table, and result buffer are isolated from the caller's state.
    """
    state = _VmState(program=program, backend=backend)
    state.working_set_data = working_set_rows
    instructions = program.instructions
    n = len(instructions)
    while state.pc < n:
        ins = instructions[state.pc]
        state.pc += 1
        if isinstance(ins, Halt):
            break
        _dispatch(ins, state)
    return state.result.freeze()


def _do_run_recursive_cte(ins: RunRecursiveCTE, st: _VmState) -> None:
    """Execute anchor, then iterate the recursive step until a fixed point.

    The anchor runs once to produce the initial working set.  The recursive
    step then runs in a loop, with cursor ``working_cursor_id`` pre-loaded
    with the current working set rows.  Each iteration's output becomes the
    next working set.  The loop stops when the recursive step returns zero
    new rows (fixed-point / empty step).

    For UNION ALL: all rows from every iteration are accumulated.
    For UNION: duplicate rows (compared as sorted key-value tuples) are
    discarded, which also prevents infinite loops in cyclic graphs.

    The accumulated rows are materialised as a :class:`_SubqueryCursor` under
    ``cursor_id`` so the outer scan loop's ``AdvanceCursor`` / ``LoadColumn`` /
    ``CloseScan`` instructions work without any special casing.
    """
    # --- Anchor phase -------------------------------------------------------
    anchor_result = execute(ins.anchor_program, st.backend)
    anchor_cols = anchor_result.columns

    working_rows: list[dict[str, SqlValue]] = [
        dict(zip(anchor_cols, row, strict=False)) for row in anchor_result.rows
    ]
    all_rows: list[dict[str, SqlValue]] = list(working_rows)

    # For UNION (deduplicated): track seen rows to prevent cycles.
    seen: set[tuple[tuple[str, SqlValue], ...]] = set()
    if not ins.union_all:
        for row in all_rows:
            seen.add(tuple(sorted(row.items())))

    # --- Recursive phase (fixed-point iteration) ----------------------------
    while working_rows:
        recursive_result = _execute_with_cursors(
            ins.recursive_program,
            st.backend,
            working_rows,
        )
        # Relabel with anchor column names (SQL standard: UNION output names
        # from the leftmost / anchor SELECT).
        new_rows: list[dict[str, SqlValue]] = [
            dict(zip(anchor_cols, row, strict=False)) for row in recursive_result.rows
        ]

        if ins.union_all:
            working_rows = new_rows
        else:
            # Only keep rows not already seen (cycle safety for UNION).
            next_working: list[dict[str, SqlValue]] = []
            for row in new_rows:
                key = tuple(sorted(row.items()))
                if key not in seen:
                    seen.add(key)
                    next_working.append(row)
            working_rows = next_working

        all_rows.extend(working_rows)

    # Materialise accumulated result as a cursor for the outer scan loop.
    st.cursors[ins.cursor_id] = _SubqueryCursor(rows=all_rows)


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


def _do_compute_window(ins: ComputeWindowFunctions, st: _VmState) -> None:
    """Evaluate all window functions against the materialised result buffer.

    Algorithm
    ---------
    1. Convert each result tuple to a ``dict[str, SqlValue]`` keyed by the
       current ``result.columns``.
    2. For each :class:`WinFuncSpec`:
       a. Build partitions — a dict mapping a frozen tuple of partition-key
          values to the list of row dicts in that partition.
       b. Sort each partition by the spec's ``order_cols``.
       c. Evaluate the window function across each sorted partition.
       d. Write the result value into each row dict under ``result_col``.
    3. Project each dict to ``ins.output_cols`` and rebuild the rows list.
    4. Update ``result.columns``.

    Partition sort key
    ------------------
    NULL sorts before all other values (SQLite BINARY collation for ORDER BY
    within window frames).  We use the same ``_sql_sort_key`` helper as the
    index scan code so behaviour is consistent.
    """
    columns = st.result.columns

    # Convert tuples → dicts for easy column access.
    rows: list[dict[str, SqlValue]] = [
        dict(zip(columns, row, strict=True))
        for row in st.result.rows
    ]

    for spec in ins.specs:
        # --- Build partitions -------------------------------------------
        partitions: dict[tuple[SqlValue, ...], list[dict[str, SqlValue]]] = {}
        for row in rows:
            pk = tuple(row.get(c) for c in spec.partition_cols)
            if pk not in partitions:
                partitions[pk] = []
            partitions[pk].append(row)

        for partition in partitions.values():
            # --- Sort within partition -----------------------------------
            if spec.order_cols:
                def _sort_key(
                    r: dict[str, SqlValue], cols: tuple[tuple[str, bool], ...]
                ) -> tuple[object, ...]:
                    result_key: list[object] = []
                    for col, desc in cols:
                        v = r.get(col)
                        k = _win_sort_key(v)
                        result_key.append(_Descending(k) if desc else k)
                    return tuple(result_key)

                order_cols = spec.order_cols
                partition.sort(key=lambda r: _sort_key(r, order_cols))

            # --- Evaluate window function --------------------------------
            func = spec.func
            arg_col = spec.arg_col
            result_col = spec.result_col

            if func == WinFunc.ROW_NUMBER:
                for i, row in enumerate(partition, start=1):
                    row[result_col] = i

            elif func == WinFunc.RANK:
                rank = 1
                for i, row in enumerate(partition):
                    if i == 0:
                        row[result_col] = 1
                    else:
                        prev = partition[i - 1]
                        if _order_vals(prev, spec.order_cols) == _order_vals(row, spec.order_cols):
                            row[result_col] = prev[result_col]
                        else:
                            rank = i + 1
                            row[result_col] = rank

            elif func == WinFunc.DENSE_RANK:
                rank = 1
                for i, row in enumerate(partition):
                    if i == 0:
                        row[result_col] = 1
                    else:
                        prev = partition[i - 1]
                        if _order_vals(prev, spec.order_cols) == _order_vals(row, spec.order_cols):
                            row[result_col] = prev[result_col]
                        else:
                            rank += 1
                            row[result_col] = rank

            elif func == WinFunc.SUM:
                total: SqlValue = None
                for row in partition:
                    v = row.get(arg_col) if arg_col else None
                    if v is not None:
                        total = v if total is None else total + v  # type: ignore[operator]
                for row in partition:
                    row[result_col] = total

            elif func == WinFunc.COUNT:
                count = sum(1 for row in partition if arg_col and row.get(arg_col) is not None)
                for row in partition:
                    row[result_col] = count

            elif func == WinFunc.COUNT_STAR:
                count = len(partition)
                for row in partition:
                    row[result_col] = count

            elif func == WinFunc.AVG:
                vals = [
                    row.get(arg_col) for row in partition
                    if arg_col and row.get(arg_col) is not None
                ]
                avg: SqlValue = None
                if vals:
                    s = sum(float(v) for v in vals)  # type: ignore[arg-type]
                    avg = s / len(vals)
                for row in partition:
                    row[result_col] = avg

            elif func == WinFunc.MIN:
                def _min_key(v: SqlValue) -> tuple[int, object]:
                    return _win_sort_key(v)
                vals = [row.get(arg_col) for row in partition if arg_col] if arg_col else []
                non_null = [v for v in vals if v is not None]
                min_val: SqlValue = min(non_null, key=_min_key) if non_null else None  # type: ignore[arg-type]
                for row in partition:
                    row[result_col] = min_val

            elif func == WinFunc.MAX:
                def _max_key(v: SqlValue) -> tuple[int, object]:
                    return _win_sort_key(v)
                vals = [row.get(arg_col) for row in partition if arg_col] if arg_col else []
                non_null = [v for v in vals if v is not None]
                max_val: SqlValue = max(non_null, key=_max_key) if non_null else None  # type: ignore[arg-type]
                for row in partition:
                    row[result_col] = max_val

            elif func == WinFunc.FIRST_VALUE:
                first = partition[0].get(arg_col) if partition and arg_col else None
                for row in partition:
                    row[result_col] = first

            elif func == WinFunc.LAST_VALUE:
                last = partition[-1].get(arg_col) if partition and arg_col else None
                for row in partition:
                    row[result_col] = last

            elif func == WinFunc.LAG:
                # LAG(col, offset=1, default=None): return the value of ``col``
                # from the row that is ``offset`` positions *before* the current
                # row within the partition (ordered by order_cols).  Rows with
                # no preceding peer at that distance return ``default``.
                #
                # extra_args = (offset: int, default: SqlValue)
                # These are normalised to exactly two elements by the codegen
                # compiler, so we can unpack directly.
                offset_val, default_val = spec.extra_args if spec.extra_args else (1, None)
                offset_int = int(offset_val) if offset_val is not None else 1  # type: ignore[arg-type]
                for i, row in enumerate(partition):
                    src_idx = i - offset_int
                    if 0 <= src_idx < len(partition) and arg_col:
                        row[result_col] = partition[src_idx].get(arg_col)
                    else:
                        row[result_col] = default_val

            elif func == WinFunc.LEAD:
                # LEAD(col, offset=1, default=None): mirror of LAG, but looks
                # *ahead* instead of behind.  Row i fetches from row i+offset.
                offset_val, default_val = spec.extra_args if spec.extra_args else (1, None)
                offset_int = int(offset_val) if offset_val is not None else 1  # type: ignore[arg-type]
                for i, row in enumerate(partition):
                    src_idx = i + offset_int
                    if 0 <= src_idx < len(partition) and arg_col:
                        row[result_col] = partition[src_idx].get(arg_col)
                    else:
                        row[result_col] = default_val

            elif func == WinFunc.NTILE:
                # NTILE(n): divide the partition into n approximately equal
                # buckets and assign each row a bucket number 1..n.
                #
                # Distribution rule (matches SQLite and PostgreSQL):
                #   q, r = divmod(len(partition), n)
                # The first r buckets have q+1 rows; the remaining n-r buckets
                # have q rows.  Rows are numbered from 1.
                #
                # extra_args = (n: int,) set by codegen.
                (n_buckets_raw,) = spec.extra_args if spec.extra_args else (1,)
                total = len(partition)
                # Cap n_buckets to the partition size to prevent a DoS where a
                # caller specifies NTILE(1_000_000_000) on a tiny partition,
                # forcing the outer loop to run billions of iterations for no
                # useful work.  SQL semantics are unaffected: if n > N then
                # every row gets its own bucket (1..N) and empty buckets are
                # simply never emitted, which is exactly what capping achieves.
                raw_n = max(1, int(n_buckets_raw))  # type: ignore[arg-type]
                n_buckets = min(raw_n, total) if total > 0 else 1
                q, r = divmod(total, n_buckets)
                # Build a bucket boundary list: bucket k (1-indexed) ends at
                # the index computed below.
                row_idx = 0
                for bucket in range(1, n_buckets + 1):
                    bucket_size = q + (1 if bucket <= r else 0)
                    for _ in range(bucket_size):
                        if row_idx < total:
                            partition[row_idx][result_col] = bucket
                            row_idx += 1

            elif func == WinFunc.PERCENT_RANK:
                # PERCENT_RANK: (rank − 1) / (N − 1) where rank is the RANK()
                # value and N is the partition size.
                # Special case: if N == 1, every row gets 0.0 (division by zero
                # is avoided; the only row is trivially first).
                n = len(partition)
                if n <= 1:
                    for row in partition:
                        row[result_col] = 0.0
                else:
                    # Reuse the RANK computation — two consecutive rows share a
                    # rank when their order-key values are identical.
                    rank = 1
                    for i, row in enumerate(partition):
                        if i == 0:
                            rank = 1
                        else:
                            prev = partition[i - 1]
                            prev_key = _order_vals(prev, spec.order_cols)
                            cur_key = _order_vals(row, spec.order_cols)
                            if prev_key != cur_key:
                                rank = i + 1
                        row[result_col] = (rank - 1) / (n - 1)

            elif func == WinFunc.CUME_DIST:
                # CUME_DIST: (number of rows whose order key ≤ current row's
                # order key) / N.  Two rows with the same order key get the
                # same CUME_DIST value (the *last* position in the tie group).
                #
                # Equivalently: for each row find the *last* row with an equal
                # order key (i.e. the end of the peer group) and compute
                # (end_index + 1) / N.
                n = len(partition)
                i = 0
                while i < n:
                    # Find the end of the current peer group (all rows with
                    # the same order-key values).
                    j = i
                    while j + 1 < n and (
                        _order_vals(partition[j], spec.order_cols)
                        == _order_vals(partition[j + 1], spec.order_cols)
                    ):
                        j += 1
                    # Rows i..j all belong to the same peer group.
                    cd = (j + 1) / n
                    for k in range(i, j + 1):
                        partition[k][result_col] = cd
                    i = j + 1

            elif func == WinFunc.NTH_VALUE:
                # NTH_VALUE(col, n): return the value of ``col`` at the n-th
                # row (1-indexed) of the partition.  Rows before the n-th row
                # also receive that value (the window frame logically grows as
                # each row is processed, but in our materialised model we simply
                # broadcast the n-th value to all rows).  If the partition has
                # fewer than n rows, return NULL.
                #
                # extra_args = (n: int,) — 1-indexed.
                (n_raw,) = spec.extra_args if spec.extra_args else (1,)
                n_idx = int(n_raw) - 1  # type: ignore[arg-type]  # convert to 0-indexed
                if 0 <= n_idx < len(partition) and arg_col:
                    nth_val: SqlValue = partition[n_idx].get(arg_col)
                else:
                    nth_val = None
                for row in partition:
                    row[result_col] = nth_val

    # Project rows to output_cols and rebuild tuples.
    out_cols = ins.output_cols
    st.result.rows = [
        tuple(row.get(c) for c in out_cols)
        for row in rows
    ]
    st.result.columns = out_cols


def _win_sort_key(v: SqlValue) -> tuple[int, object]:
    """Return a sort key for a SQL value using NULL-first ordering.

    Matches the ``_sql_sort_key`` convention used by ``InMemoryBackend``
    for index scans: NULL < numbers < strings < bytes.
    """
    if v is None:
        return (0, b"")
    if isinstance(v, bool):
        return (1, int(v))
    if isinstance(v, (int, float)):
        return (1, v)
    if isinstance(v, str):
        return (2, v)
    if isinstance(v, bytes):
        return (3, v)
    return (4, repr(v))


class _Descending:
    """Wrapper that reverses comparison for descending sort.

    Python's ``sort`` is ascending-only; wrapping a key in this class
    inverts the comparison so the sort behaves as descending.
    """

    __slots__ = ("key",)

    def __init__(self, key: tuple[int, object]) -> None:
        self.key = key

    def __lt__(self, other: _Descending) -> bool:
        return self.key > other.key

    def __le__(self, other: _Descending) -> bool:
        return self.key >= other.key

    def __gt__(self, other: _Descending) -> bool:
        return self.key < other.key

    def __ge__(self, other: _Descending) -> bool:
        return self.key <= other.key

    def __eq__(self, other: object) -> bool:
        return isinstance(other, _Descending) and self.key == other.key


def _order_vals(
    row: dict[str, SqlValue], order_cols: tuple[tuple[str, bool], ...]
) -> tuple[SqlValue, ...]:
    """Extract the ORDER BY key values from a row dict."""
    return tuple(row.get(col) for col, _ in order_cols)


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


def _fire_trigger(
    defn: BackendTriggerDef,
    new_row: dict | None,
    old_row: dict | None,
    st: _VmState,
) -> None:
    """Invoke the trigger executor callback for a single trigger.

    Raises :class:`TriggerDepthError` when the nesting depth would exceed 16.
    When no executor is registered the trigger is silently skipped — this
    allows unit-testing the VM in isolation without a full pipeline.
    """
    if st.trigger_executor is None:
        return
    next_depth = st.trigger_depth + 1
    if next_depth > 16:
        raise TriggerDepthError(trigger_name=defn.name)
    st.trigger_executor(defn, new_row, old_row, next_depth)


def _do_insert(ins: InsertRow, st: _VmState) -> None:
    values = st.pop_n(len(ins.columns))
    row = dict(zip(ins.columns, values, strict=True))
    _check_constraints(ins.table, row, st)
    _check_fk_child(ins.table, row, st)
    # Fire BEFORE INSERT triggers.
    before_triggers = [
        t for t in st.backend.list_triggers(ins.table)
        if t.timing == "BEFORE" and t.event == "INSERT"
    ]
    for defn in before_triggers:
        _fire_trigger(defn, row, None, st)
    try:
        st.backend.insert(ins.table, row)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    # Fire AFTER INSERT triggers.
    after_triggers = [
        t for t in st.backend.list_triggers(ins.table)
        if t.timing == "AFTER" and t.event == "INSERT"
    ]
    for defn in after_triggers:
        _fire_trigger(defn, row, None, st)
    # Save the inserted row so that RETURNING … can read it back via
    # ``LoadLastInsertedColumn``.  We save it after the successful insert so
    # constraint violations leave the previous value intact (and RETURNING
    # would not be reached anyway because an exception is raised).
    st.last_inserted_row = row
    st.result.rows_affected = (st.result.rows_affected or 0) + 1


def _do_update(ins: UpdateRows, st: _VmState) -> None:
    values = st.pop_n(len(ins.assignments))
    assignments = dict(zip(ins.assignments, values, strict=True))
    cursor = st.cursors.get(ins.cursor_id)
    if cursor is None:
        raise InternalError(message=f"update: cursor {ins.cursor_id} not open")
    # Evaluate CHECK and FK constraints against the post-update row.
    # Copy current row so the AFTER trigger sees the pre-update snapshot in old_row,
    # even after st.current_row is mutated below.
    current = dict(st.current_row.get(ins.cursor_id, {}))
    merged = {**current, **assignments}
    _check_constraints(ins.table, merged, st)
    _check_fk_child(ins.table, merged, st)
    # Fire BEFORE UPDATE triggers.
    before_triggers = [
        t for t in st.backend.list_triggers(ins.table)
        if t.timing == "BEFORE" and t.event == "UPDATE"
    ]
    for defn in before_triggers:
        _fire_trigger(defn, merged, current, st)
    try:
        st.backend.update(ins.table, cursor, assignments)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    # Keep the local current_row in sync so subsequent LoadColumn in the loop
    # sees the updated values.
    if ins.cursor_id in st.current_row:
        st.current_row[ins.cursor_id].update(assignments)
    # Fire AFTER UPDATE triggers.
    after_triggers = [
        t for t in st.backend.list_triggers(ins.table)
        if t.timing == "AFTER" and t.event == "UPDATE"
    ]
    for defn in after_triggers:
        _fire_trigger(defn, merged, current, st)
    st.result.rows_affected = (st.result.rows_affected or 0) + 1


def _do_delete(ins: DeleteRows, st: _VmState) -> None:
    cursor = st.cursors.get(ins.cursor_id)
    if cursor is None:
        raise InternalError(message=f"delete: cursor {ins.cursor_id} not open")
    # Check RESTRICT: reject deletion if any child table references this row.
    current = st.current_row.get(ins.cursor_id, {})
    _check_fk_parent(ins.table, current, st)
    # Fire BEFORE DELETE triggers.
    before_triggers = [
        t for t in st.backend.list_triggers(ins.table)
        if t.timing == "BEFORE" and t.event == "DELETE"
    ]
    for defn in before_triggers:
        _fire_trigger(defn, None, current, st)
    try:
        st.backend.delete(ins.table, cursor)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    # Fire AFTER DELETE triggers.
    after_triggers = [
        t for t in st.backend.list_triggers(ins.table)
        if t.timing == "AFTER" and t.event == "DELETE"
    ]
    for defn in after_triggers:
        _fire_trigger(defn, None, current, st)
    st.current_row.pop(ins.cursor_id, None)
    st.result.rows_affected = (st.result.rows_affected or 0) + 1


def _do_create_table(ins: CreateTable, st: _VmState) -> None:
    from sql_backend.schema import ColumnDef as BackendColumnDef

    col_defs = [
        BackendColumnDef(
            name=c.name,
            type_name=c.type,
            not_null=not c.nullable,
            primary_key=c.primary_key,
        )
        for c in ins.columns
    ]
    try:
        st.backend.create_table(ins.table, col_defs, ins.if_not_exists)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    # Register CHECK constraints.
    checks = [(c.name, c.check_instrs) for c in ins.columns if c.check_instrs]
    if checks:
        st.check_registry[ins.table] = checks
    # Register FOREIGN KEY constraints — both child (forward) and parent (reverse).
    for col in ins.columns:
        if col.foreign_key is None:
            continue
        ref_table, ref_col = col.foreign_key
        # Forward: child_table → parent lookups on INSERT/UPDATE
        st.fk_child.setdefault(ins.table, []).append((col.name, ref_table, ref_col))
        # Reverse: parent_table → restrict on DELETE
        st.fk_parent.setdefault(ref_table, []).append((ins.table, col.name, ref_col))
    st.result.rows_affected = 0


def _check_constraints(table: str, row: dict[str, SqlValue], st: _VmState) -> None:
    """Evaluate every CHECK constraint for *table* against *row*.

    The check instructions are evaluated using ``CHECK_CURSOR_ID`` as a
    synthetic cursor so ``LoadColumn`` can resolve column names.  NULL
    result is treated as passing (standard SQL behaviour).  ``False`` raises
    :class:`ConstraintViolation`.
    """
    constraints = st.check_registry.get(table)
    if not constraints:
        return
    st.current_row[CHECK_CURSOR_ID] = row
    try:
        for col_name, instrs in constraints:
            depth_before = len(st.stack)
            for instr in instrs:
                _dispatch(instr, st)
            result = st.pop()
            assert len(st.stack) == depth_before, "CHECK expr left extra values on stack"
            if result is False:
                raise ConstraintViolation(
                    table=table,
                    column=col_name,
                    message=f"CHECK constraint failed: {table}.{col_name}",
                )
    finally:
        st.current_row.pop(CHECK_CURSOR_ID, None)


def _fk_find_pk(table: str, backend: object) -> str:
    """Return the PRIMARY KEY column name for *table*, falling back to 'id'."""
    try:
        cols = backend.columns(table)  # type: ignore[union-attr]
    except Exception:  # noqa: BLE001
        return "id"
    for c in cols:
        if getattr(c, "primary_key", False):
            return c.name
    return "id"


def _fk_row_exists(table: str, col: str, value: object, backend: object) -> bool:
    """Return True if any row in *table* has *col* == *value*."""
    cur = backend.scan(table)  # type: ignore[union-attr]
    try:
        while True:
            row = cur.next()
            if row is None:
                return False
            if row.get(col) == value:
                return True
    finally:
        cur.close()


def _check_fk_child(table: str, row: dict, st: _VmState) -> None:
    """Verify every FOREIGN KEY on the child *table* is satisfied by *row*.

    NULL FK values pass unconditionally (SQL standard: unknown reference is
    not an error).  Non-NULL values must have a matching row in the parent.
    """
    fks = st.fk_child.get(table)
    if not fks:
        return
    for child_col, parent_table, parent_col in fks:
        value = row.get(child_col)
        if value is None:
            continue
        ref_col = parent_col if parent_col is not None else _fk_find_pk(parent_table, st.backend)
        if not _fk_row_exists(parent_table, ref_col, value, st.backend):
            raise ConstraintViolation(
                table=table,
                column=child_col,
                message=(
                    f"FOREIGN KEY constraint failed: "
                    f"{table}.{child_col} → {parent_table}.{ref_col} = {value!r}"
                ),
            )


def _check_fk_parent(table: str, row: dict, st: _VmState) -> None:
    """Enforce RESTRICT on deletion: reject if any child row references *row*.

    Only the columns registered in ``fk_parent`` are checked.  NULL values in
    the parent's referenced column cannot be referenced by any child (because
    child NULL passes unconditionally in :func:`_check_fk_child`), so we skip.
    """
    refs = st.fk_parent.get(table)
    if not refs:
        return
    for child_table, child_col, parent_col in refs:
        ref_col = parent_col if parent_col is not None else _fk_find_pk(table, st.backend)
        value = row.get(ref_col)
        if value is None:
            continue
        if _fk_row_exists(child_table, child_col, value, st.backend):
            raise ConstraintViolation(
                table=table,
                column=ref_col,
                message=(
                    f"FOREIGN KEY constraint failed: "
                    f"cannot delete {table}.{ref_col} = {value!r}, "
                    f"referenced by {child_table}.{child_col}"
                ),
            )


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


def _do_create_trigger(ins: CreateTriggerDef, st: _VmState) -> None:
    """Store a trigger definition in the backend."""
    defn = BackendTriggerDef(
        name=ins.name,
        table=ins.table,
        timing=ins.timing,  # type: ignore[arg-type]
        event=ins.event,    # type: ignore[arg-type]
        body=ins.body_sql,
    )
    try:
        st.backend.create_trigger(defn)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
    st.result.rows_affected = 0


def _do_drop_trigger(ins: DropTriggerDef, st: _VmState) -> None:
    """Remove a trigger definition from the backend."""
    try:
        st.backend.drop_trigger(ins.name, ins.if_exists)
    except be.BackendError as e:
        raise _translate_backend_error(e) from e
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
