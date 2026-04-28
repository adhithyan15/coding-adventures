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
    Intersect,
    Join,
    JoinKind,
    Literal,
    LogicalPlan,
    NotIn,
    Project,
    Rollback,
    Scan,
    Sort,
    UnaryExpr,
    Union,
    Update,
    Wildcard,
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
    DropIndex as PlanDropIndex,
)
from sql_planner import (
    DropTable as PlanDropTable,
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
    UnaryOp as AstUnaryOp,
)
from sql_planner.ast import ColumnDef as AstColumnDef
from sql_planner.expr import AggregateExpr
from sql_planner.plan import AggFunc as PlanAggFunc
from sql_planner.plan import Limit as PlanLimit

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
    Label,
    Like,
    LimitResult,
    LoadColumn,
    LoadConst,
    LoadGroupKey,
    NullsOrder,
    OpenIndexScan,
    OpenScan,
    Program,
    RollbackTransaction,
    RunExistsSubquery,
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
    """

    cursor_counter: int = 0
    label_counter: int = 0
    agg_counter: int = 0
    alias_to_cursor: dict[str, int] = field(default_factory=dict)

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
        out: list[Instruction] = []
        if predicate is not None:
            out.extend(_compile_expr(predicate, c))
            out.append(JumpIfFalse(label=_skip_label))
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
            out.append(Label(name=_skip_label))
        return out

    # Unique skip label per SELECT body.
    _skip_label = ctx.new_label("filter_skip")
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

    # LEFT / RIGHT / FULL — not yet implemented; raise a clear error.
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
                    if a.func is f and (
                        (arg is None and a.arg.star)
                        or (a.arg.value == arg)
                    ):
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
            inner_ctx = _Ctx()
            inner_instrs, _ = _compile_plan(inner_plan, inner_ctx)  # type: ignore[arg-type]
            inner_instrs.append(Halt())
            inner_resolved = _resolve_labels(inner_instrs)
            sub = Program(
                instructions=tuple(inner_instrs),
                labels=inner_resolved,
                result_schema=(),
            )
            return [RunExistsSubquery(sub_program=sub)]
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


def _compile_insert(ins: Insert, ctx: _Ctx) -> list[Instruction]:
    src = ins.source
    cols = ins.columns or ()
    if src.values is not None:
        out: list[Instruction] = []
        for row in src.values:
            for v in row:
                out.extend(_compile_expr(v, ctx))
            out.append(InsertRow(table=ins.table, columns=tuple(cols)))
        return out
    # INSERT … SELECT: compile the sub-SELECT into the result buffer, then
    # drain it with InsertFromResult. _compile_plan is safe to call
    # recursively here — it shares the same _Ctx (cursor/label counters stay
    # globally unique) and it does NOT emit a Halt.
    assert src.query is not None
    select_instrs, _ = _compile_plan(src.query, ctx)
    select_instrs.append(InsertFromResult(table=ins.table, columns=tuple(cols)))
    return select_instrs


def _compile_update(upd: Update, ctx: _Ctx) -> list[Instruction]:
    cid = ctx.new_cursor(upd.table)
    loop = ctx.new_label("update_loop")
    end = ctx.new_label("update_end")
    skip = ctx.new_label("update_skip")
    out: list[Instruction] = [
        OpenScan(cursor_id=cid, table=upd.table),
        Label(name=loop),
        AdvanceCursor(cursor_id=cid, on_exhausted=end),
    ]
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
    if upd.predicate is not None:
        out.append(Label(name=skip))
    out.extend([Jump(label=loop), Label(name=end), CloseScan(cursor_id=cid)])
    return out


def _compile_delete(dlt: Delete, ctx: _Ctx) -> list[Instruction]:
    cid = ctx.new_cursor(dlt.table)
    loop = ctx.new_label("delete_loop")
    end = ctx.new_label("delete_end")
    skip = ctx.new_label("delete_skip")
    out: list[Instruction] = [
        OpenScan(cursor_id=cid, table=dlt.table),
        Label(name=loop),
        AdvanceCursor(cursor_id=cid, on_exhausted=end),
    ]
    if dlt.predicate is not None:
        out.extend(_compile_expr(dlt.predicate, ctx))
        out.append(JumpIfFalse(label=skip))
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
    return IrColumnDef(
        name=c.name,
        type=c.type_name,
        nullable=not c.effective_not_null(),
        check_instrs=check_instrs,
    )


def _to_sort_key(k: object) -> SortKey:
    from sql_planner.plan import SortKey as PlanSortKey

    assert isinstance(k, PlanSortKey)
    col = _column_display_name(k.expr) or "?"
    direction = Direction.DESC if k.descending else Direction.ASC
    nulls = NullsOrder.FIRST if k.nulls_first else NullsOrder.LAST
    return SortKey(column=col, direction=direction, nulls=nulls)
