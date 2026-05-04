"""
Plan → IR bytecode compiler.

Design
------

The compiler is a recursive post-order traversal. Each node produces a
sequence of instructions, and parent nodes interleave their code with
the child's output — in particular, plan nodes that emit a loop (Scan)
ask a child ``body_fn`` to produce the per-row body, which the parent
then splices between the loop header and footer.

That is the crux of this compiler: rather than trying to linearize a
whole tree first and stitch control flow later, each plan node owns
the responsibility of calling into its parents' body builders at the
right place in its own emitted sequence.

Two-phase code emission:

1. **Emit** a list of ``Instruction`` values using string labels for
   jump targets. Forward jumps are fine because labels are names.
2. **Resolve** labels — walk the stream once, record each ``Label``'s
   index, and build a ``labels`` dict. ``Jump*`` instructions keep
   their string labels; the VM looks up the index at dispatch time.

Cursor & label IDs
------------------

Cursor IDs and label suffixes come from a single monotonic counter held
on the ``_Ctx`` compilation context. Sharing the counter keeps names
unique even for deeply-nested joins.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field

from sql_planner import (
    Aggregate,
    Begin,
    BinaryExpr,
    CaseExpr,
    Column,
    Commit,
    CorrelatedRef,
    Delete,
    DerivedTable,
    Distinct,
    EmptyResult,
    Except,
    ExistsSubquery,
    Expr,
    Filter,
    FunctionCall,
    Having,
    In,
    IndexScan,
    Insert,
    InSubquery,
    Intersect,
    Join,
    JoinKind,
    Literal,
    LogicalPlan,
    NotIn,
    NotInSubquery,
    Project,
    Rollback,
    ScalarSubquery,
    Scan,
    Sort,
    UnaryExpr,
    Union,
    Update,
    Wildcard,
    WorkingSetScan,
)
from sql_planner import (
    AlterTable as PlanAlterTable,
)
from sql_planner import (
    Between as AstBetween,
)
from sql_planner import (
    BinaryOp as AstBinaryOp,
)
from sql_planner import (
    CreateIndex as PlanCreateIndex,
)
from sql_planner import (
    CreateTable as PlanCreateTable,
)
from sql_planner import (
    CreateTrigger as PlanCreateTrigger,
)
from sql_planner import (
    DropIndex as PlanDropIndex,
)
from sql_planner import (
    DropTable as PlanDropTable,
)
from sql_planner import (
    DropTrigger as PlanDropTrigger,
)
from sql_planner import (
    IsNotNull as AstIsNotNull,
)
from sql_planner import (
    IsNull as AstIsNull,
)
from sql_planner import (
    Like as AstLike,
)
from sql_planner import (
    NotLike as AstNotLike,
)
from sql_planner import (
    RecursiveCTE as PlanRecursiveCTE,
)
from sql_planner import (
    UnaryOp as AstUnaryOp,
)
from sql_planner import (
    WindowAgg as PlanWindowAgg,
)
from sql_planner.ast import ColumnDef as AstColumnDef
from sql_planner.expr import AggregateExpr
from sql_planner.plan import AggFunc as PlanAggFunc
from sql_planner.plan import Limit as PlanLimit
from sql_planner.plan import WindowFuncSpec as PlanWindowFuncSpec

from .errors import UnsupportedNode
from .ir import (
    AdvanceCursor,
    AdvanceGroupKey,
    AlterTable,
    BeginRow,
    BeginTransaction,
    Between,
    BinaryOp,
    BinaryOpCode,
    CallScalar,
    CaptureLeftResult,
    CloseScan,
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
    SortKey,
    SortResult,
    UnaryOp,
    UnaryOpCode,
    UpdateAgg,
    UpdateRows,
    WinFunc,
    WinFuncSpec,
)
from .ir import (
    AggFunc as IrAggFunc,
)
from .ir import (
    ColumnDef as IrColumnDef,
)

# --------------------------------------------------------------------------
# Compilation context — threaded through every recursive call.
# --------------------------------------------------------------------------


@dataclass
class _Ctx:
    """Monotonic ID sources + the scan-alias → cursor_id map.

    The alias map is how later ``LoadColumn`` calls know which cursor to
    read from. A Scan populates the map when it opens; a Column reference
    uses it to find the scan's cursor.

    ``outer_alias_to_cursor`` is set only when compiling an inner
    sub-program that may contain :class:`~sql_planner.expr.CorrelatedRef`
    nodes.  It is a snapshot of the *outer* context's ``alias_to_cursor``
    at the time the subquery expression was compiled.  When
    :func:`_compile_expr` encounters a ``CorrelatedRef``, it looks up the
    outer cursor ID here and emits :class:`~sql_codegen.ir.LoadOuterColumn`.
    """

    cursor_counter: int = 0
    label_counter: int = 0
    agg_counter: int = 0
    alias_to_cursor: dict[str, int] = field(default_factory=dict)
    working_set_cursor_id: int | None = None
    # Outer cursor map — non-None only inside correlated sub-program compilation.
    outer_alias_to_cursor: dict[str, int] | None = None

    def new_cursor(self, alias: str) -> int:
        cid = self.cursor_counter
        self.cursor_counter += 1
        self.alias_to_cursor[alias] = cid
        return cid

    def new_label(self, base: str) -> str:
        n = self.label_counter
        self.label_counter += 1
        return f"{base}_{n}"

    def new_agg_slot(self) -> int:
        slot = self.agg_counter
        self.agg_counter += 1
        return slot


# A BodyFn takes the context and returns the per-row body instructions.
# Scan / Join will call it inside their loop.
BodyFn = Callable[[_Ctx], list[Instruction]]


# --------------------------------------------------------------------------
# Public entry points.
# --------------------------------------------------------------------------


def compile(plan: LogicalPlan) -> Program:  # noqa: A001 — name matches spec
    """Compile ``plan`` to a ``Program`` ready for the VM."""
    ctx = _Ctx()
    instrs, schema = _compile_plan(plan, ctx)
    instrs.append(Halt())
    resolved = _resolve_labels(instrs)
    return Program(
        instructions=tuple(instrs),
        labels=resolved,
        result_schema=schema,
    )


def compile_expr(expr: Expr, ctx: _Ctx | None = None) -> list[Instruction]:
    """Compile a single expression in isolation — exposed for tests."""
    return _compile_expr(expr, ctx or _Ctx())


# --------------------------------------------------------------------------
# Label resolution — single pass over the emitted instructions. Returns a
# ``{name: index}`` dict and leaves the list otherwise untouched.
# --------------------------------------------------------------------------


def _resolve_labels(instrs: list[Instruction]) -> dict[str, int]:
    labels: dict[str, int] = {}
    for i, ins in enumerate(instrs):
        if isinstance(ins, Label):
            labels[ins.name] = i
    return labels


# --------------------------------------------------------------------------
# Top-level plan dispatch. Returns (instructions, schema) where ``schema``
# is the output column list — set only by the outermost Project (or
# EmptyResult). Non-SELECT statements return an empty schema.
# --------------------------------------------------------------------------


def _compile_plan(p: LogicalPlan, ctx: _Ctx) -> tuple[list[Instruction], tuple[str, ...]]:
    match p:
        case EmptyResult(columns=cols):
            return [SetResultSchema(columns=cols)], cols

        case PlanCreateTable(table=t, columns=cols, if_not_exists=ine):
            ir_cols = tuple(_to_ir_col(c) for c in cols)
            return [CreateTable(table=t, columns=ir_cols, if_not_exists=ine)], ()

        case PlanDropTable(table=t, if_exists=ie):
            return [DropTable(table=t, if_exists=ie)], ()

        case PlanAlterTable(table=t, column=col):
            return [AlterTable(table=t, column=_to_ir_col(col))], ()

        case PlanCreateIndex(name=name, table=table, columns=cols, unique=uniq, if_not_exists=ine):
            return [
                CreateIndex(name=name, table=table, columns=cols, unique=uniq, if_not_exists=ine),
            ], ()

        case PlanDropIndex(name=name, if_exists=ie):
            return [DropIndex(name=name, if_exists=ie)], ()

        case PlanCreateTrigger(
            name=name, timing=timing, event=event, table=table, body_sql=body
        ):
            return [
                CreateTriggerDef(
                    name=name, timing=timing, event=event, table=table, body_sql=body
                ),
            ], ()

        case PlanDropTrigger(name=name, if_exists=ie):
            return [DropTriggerDef(name=name, if_exists=ie)], ()

        case Insert():
            return _compile_insert(p, ctx), ()

        case Update():
            return _compile_update(p, ctx), ()

        case Delete():
            return _compile_delete(p, ctx), ()

        # Transaction-control statements — leaf nodes, no result schema.
        case Begin():
            return [BeginTransaction()], ()

        case Commit():
            return [CommitTransaction()], ()

        case Rollback():
            return [RollbackTransaction()], ()

        case PlanWindowAgg(input=inner, specs=specs, output_cols=output_cols):
            # Window aggregation: compile the inner plan (which emits the
            # scan loop), then append ComputeWindowFunctions.  We prepend
            # SetResultSchema(inner_schema) so result.columns correctly
            # reflects the INNER column layout when ComputeWindowFunctions
            # looks up arg/partition/order column names.  ComputeWindowFunctions
            # itself sets result.columns = output_cols at the end.
            inner_instrs, inner_schema = _compile_plan(inner, ctx)
            ir_specs = tuple(_to_ir_win_spec(s) for s in specs)
            win_instr = ComputeWindowFunctions(
                specs=ir_specs,
                output_cols=output_cols,
            )
            return (
                [SetResultSchema(columns=inner_schema)]
                + inner_instrs
                + [win_instr]
            ), output_cols

        case _:
            # Read-only query path: emit SetResultSchema, then build the
            # scan/join/filter/aggregate/project stack. The outermost
            # Project, Distinct, Sort, or Limit owns the result schema.
            schema = _schema_of(p)
            prelude: list[Instruction] = []
            if schema:
                prelude.append(SetResultSchema(columns=schema))
            body = _compile_read(p, ctx)
            return prelude + body, schema


# --------------------------------------------------------------------------
# Schema extraction — follows the top of the tree down until it finds a
# Project (or equivalent) that tells us the output column names.
# --------------------------------------------------------------------------


def _schema_of(p: LogicalPlan) -> tuple[str, ...]:
    match p:
        case Project(items=items):
            # Wildcard → schema is resolved at runtime; return empty tuple
            # (the ScanAllColumns pseudo-instruction fills it in).
            if any(isinstance(i.expr, Wildcard) for i in items):
                return ()
            return tuple(_projection_name(i) for i in items)
        case Sort(input=inner) | Distinct(input=inner) | PlanLimit(input=inner):
            return _schema_of(inner)
        case Aggregate(group_by=gb, aggregates=aggs):
            names: list[str] = []
            for i, e in enumerate(gb):
                names.append(_column_display_name(e) or f"group_{i}")
            names.extend(a.alias for a in aggs)
            return tuple(names)
        case Having(input=inner):
            return _schema_of(inner)
        # Set operations: output schema follows the left side's columns.
        case Union(left=left_plan) | Intersect(left=left_plan) | Except(left=left_plan):
            return _schema_of(left_plan)
        case PlanWindowAgg(output_cols=cols):
            return cols
        case _:
            return ()


def _projection_name(item: object) -> str:
    from sql_planner import ProjectionItem

    if isinstance(item, ProjectionItem):
        if item.alias is not None:
            return item.alias
        name = _column_display_name(item.expr)
        if name is not None:
            return name
    return "?"


def _column_display_name(expr: Expr) -> str | None:
    if isinstance(expr, Column):
        return expr.col
    return None


# --------------------------------------------------------------------------
# Read-path compilation. Decomposes the plan into:
#   1. optional post-processing operators (Distinct / Sort / Limit) that
#      apply to the assembled result buffer;
#   2. an inner read core (Aggregate / Project over a Scan / Join / Filter).
# --------------------------------------------------------------------------


def _compile_read(p: LogicalPlan, ctx: _Ctx) -> list[Instruction]:
    # Peel outer post-processing operators off the top. Order per spec:
    # Project → Distinct → Sort → Limit — so Limit is the outermost.
    post: list[Instruction] = []
    cur = p
    while True:
        match cur:
            case PlanLimit(input=inner, count=c, offset=o):
                post.append(LimitResult(count=c, offset=o))
                cur = inner
            case Sort(input=inner, keys=keys):
                post.append(SortResult(keys=tuple(_to_sort_key(k) for k in keys)))
                cur = inner
            case Distinct(input=inner):
                post.append(DistinctResult())
                cur = inner
            case _:
                break
    # ``post`` is in outer-to-inner order: Limit was outermost → appended
    # first. Execution must run Sort/Distinct/Limit in the spec order
    # (Sort, then Limit). We reverse so Sort runs before Limit.
    post.reverse()

    core = _compile_core(cur, ctx)
    return core + post


# --------------------------------------------------------------------------
# Core compilation — Aggregate or "scan + filter + project".
# --------------------------------------------------------------------------


def _compile_core(p: LogicalPlan, ctx: _Ctx) -> list[Instruction]:
    match p:
        case Aggregate():
            return _compile_aggregate(p, ctx)
        case Having(input=Aggregate() as agg, predicate=pred):
            return _compile_aggregate(agg, ctx, having=pred)
        # Project(Aggregate) — occurs in scalar subquery inner plans that haven't
        # had _flatten_project_over_aggregate applied. Skip the projection layer;
        # column names don't matter for sub-program result rows.
        case Project(input=Aggregate() as agg):
            return _compile_aggregate(agg, ctx)
        case Project(input=Having(input=Aggregate() as agg, predicate=pred)):
            return _compile_aggregate(agg, ctx, having=pred)

        # Set operations — compile left side, then right side, then post-process.
        #
        # UNION [ALL]:  left rows + right rows → optional DistinctResult
        # INTERSECT:    left rows → CaptureLeftResult → right rows → IntersectResult
        # EXCEPT:       left rows → CaptureLeftResult → right rows → ExceptResult
        #
        # Both sides are compiled via _compile_read so that each side's own
        # Distinct/Sort/Limit wrappers are handled correctly before the set
        # operation takes place. The ctx (cursor + label counters) is shared
        # so IDs stay globally unique across both sides.

        case Union(left=left_plan, right=right_plan, all=all_flag):
            out = _compile_read(left_plan, ctx)
            out.extend(_compile_read(right_plan, ctx))
            if not all_flag:
                out.append(DistinctResult())
            return out

        case Intersect(left=left_plan, right=right_plan, all=all_flag):
            out = _compile_read(left_plan, ctx)
            out.append(CaptureLeftResult())
            out.extend(_compile_read(right_plan, ctx))
            out.append(IntersectResult(all=all_flag))
            return out

        case Except(left=left_plan, right=right_plan, all=all_flag):
            out = _compile_read(left_plan, ctx)
            out.append(CaptureLeftResult())
            out.extend(_compile_read(right_plan, ctx))
            out.append(ExceptResult(all=all_flag))
            return out

        case PlanWindowAgg(input=inner, specs=specs, output_cols=output_cols):
            # Compile the inner plan, then append ComputeWindowFunctions.
            #
            # Critical invariant: when ComputeWindowFunctions executes,
            # result.columns must reflect the INNER schema (the columns
            # emitted by the scan loop) so it can look up arg/partition/order
            # columns by name.  The outer _compile_plan catch-all has already
            # emitted SetResultSchema(output_cols), so we prepend an explicit
            # SetResultSchema(inner_schema) to override it.  This override is
            # always correct:
            #   - non-empty inner_schema: inner_instrs also starts with
            #     SetResultSchema(inner_schema), so we get a harmless duplicate.
            #   - empty inner_schema (window-only SELECT with no non-window
            #     items): inner_instrs emits no SetResultSchema (because the
            #     catch-all skips it for empty schemas), so the prepend is the
            #     only one and is required.
            inner_instrs, inner_schema = _compile_plan(inner, ctx)
            ir_specs = tuple(_to_ir_win_spec(s) for s in specs)
            return (
                [SetResultSchema(columns=inner_schema)]
                + inner_instrs
                + [ComputeWindowFunctions(specs=ir_specs, output_cols=output_cols)]
            )

        case _:
            # Project(Filter(Join/Scan)) — the ordinary SELECT shape.
            return _compile_select(p, ctx)


# --------------------------------------------------------------------------
# SELECT compilation — build a body function then hand it to the Scan/Join.
#
# The shape of a read plan after the optimizer is::
#
#     Project
#       Filter
#         Join / Scan
#
# ``_peel_projection`` pulls the Project off the top if present; the body
# function then assembles each row using the projection items (or
# ScanAllColumns for SELECT *). ``_peel_filter`` wraps that body with a
# JumpIfFalse predicate check.
# --------------------------------------------------------------------------


def _compile_select(p: LogicalPlan, ctx: _Ctx) -> list[Instruction]:
    project_items, inner = _peel_projection(p)
    predicate, inner = _peel_filter(inner)

    def body(c: _Ctx) -> list[Instruction]:
        # Generate a fresh skip label on each invocation so that calling
        # body twice (e.g. matched path + null-padded path in a LEFT JOIN)
        # does not produce duplicate Label names in the instruction stream.
        skip = c.new_label("filter_skip") if predicate is not None else ""
        out: list[Instruction] = []
        if predicate is not None:
            out.extend(_compile_expr(predicate, c))
            out.append(JumpIfFalse(label=skip))
        # Build the row.
        out.append(BeginRow())
        if project_items is None:
            # Happens for bare Scan (no explicit Project) — shouldn't be a
            # SELECT shape, but handle defensively with wildcard emit.
            primary = _primary_cursor(c)
            if primary is not None:
                out.append(ScanAllColumns(cursor_id=primary))
        else:
            for it in project_items:
                if isinstance(it.expr, Wildcard):
                    primary = _primary_cursor(c)
                    if primary is not None:
                        out.append(ScanAllColumns(cursor_id=primary))
                else:
                    out.extend(_compile_expr(it.expr, c))
                    out.append(EmitColumn(name=_projection_name(it)))
        out.append(EmitRow())
        if predicate is not None:
            out.append(Label(name=skip))
        return out

    return _compile_source(inner, body, ctx)


def _peel_projection(p: LogicalPlan) -> tuple[object, LogicalPlan]:
    if isinstance(p, Project):
        return p.items, p.input
    return None, p


def _peel_filter(p: LogicalPlan) -> tuple[Expr | None, LogicalPlan]:
    if isinstance(p, Filter):
        return p.predicate, p.input
    return None, p


def _primary_cursor(ctx: _Ctx) -> int | None:
    """The cursor id of the first scan opened — used by SELECT *."""
    if not ctx.alias_to_cursor:
        return None
    return next(iter(ctx.alias_to_cursor.values()))


# --------------------------------------------------------------------------
# Data-source compilation — Scan / Join. These emit the loop scaffolding
# and splice ``body(ctx)`` inside.
# --------------------------------------------------------------------------


def _compile_source(
    p: LogicalPlan, body: BodyFn, ctx: _Ctx
) -> list[Instruction]:
    match p:
        case Scan(table=t, alias=a):
            alias = a or t
            cid = ctx.new_cursor(alias)
            loop = ctx.new_label("scan_loop")
            end = ctx.new_label("scan_end")
            out: list[Instruction] = [
                OpenScan(cursor_id=cid, table=t),
                Label(name=loop),
                AdvanceCursor(cursor_id=cid, on_exhausted=end),
            ]
            out.extend(body(ctx))
            out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
            return out

        case IndexScan(
            table=t,
            alias=a,
            index_name=index_name,
            columns=_,
            lo=lo,
            hi=hi,
            lo_inclusive=lo_inc,
            hi_inclusive=hi_inc,
            residual=residual,
        ):
            # An index scan replaces OpenScan with OpenIndexScan, which does
            # the index lookup and materialises matching rows into the cursor.
            # The AdvanceCursor / LoadColumn / CloseScan loop then works
            # exactly as for a full table scan — the caller sees no difference.
            alias = a or t
            cid = ctx.new_cursor(alias)
            loop = ctx.new_label("idx_loop")
            end = ctx.new_label("idx_end")
            out: list[Instruction] = [
                OpenIndexScan(
                    cursor_id=cid,
                    table=t,
                    index_name=index_name,
                    lo=lo,
                    hi=hi,
                    lo_inclusive=lo_inc,
                    hi_inclusive=hi_inc,
                ),
                Label(name=loop),
                AdvanceCursor(cursor_id=cid, on_exhausted=end),
            ]
            # Residual predicate: a condition not fully covered by the index
            # (e.g. the second column of a compound AND).  Compile it and skip
            # the row-building body if false, mirroring the Filter-under-Scan
            # pattern used elsewhere.
            if residual is not None:
                skip = ctx.new_label("idx_skip")
                out.extend(_compile_expr(residual, ctx))
                out.append(JumpIfFalse(label=skip))
                out.extend(body(ctx))
                out.append(Label(name=skip))
            else:
                out.extend(body(ctx))
            out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
            return out

        case DerivedTable(query=inner_query, alias=alias, columns=_):
            # Compile the inner query independently with its own cursor/label
            # namespace so IDs don't collide with the outer program.
            inner_ctx = _Ctx()
            inner_instrs, inner_schema = _compile_plan(inner_query, inner_ctx)
            inner_instrs.append(Halt())
            inner_resolved = _resolve_labels(inner_instrs)
            sub_program = Program(
                instructions=tuple(inner_instrs),
                labels=inner_resolved,
                result_schema=inner_schema,
            )
            # In the outer program: emit RunSubquery to materialise rows, then
            # loop over them with the normal AdvanceCursor / CloseScan pattern.
            cid = ctx.new_cursor(alias)
            loop = ctx.new_label("subq_loop")
            end = ctx.new_label("subq_end")
            out = [
                RunSubquery(cursor_id=cid, sub_program=sub_program),
                Label(name=loop),
                AdvanceCursor(cursor_id=cid, on_exhausted=end),
            ]
            out.extend(body(ctx))
            out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
            return out

        case Join(left=lft, right=rgt, kind=kind, condition=cond):
            return _compile_join(lft, rgt, kind, cond, body, ctx)

        case Filter(input=inner, predicate=pred):
            # A filter below a filter (rare after the optimizer, but valid).
            skip = ctx.new_label("filter_skip")

            def wrapped(c: _Ctx) -> list[Instruction]:
                out_: list[Instruction] = _compile_expr(pred, c)
                out_.append(JumpIfFalse(label=skip))
                out_.extend(body(c))
                out_.append(Label(name=skip))
                return out_

            return _compile_source(inner, wrapped, ctx)

        case WorkingSetScan(alias=alias, columns=_):
            # The VM stores the working set rows in VmState.working_set_data.
            # OpenWorkingSetScan creates a fresh _SubqueryCursor from those
            # rows each time it executes — critical for correctness when the
            # CTE self-reference appears inside a JOIN (the outer loop would
            # otherwise exhaust the cursor on the first row and leave nothing
            # for subsequent rows).
            cid = ctx.working_set_cursor_id if ctx.working_set_cursor_id is not None else 0
            ctx.alias_to_cursor[alias] = cid
            loop = ctx.new_label("wss_loop")
            end = ctx.new_label("wss_end")
            out = [
                OpenWorkingSetScan(cursor_id=cid),
                Label(name=loop),
                AdvanceCursor(cursor_id=cid, on_exhausted=end),
            ]
            out.extend(body(ctx))
            out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
            return out

        case PlanRecursiveCTE(
            anchor=anchor_plan,
            recursive=recursive_plan,
            alias=alias,
            columns=_,
            union_all=union_all,
        ):
            # Compile anchor as an independent sub-program.
            anchor_ctx = _Ctx()
            anchor_instrs, anchor_schema = _compile_plan(anchor_plan, anchor_ctx)
            anchor_instrs.append(Halt())
            anchor_prog = Program(
                instructions=tuple(anchor_instrs),
                labels=_resolve_labels(anchor_instrs),
                result_schema=anchor_schema,
            )
            # Compile recursive step: cursor 0 is pre-reserved for the working
            # set cursor (VM pre-populates it before each iteration), so we
            # start assigning other cursors from 1.
            recursive_ctx = _Ctx(cursor_counter=1, working_set_cursor_id=0)
            recursive_instrs, recursive_schema = _compile_plan(recursive_plan, recursive_ctx)
            recursive_instrs.append(Halt())
            recursive_prog = Program(
                instructions=tuple(recursive_instrs),
                labels=_resolve_labels(recursive_instrs),
                result_schema=recursive_schema,
            )
            cid = ctx.new_cursor(alias)
            loop = ctx.new_label("rcte_loop")
            end = ctx.new_label("rcte_end")
            out = [
                RunRecursiveCTE(
                    cursor_id=cid,
                    anchor_program=anchor_prog,
                    recursive_program=recursive_prog,
                    working_cursor_id=0,
                    union_all=union_all,
                ),
                Label(name=loop),
                AdvanceCursor(cursor_id=cid, on_exhausted=end),
            ]
            out.extend(body(ctx))
            out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
            return out

        case _:
            raise UnsupportedNode(type(p).__name__)


def _compile_join(
    lft: LogicalPlan,
    rgt: LogicalPlan,
    kind: JoinKind,
    cond: Expr | None,
    body: BodyFn,
    ctx: _Ctx,
) -> list[Instruction]:
    if kind in (JoinKind.INNER, JoinKind.CROSS):
        def inner_body(c: _Ctx) -> list[Instruction]:
            out: list[Instruction] = []
            skip = c.new_label("join_skip")
            if cond is not None and kind == JoinKind.INNER:
                out.extend(_compile_expr(cond, c))
                out.append(JumpIfFalse(label=skip))
            out.extend(body(c))
            if cond is not None and kind == JoinKind.INNER:
                out.append(Label(name=skip))
            return out

        def outer_body(c: _Ctx) -> list[Instruction]:
            return _compile_source(rgt, inner_body, c)

        return _compile_source(lft, outer_body, ctx)

    if kind == JoinKind.LEFT:
        # Nested-loop LEFT OUTER JOIN.
        #
        # For each left row we track whether any right row satisfied the ON
        # condition (join_match_stack in the VM).  After the right scan
        # exhausts, if no match was found we emit ``body`` once more; at that
        # point the right cursor has no current row so every LoadColumn for a
        # right-side column returns NULL — exactly the null-padding SQL
        # requires.
        #
        # ``body`` is called at most twice per left row:
        #   1. Once per matching (left, right) pair inside the inner loop.
        #   2. At most once on the null-padded path if zero right rows matched.
        # Each call to ``body`` generates a fresh filter-skip label (the body
        # closure uses c.new_label, not a closed-over static string), so
        # duplicate label names cannot appear in the instruction stream.
        matched_label = ctx.new_label("loj_matched")

        def loj_inner_body(c: _Ctx) -> list[Instruction]:
            out: list[Instruction] = []
            skip = c.new_label("loj_cond_skip")
            if cond is not None:
                out.extend(_compile_expr(cond, c))
                out.append(JumpIfFalse(label=skip))
            # ON condition passed: mark this left row as having a match,
            # then emit the regular (non-null-padded) output row.
            out.append(JoinSetMatched())
            out.extend(body(c))
            out.append(Label(name=skip))
            return out

        def loj_outer_body(c: _Ctx) -> list[Instruction]:
            out: list[Instruction] = []
            # Begin a new match-tracking epoch for this left row.
            out.append(JoinBeginRow())
            out.extend(_compile_source(rgt, loj_inner_body, c))
            # After the right scan: if at least one right row matched ON,
            # jump past the null-padded emission.
            out.append(JoinIfMatched(label=matched_label))
            # No match found: emit body with the right cursor closed.
            # LoadColumn for right-side columns returns NULL automatically
            # because the cursor has no current row.
            out.extend(body(c))
            out.append(Label(name=matched_label))
            return out

        return _compile_source(lft, loj_outer_body, ctx)

    if kind == JoinKind.RIGHT:
        # RIGHT OUTER JOIN = LEFT OUTER JOIN with the two sides swapped in the
        # execution loop.  The ON condition and body both reference columns by
        # table alias (via alias_to_cursor), not by physical scan position, so
        # reversing which side is the outer loop is sufficient: the original
        # right table becomes the outer "left" (preserved for every row) and
        # the original left table becomes the inner "right" (null-padded when
        # no ON match is found).  Output column order is controlled by the
        # Project node above the join and is not affected by the swap.
        return _compile_join(rgt, lft, JoinKind.LEFT, cond, body, ctx)

    if kind == JoinKind.FULL:
        # FULL OUTER JOIN — two-pass strategy:
        #
        # Pass 1: LEFT JOIN(lft, rgt)
        #   Emits every lft row.  If a rgt row matches, the body runs with
        #   real values on both sides.  If no rgt row matches, the body runs
        #   with the right cursor closed (right cols = NULL).
        #
        # Pass 2: right-anti-join — scan rgt as outer, lft as inner.
        #   For each rgt row, scan lft and check the ON condition.  If ANY
        #   lft row matched, the rgt row was already emitted by Pass 1 and
        #   must be skipped here.  If NO lft row matched, emit the rgt row
        #   with the left cursor closed (left cols = NULL).
        #
        # Cursor IDs: Pass 1 allocates cursors 0=lft, 1=rgt.  Pass 2 calls
        # _compile_source again, so ctx.new_cursor reassigns the aliases to
        # fresh IDs (2=rgt, 3=lft for pass 2).  When body(c) executes in
        # pass 2's null-padded path, alias_to_cursor maps lft_alias→3
        # (closed, returns NULL) and rgt_alias→2 (open, real values). ✓

        # ------------------------------------------------------------------
        # Pass 1: standard LEFT JOIN
        # ------------------------------------------------------------------
        pass1_instrs = _compile_join(lft, rgt, JoinKind.LEFT, cond, body, ctx)

        # ------------------------------------------------------------------
        # Pass 2: right-anti-join (rgt rows not matched by any lft row)
        # ------------------------------------------------------------------
        anti_matched_label = ctx.new_label("foj_anti_matched")

        def foj_anti_inner(c: _Ctx) -> list[Instruction]:
            # Inner body: only check ON condition and mark matched.
            # Do NOT call body(c) here — we only want to detect a match.
            out: list[Instruction] = []
            skip = c.new_label("foj_anti_cond_skip")
            if cond is not None:
                out.extend(_compile_expr(cond, c))
                out.append(JumpIfFalse(label=skip))
            out.append(JoinSetMatched())
            out.append(Label(name=skip))
            return out

        def foj_anti_outer(c: _Ctx) -> list[Instruction]:
            # Outer body: per-rgt-row match tracking.
            out: list[Instruction] = []
            out.append(JoinBeginRow())
            out.extend(_compile_source(lft, foj_anti_inner, c))
            # If any lft row matched the ON condition, skip emission —
            # this rgt row was already in Pass 1's output.
            out.append(JoinIfMatched(label=anti_matched_label))
            # No lft row matched: emit body with lft cursor closed.
            # LoadColumn for lft-side columns returns NULL because the
            # inner cursor has no current row.
            out.extend(body(c))
            out.append(Label(name=anti_matched_label))
            return out

        pass2_instrs = _compile_source(rgt, foj_anti_outer, ctx)

        return pass1_instrs + pass2_instrs

    raise UnsupportedNode(f"Join({kind})")


# --------------------------------------------------------------------------
# Aggregate compilation.
#
# Two-phase: the scan loop body maintains group state; after the loop we
# emit one row per group. The VM owns the actual hash-map keyed on the
# saved group key — the compiler just emits instructions that address
# slots by index.
# --------------------------------------------------------------------------


def _compile_aggregate(
    agg: Aggregate, ctx: _Ctx, having: Expr | None = None
) -> list[Instruction]:
    group_by = agg.group_by
    aggregates = agg.aggregates

    # Allocate slots for each aggregate.
    slots = [ctx.new_agg_slot() for _ in aggregates]

    def body(c: _Ctx) -> list[Instruction]:
        out: list[Instruction] = []
        # Build the group key.
        for e in group_by:
            out.extend(_compile_expr(e, c))
        out.append(SaveGroupKey(n=len(group_by)))
        # Initialize slots on first encounter (VM handles idempotence).
        for s, a in zip(slots, aggregates, strict=True):
            out.append(InitAgg(slot=s, func=_plan_agg_to_ir(a.func)))
        # Update each aggregate with the row's value.
        for s, a in zip(slots, aggregates, strict=True):
            if a.arg.star:
                # COUNT(*) always increments — push a sentinel 1; the VM's
                # COUNT_STAR semantics ignore the value and just increment.
                out.append(LoadConst(value=1))
            else:
                out.extend(_compile_expr(a.arg.value, c))  # type: ignore[arg-type]
            out.append(UpdateAgg(slot=s))
        return out

    core = _compile_source(agg.input, body, ctx)

    # Emit a row per group after the scan loop.
    emit_start = ctx.new_label("group_emit_start")
    emit_next = ctx.new_label("group_emit_next")
    emit_end = ctx.new_label("group_emit_end")

    # The VM expands this pseudo-instruction into the per-group iteration.
    # We express it here as a label-bounded block containing one set of
    # per-group instructions; the VM runs the block once per group.
    # The emit loop: advance to the next group at the top; if exhausted jump
    # past the block. This is the same header-then-body pattern as Scan.
    post: list[Instruction] = [
        Label(name=emit_start),
        AdvanceGroupKey(on_exhausted=emit_end),
    ]

    if having is not None:
        # HAVING is evaluated per group — Finalize each aggregate referenced
        # by the predicate on demand. In this v1 we take a simpler approach
        # and ask the VM to run the whole post block per group, discarding
        # rows for which the predicate is false.
        post.extend(_compile_having(having, group_by, aggregates, slots, ctx))
        post.append(JumpIfFalse(label=emit_next))

    # Build the output row: group keys first, then finalized aggregates.
    post.append(BeginRow())
    for i, e in enumerate(group_by):
        post.append(LoadGroupKey(i=i))
        name = _column_display_name(e) or f"group_{i}"
        post.append(EmitColumn(name=name))
    for s, a in zip(slots, aggregates, strict=True):
        post.append(FinalizeAgg(slot=s))
        post.append(EmitColumn(name=a.alias))
    post.append(EmitRow())
    post.append(Label(name=emit_next))
    # A Jump to emit_start here would tell the VM to advance the group
    # iterator; the VM walks its hash map and reruns the block until done.
    post.append(Jump(label=emit_start))
    post.append(Label(name=emit_end))
    return core + post


def _compile_having(
    having: Expr,
    group_by: tuple[Expr, ...],
    aggregates: tuple[object, ...],
    slots: list[int],
    ctx: _Ctx,
) -> list[Instruction]:
    """Compile a HAVING predicate — aggregates reference slots; plain
    columns reference the saved group key by position.

    AggregateExpr and Column references are handled via the slot/group-key
    maps. All other expressions (including EXISTS subqueries, unary ops,
    and binary comparisons) are delegated to _compile_expr so the full
    expression language is available in HAVING predicates.
    """
    # Build a lookup: aggregate expression → slot, and column → group-key index.
    group_lookup = {_column_display_name(e): i for i, e in enumerate(group_by)}

    def walk(e: Expr) -> list[Instruction]:
        match e:
            case AggregateExpr(func=f, arg=arg):
                # Find the slot whose (func, arg) matches.
                from sql_planner import AggregateItem  # local to avoid cycle

                for s, a in zip(slots, aggregates, strict=True):
                    assert isinstance(a, AggregateItem)
                    # arg=None is the legacy direct-construction form for COUNT(*);
                    # planner always produces FuncArg(star=True). Accept both.
                    matched = (
                        (arg is None and a.arg.star)
                        or (arg is not None and a.arg == arg)
                    )
                    if a.func is f and matched:
                        return [FinalizeAgg(slot=s)]
                raise UnsupportedNode("AggregateExpr not present in Aggregate.aggregates")
            case Column(col=c):
                if c in group_lookup:
                    return [LoadGroupKey(i=group_lookup[c])]
                raise UnsupportedNode(f"HAVING column '{c}' not in GROUP BY")
            case Literal(value=v):
                return [LoadConst(value=v)]
            case BinaryExpr(op=op, left=l_, right=r_):
                return walk(l_) + walk(r_) + [BinaryOp(op=_binop_to_ir(op))]
            case _:
                # Delegate to general expression compiler for EXISTS, UnaryExpr,
                # and any other expression that doesn't reference aggregate slots.
                return _compile_expr(e, ctx)

    return walk(having)


# --------------------------------------------------------------------------
# Expression compilation — stack-machine emission.
# --------------------------------------------------------------------------


def _compile_expr(e: Expr, ctx: _Ctx) -> list[Instruction]:
    match e:
        case Literal(value=v):
            return [LoadConst(value=v)]
        case Column(table=t, col=c):
            cid = ctx.alias_to_cursor.get(t or "", 0)
            return [LoadColumn(cursor_id=cid, column=c)]
        case CorrelatedRef(outer_alias=alias, col=c):
            # A correlated reference resolves against the *outer* query's
            # cursor map.  The outer map was captured when the inner sub-program
            # compilation started (see the ExistsSubquery / ScalarSubquery /
            # InSubquery / NotInSubquery cases below).
            #
            # At runtime the VM passes the outer state's ``current_row``
            # snapshot to the inner execution; ``LoadOuterColumn`` reads from
            # that snapshot rather than the inner program's own cursor table.
            if ctx.outer_alias_to_cursor is None:
                raise UnsupportedNode(
                    f"CorrelatedRef({alias!r}.{c!r}) found but no outer cursor map in context"
                )
            cid = ctx.outer_alias_to_cursor.get(alias, 0)
            return [LoadOuterColumn(cursor_id=cid, col=c)]
        case BinaryExpr(op=op, left=l_, right=r_):
            return _compile_expr(l_, ctx) + _compile_expr(r_, ctx) + [BinaryOp(op=_binop_to_ir(op))]
        case UnaryExpr(op=op, operand=o):
            return _compile_expr(o, ctx) + [UnaryOp(op=_unop_to_ir(op))]
        case AstIsNull(operand=o):
            return _compile_expr(o, ctx) + [IsNull()]
        case AstIsNotNull(operand=o):
            return _compile_expr(o, ctx) + [IsNotNull()]
        case AstBetween(operand=op_, low=lo, high=hi):
            return (
                _compile_expr(op_, ctx)
                + _compile_expr(lo, ctx)
                + _compile_expr(hi, ctx)
                + [Between()]
            )
        case In(operand=op_, values=vs):
            out: list[Instruction] = _compile_expr(op_, ctx)
            for v in vs:
                out.extend(_compile_expr(v, ctx))
            out.append(InList(n=len(vs)))
            return out
        case NotIn(operand=op_, values=vs):
            out = _compile_expr(op_, ctx)
            for v in vs:
                out.extend(_compile_expr(v, ctx))
            out.extend([InList(n=len(vs)), UnaryOp(op=UnaryOpCode.NOT)])
            return out
        case AstLike(operand=op_, pattern=pat):
            return _compile_expr(op_, ctx) + [LoadConst(value=pat), Like(negated=False)]
        case AstNotLike(operand=op_, pattern=pat):
            return _compile_expr(op_, ctx) + [LoadConst(value=pat), Like(negated=True)]
        case FunctionCall(name=name, args=args):
            # Compile all positional arguments onto the stack left-to-right,
            # then emit CallScalar(func, n_args). The VM's built-in function
            # registry does the rest. Unknown names raise UnsupportedFunction
            # at runtime rather than compile time so user-defined functions
            # (a future feature) can be registered without recompiling.
            out = []
            n = 0
            for a in args:
                if a.value is not None:
                    out.extend(_compile_expr(a.value, ctx))
                    n += 1
            out.append(CallScalar(func=name.lower(), n_args=n))
            return out
        case CaseExpr(whens=whens, else_=else_):
            # Compile CASE to a conditional-jump chain using JumpIfFalse /
            # Jump. After the END label exactly one value sits on the stack.
            #
            # Pattern for each WHEN branch:
            #   compile(condition)
            #   JumpIfFalse(next_lbl)
            #   compile(result)
            #   Jump(end_lbl)
            #   Label(next_lbl)
            # After all WHENs: compile(else) or LoadConst(None)
            #   Label(end_lbl)
            end_lbl = ctx.new_label("case_end")
            out: list[Instruction] = []
            for cond, result in whens:
                next_lbl = ctx.new_label("case_next")
                out.extend(_compile_expr(cond, ctx))
                out.append(JumpIfFalse(label=next_lbl))
                out.extend(_compile_expr(result, ctx))
                out.append(Jump(label=end_lbl))
                out.append(Label(name=next_lbl))
            # ELSE branch (or NULL if absent)
            if else_ is not None:
                out.extend(_compile_expr(else_, ctx))
            else:
                out.append(LoadConst(value=None))
            out.append(Label(name=end_lbl))
            return out
        case ExistsSubquery(query=inner_plan):
            # Compile the inner LogicalPlan to a standalone sub-program.
            # The inner program runs against the same backend but with its
            # own cursor/label namespace so there is no state leakage.
            #
            # ``outer_alias_to_cursor`` captures the enclosing scan map so
            # that any CorrelatedRef nodes inside the inner plan can resolve
            # to the correct outer cursor IDs at runtime.
            inner_ctx = _Ctx(outer_alias_to_cursor=dict(ctx.alias_to_cursor))
            inner_instrs, _ = _compile_plan(inner_plan, inner_ctx)  # type: ignore[arg-type]
            inner_instrs.append(Halt())
            inner_resolved = _resolve_labels(inner_instrs)
            sub = Program(
                instructions=tuple(inner_instrs),
                labels=inner_resolved,
                result_schema=(),
            )
            return [RunExistsSubquery(sub_program=sub)]
        case ScalarSubquery(query=inner_plan):
            # Compile the inner SELECT to a standalone sub-program.  At
            # runtime the VM executes it, takes the first column of the
            # single result row, and pushes it as the scalar value.
            inner_ctx = _Ctx(outer_alias_to_cursor=dict(ctx.alias_to_cursor))
            inner_instrs, _ = _compile_plan(inner_plan, inner_ctx)  # type: ignore[arg-type]
            inner_instrs.append(Halt())
            inner_resolved = _resolve_labels(inner_instrs)
            sub = Program(
                instructions=tuple(inner_instrs),
                labels=inner_resolved,
                result_schema=(),
            )
            return [RunScalarSubquery(sub_program=sub)]
        case InSubquery(operand=op, query=inner_plan):
            # Compile the inner plan to a standalone sub-program, then compile
            # the outer operand expression (pushes test value), then emit
            # RunInSubquery which pops the test value and pushes a bool.
            inner_ctx = _Ctx(outer_alias_to_cursor=dict(ctx.alias_to_cursor))
            inner_instrs, _ = _compile_plan(inner_plan, inner_ctx)  # type: ignore[arg-type]
            inner_instrs.append(Halt())
            sub = Program(
                instructions=tuple(inner_instrs),
                labels=_resolve_labels(inner_instrs),
                result_schema=(),
            )
            return [*_compile_expr(op, ctx), RunInSubquery(sub_program=sub, negate=False)]  # type: ignore[arg-type]
        case NotInSubquery(operand=op, query=inner_plan):
            inner_ctx = _Ctx(outer_alias_to_cursor=dict(ctx.alias_to_cursor))
            inner_instrs, _ = _compile_plan(inner_plan, inner_ctx)  # type: ignore[arg-type]
            inner_instrs.append(Halt())
            sub = Program(
                instructions=tuple(inner_instrs),
                labels=_resolve_labels(inner_instrs),
                result_schema=(),
            )
            return [*_compile_expr(op, ctx), RunInSubquery(sub_program=sub, negate=True)]  # type: ignore[arg-type]
        case AggregateExpr():
            # At this point in compilation we're inside an aggregate node's
            # HAVING or projection; direct emission isn't possible without
            # knowing the slot. The aggregate compiler handles these paths.
            raise UnsupportedNode("AggregateExpr outside aggregate context")
        case Wildcard():
            raise UnsupportedNode("Wildcard in expression position")
        case _:
            raise UnsupportedNode(type(e).__name__)


# --------------------------------------------------------------------------
# DML — Insert, Update, Delete.
# --------------------------------------------------------------------------


def _returning_col_name(expr: Expr, idx: int) -> str:
    """Derive a display name for a RETURNING column expression.

    For plain column references we use the column name (e.g. ``'id'``).
    For anything more complex we fall back to a positional name so downstream
    code (SetResultSchema, cursor.description) always has a non-empty string.
    """
    if isinstance(expr, Column):
        return expr.col
    return f"column_{idx}"


def _compile_returning_insert_expr(expr: Expr, ctx: _Ctx) -> list[Instruction]:
    """Compile a RETURNING expression in the context of an INSERT statement.

    Column references (``Column``) are emitted as ``LoadLastInsertedColumn``
    because there is no open cursor after an INSERT — the row is only
    accessible via ``_VmState.last_inserted_row``.

    All other expression types (``Literal``, ``BinaryExpr``, etc.) are handled
    by the regular ``_compile_expr`` — they do not reference cursor state.
    Note: if a non-Column sub-expression contains a nested Column node it will
    fall back to the alias-map cursor lookup, which is wrong for INSERT.  In
    practice RETURNING expressions are simple column references or literals, and
    complex arithmetic over inserted columns is uncommon.  Support for nested
    Column refs inside expressions can be added later by making this function
    fully recursive over every expression kind.
    """
    if isinstance(expr, Column):
        return [LoadLastInsertedColumn(col=expr.col)]
    # Literals, function calls, arithmetic over literals — no column reads.
    return _compile_expr(expr, ctx)


def _compile_returning_cursor_expr(
    expr: Expr, ctx: _Ctx, cursor_id: int
) -> list[Instruction]:
    """Compile a RETURNING expression in the context of UPDATE or DELETE.

    Column references are emitted as ``LoadColumn(cursor_id, col)`` where
    ``cursor_id`` is the scan cursor that holds the current row.

    For UPDATE, the cursor's ``current_row`` is already updated when RETURNING
    is emitted (``UpdateRows`` patches ``st.current_row`` before returning).
    For DELETE, RETURNING is emitted *before* ``DeleteRows`` so the cursor
    still holds the pre-deletion row.
    """
    if isinstance(expr, Column):
        return [LoadColumn(cursor_id=cursor_id, column=expr.col)]
    return _compile_expr(expr, ctx)


def _compile_insert(ins: Insert, ctx: _Ctx) -> list[Instruction]:
    src = ins.source
    cols = ins.columns or ()

    # RETURNING preamble: if this INSERT has a RETURNING clause we emit a
    # SetResultSchema once at the top so the VM knows the output column names.
    ret_schema: list[Instruction] = []
    if ins.returning:
        ret_cols = tuple(
            _returning_col_name(expr, i + 1) for i, expr in enumerate(ins.returning)
        )
        ret_schema = [SetResultSchema(columns=ret_cols)]

    if src.values is not None:
        out: list[Instruction] = list(ret_schema)
        for row in src.values:
            for v in row:
                out.extend(_compile_expr(v, ctx))
            out.append(InsertRow(table=ins.table, columns=tuple(cols)))
            # After InsertRow the VM stores the inserted row in
            # ``last_inserted_row``.  Emit RETURNING columns by reading from it.
            if ins.returning:
                out.append(BeginRow())
                for i, ret_expr in enumerate(ins.returning):
                    out.extend(_compile_returning_insert_expr(ret_expr, ctx))
                    out.append(
                        EmitColumn(name=_returning_col_name(ret_expr, i + 1))
                    )
                out.append(EmitRow())
        return out

    # INSERT … SELECT: compile the sub-SELECT into the result buffer, then
    # drain it with InsertFromResult. _compile_plan is safe to call
    # recursively here — it shares the same _Ctx (cursor/label counters stay
    # globally unique) and it does NOT emit a Halt.
    # Note: RETURNING is not supported with INSERT … SELECT in this version.
    assert src.query is not None
    select_instrs, _ = _compile_plan(src.query, ctx)
    select_instrs.append(InsertFromResult(table=ins.table, columns=tuple(cols)))
    return select_instrs


def _compile_update(upd: Update, ctx: _Ctx) -> list[Instruction]:
    cid = ctx.new_cursor(upd.table)
    loop = ctx.new_label("update_loop")
    end = ctx.new_label("update_end")
    skip = ctx.new_label("update_skip")
    out: list[Instruction] = []
    # Emit SetResultSchema once at the top when RETURNING is present so the VM
    # knows the output column names before any rows are emitted.
    if upd.returning:
        ret_cols = tuple(
            _returning_col_name(expr, i + 1) for i, expr in enumerate(upd.returning)
        )
        out.append(SetResultSchema(columns=ret_cols))
    out.extend([
        OpenScan(cursor_id=cid, table=upd.table),
        Label(name=loop),
        AdvanceCursor(cursor_id=cid, on_exhausted=end),
    ])
    if upd.predicate is not None:
        out.extend(_compile_expr(upd.predicate, ctx))
        out.append(JumpIfFalse(label=skip))
    for a in upd.assignments:
        out.extend(_compile_expr(a.value, ctx))
    out.append(
        UpdateRows(
            table=upd.table,
            assignments=tuple(a.column for a in upd.assignments),
            cursor_id=cid,
        )
    )
    # RETURNING: emit columns AFTER UpdateRows — ``st.current_row[cid]`` is
    # patched by the VM's ``_do_update`` before returning, so LoadColumn reads
    # the post-update values.
    if upd.returning:
        out.append(BeginRow())
        for i, ret_expr in enumerate(upd.returning):
            out.extend(_compile_returning_cursor_expr(ret_expr, ctx, cid))
            out.append(EmitColumn(name=_returning_col_name(ret_expr, i + 1)))
        out.append(EmitRow())
    if upd.predicate is not None:
        out.append(Label(name=skip))
    out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
    return out


def _compile_delete(dlt: Delete, ctx: _Ctx) -> list[Instruction]:
    cid = ctx.new_cursor(dlt.table)
    loop = ctx.new_label("delete_loop")
    end = ctx.new_label("delete_end")
    skip = ctx.new_label("delete_skip")
    out: list[Instruction] = []
    # Emit SetResultSchema once at the top when RETURNING is present.
    if dlt.returning:
        ret_cols = tuple(
            _returning_col_name(expr, i + 1) for i, expr in enumerate(dlt.returning)
        )
        out.append(SetResultSchema(columns=ret_cols))
    out.extend([
        OpenScan(cursor_id=cid, table=dlt.table),
        Label(name=loop),
        AdvanceCursor(cursor_id=cid, on_exhausted=end),
    ])
    if dlt.predicate is not None:
        out.extend(_compile_expr(dlt.predicate, ctx))
        out.append(JumpIfFalse(label=skip))
    # RETURNING: emit columns BEFORE DeleteRows — ``st.current_row[cid]``
    # still holds the row at this point.  After DeleteRows it is removed.
    if dlt.returning:
        out.append(BeginRow())
        for i, ret_expr in enumerate(dlt.returning):
            out.extend(_compile_returning_cursor_expr(ret_expr, ctx, cid))
            out.append(EmitColumn(name=_returning_col_name(ret_expr, i + 1)))
        out.append(EmitRow())
    out.append(DeleteRows(table=dlt.table, cursor_id=cid))
    if dlt.predicate is not None:
        out.append(Label(name=skip))
    out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
    return out


# --------------------------------------------------------------------------
# Enum / type mappers between planner and IR.
# --------------------------------------------------------------------------


_BINOP_MAP = {
    AstBinaryOp.ADD: BinaryOpCode.ADD,
    AstBinaryOp.SUB: BinaryOpCode.SUB,
    AstBinaryOp.MUL: BinaryOpCode.MUL,
    AstBinaryOp.DIV: BinaryOpCode.DIV,
    AstBinaryOp.MOD: BinaryOpCode.MOD,
    AstBinaryOp.EQ: BinaryOpCode.EQ,
    AstBinaryOp.NOT_EQ: BinaryOpCode.NEQ,
    AstBinaryOp.LT: BinaryOpCode.LT,
    AstBinaryOp.LTE: BinaryOpCode.LTE,
    AstBinaryOp.GT: BinaryOpCode.GT,
    AstBinaryOp.GTE: BinaryOpCode.GTE,
    AstBinaryOp.AND: BinaryOpCode.AND,
    AstBinaryOp.OR: BinaryOpCode.OR,
}


def _binop_to_ir(op: AstBinaryOp) -> BinaryOpCode:
    return _BINOP_MAP[op]


def _unop_to_ir(op: AstUnaryOp) -> UnaryOpCode:
    if op is AstUnaryOp.NEG:
        return UnaryOpCode.NEG
    return UnaryOpCode.NOT


def _plan_agg_to_ir(f: PlanAggFunc) -> IrAggFunc:
    return IrAggFunc[f.name]


def _to_ir_col(c: AstColumnDef) -> IrColumnDef:
    check_instrs: tuple[Instruction, ...] = ()
    if c.check_expr is not None:
        # Compile the CHECK expression using a synthetic context where every
        # unqualified column reference resolves to the sentinel CHECK_CURSOR_ID.
        # The VM will bind this cursor to the incoming row at validation time.
        from .ir import CHECK_CURSOR_ID
        check_ctx = _Ctx()
        check_ctx.alias_to_cursor[""] = CHECK_CURSOR_ID
        check_instrs = tuple(_compile_expr(c.check_expr, check_ctx))  # type: ignore[arg-type]
    fk: tuple[str, str | None] | None = None
    if c.foreign_key is not None:
        fk = c.foreign_key  # type: ignore[assignment]
    return IrColumnDef(
        name=c.name,
        type=c.type_name,
        nullable=not c.effective_not_null(),
        primary_key=c.primary_key,
        check_instrs=check_instrs,
        foreign_key=fk,
    )


def _to_sort_key(k: object) -> SortKey:
    from sql_planner.plan import SortKey as PlanSortKey

    assert isinstance(k, PlanSortKey)
    col = _column_display_name(k.expr) or "?"
    direction = Direction.DESC if k.descending else Direction.ASC
    nulls = NullsOrder.FIRST if k.nulls_first else NullsOrder.LAST
    return SortKey(column=col, direction=direction, nulls=nulls)


# Map lower-case window function names to the IR WinFunc enum values.
_WIN_FUNC_MAP: dict[str, WinFunc] = {
    "row_number": WinFunc.ROW_NUMBER,
    "rank": WinFunc.RANK,
    "dense_rank": WinFunc.DENSE_RANK,
    "sum": WinFunc.SUM,
    "count": WinFunc.COUNT,
    "count_star": WinFunc.COUNT_STAR,
    "avg": WinFunc.AVG,
    "min": WinFunc.MIN,
    "max": WinFunc.MAX,
    "first_value": WinFunc.FIRST_VALUE,
    "last_value": WinFunc.LAST_VALUE,
    # Offset / navigation functions (SQL:2003)
    "lag": WinFunc.LAG,
    "lead": WinFunc.LEAD,
    "nth_value": WinFunc.NTH_VALUE,
    "ntile": WinFunc.NTILE,
    "percent_rank": WinFunc.PERCENT_RANK,
    "cume_dist": WinFunc.CUME_DIST,
}


def _to_ir_win_spec(spec: PlanWindowFuncSpec) -> WinFuncSpec:
    """Convert a planner-level WindowFuncSpec to an IR WinFuncSpec.

    Primary column arguments (``arg_expr``) must be :class:`Column` references
    — the planner ensures dependency columns are present in the inner projection
    with matching names.

    Extra scalar arguments (``extra_args``) must be :class:`Literal` nodes
    that evaluate to Python primitives at codegen time.  The VM reads them from
    ``WinFuncSpec.extra_args`` rather than from the result-row buffer.

    LAG / LEAD defaults when extra_args are omitted
    -------------------------------------------------
    ``LAG(col)``         → offset=1, default=None
    ``LAG(col, 2)``      → offset=2, default=None
    ``LAG(col, 2, 0)``   → offset=2, default=0
    """
    func_key = spec.func.lower()
    ir_func = _WIN_FUNC_MAP.get(func_key)
    if ir_func is None:
        raise UnsupportedNode(f"unknown window function: {spec.func!r}")

    def _col_name(e: object) -> str:
        if isinstance(e, Column):
            return e.col
        raise UnsupportedNode(
            "window function partition/order/arg must be a column reference, "
            f"got {type(e).__name__}"
        )

    def _literal_val(e: Expr, ctx: str) -> object:
        """Extract a Python constant from a Literal node.

        Also handles the common case where the parser produces a negated
        literal (``-1``) as ``UnaryExpr(NEG, Literal(1))`` rather than
        ``Literal(-1)``.  Only the unary-minus case is folded; all other
        expression types raise ``UnsupportedNode``.
        """
        if isinstance(e, Literal):
            return e.value
        if (
            isinstance(e, UnaryExpr)
            and e.op == AstUnaryOp.NEG
            and isinstance(e.operand, Literal)
        ):
            v = e.operand.value
            if isinstance(v, (int, float)):
                return -v
        raise UnsupportedNode(
            f"window function extra argument ({ctx}) must be a literal constant, "
            f"got {type(e).__name__}"
        )

    # --- Partition / order columns ---
    partition_cols = tuple(_col_name(e) for e in spec.partition_by)
    order_cols = tuple((_col_name(e), desc) for e, desc in spec.order_by)

    # --- Extra scalar arguments + primary arg_col ---
    # NTILE is special: its first (and only) argument is the bucket-count
    # literal, NOT a column reference.  We handle it here, before the generic
    # ``arg_col = _col_name(spec.arg_expr)`` path which would wrongly call
    # _col_name on a Literal and raise UnsupportedNode.
    extra: tuple[object, ...]
    arg_col: str | None = None

    if ir_func in (WinFunc.LAG, WinFunc.LEAD):
        # Primary arg is a column reference.
        if spec.arg_expr is not None:
            arg_col = _col_name(spec.arg_expr)
        # Normalise to exactly 2 extra slots: (offset, default_value).
        # SQL: LAG(col [, offset [, default]])
        offset_val: object = 1         # default offset
        default_val: object = None     # default replacement
        if len(spec.extra_args) >= 1:
            offset_val = _literal_val(spec.extra_args[0], "LAG/LEAD offset")
        if len(spec.extra_args) >= 2:
            default_val = _literal_val(spec.extra_args[1], "LAG/LEAD default")
        extra = (offset_val, default_val)

    elif ir_func == WinFunc.NTILE:
        # NTILE(n): arg_expr holds the Literal(n) — it is not a column.
        # Move n into extra_args and leave arg_col as None.
        if spec.arg_expr is None:
            raise UnsupportedNode("NTILE requires a bucket-count argument")
        n_val = _literal_val(spec.arg_expr, "NTILE n")
        arg_col = None          # NTILE has no column arg
        extra = (n_val,)

    elif ir_func == WinFunc.NTH_VALUE:
        # NTH_VALUE(col, n): col is the primary arg; n is in extra_args[0].
        if spec.arg_expr is not None:
            arg_col = _col_name(spec.arg_expr)
        if not spec.extra_args:
            raise UnsupportedNode("NTH_VALUE requires two arguments: NTH_VALUE(col, n)")
        extra = (_literal_val(spec.extra_args[0], "NTH_VALUE n"),)

    else:
        # All other functions: primary arg is an optional column reference.
        if spec.arg_expr is not None:
            arg_col = _col_name(spec.arg_expr)
        extra = ()

    return WinFuncSpec(
        func=ir_func,
        arg_col=arg_col,
        partition_cols=partition_cols,
        order_cols=order_cols,
        result_col=spec.alias,
        extra_args=extra,
    )
