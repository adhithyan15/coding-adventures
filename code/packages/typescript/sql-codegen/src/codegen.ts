/**
 * SQL code generator — compiles a LogicalPlan into an IR Program.
 *
 * The code generator performs structural translation of the LogicalPlan tree
 * into flat bytecode instructions for the sql-vm to execute.
 *
 * Key design decisions:
 *
 *   1. Scan loop pattern (ScanNode):
 *      OpenScan → Label loop → JumpIfExhausted done → <inner> → AdvanceCursor → Jump loop → Label done → CloseScan
 *
 *   2. Filter pattern (FilterNode):
 *      Wraps the scan loop — compileExpr predicate, JumpIfFalse skip, <inner>, Label skip
 *
 *   3. Project pattern (ProjectNode):
 *      BeginRow → compileExpr each item → EmitColumn → EmitRow
 *
 *   4. Aggregate over project (ProjectNode → AggregateNode):
 *      Compiles as a single unit:
 *        Phase 1 — scan loop accumulates: SaveGroupKey + UpdateAgg per row
 *        Phase 2 — group emit loop: FinalizeAgg (→ aggBuffer), optional HAVING,
 *                  BeginRow + project items (read aggBuffer via LoadColumn -2,
 *                  group keys via LoadGroupKey) + EmitRow
 *
 *   5. Post-processing (SortNode, LimitNode, DistinctNode):
 *      Peeled from the outer plan in compileRoot, emitted after the main loop.
 */

import type {
  AggregateNode,
  AggregateSpec,
  ColumnDefinition,
  Expr,
  FilterNode,
  HavingNode,
  InsertNode,
  JoinNode,
  LogicalPlan,
  ProjectNode,
  SortKey,
  SortNode,
} from "@coding-adventures/sql-planner";
import { buildLabelIndex } from "./ir.js";
import type { ColumnSpec, Instruction, Program, SortSpec } from "./ir.js";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Compile a LogicalPlan into an executable Program. */
export function compile(plan: LogicalPlan): Program {
  const ctx = new CompileContext();
  ctx.compileRoot(plan);
  ctx.emit({ op: "Halt" });
  const labels = buildLabelIndex(ctx.instructions);
  return { instructions: ctx.instructions, labels, resultSchema: ctx.resultSchema };
}

// ---------------------------------------------------------------------------
// Compile context
// ---------------------------------------------------------------------------

class CompileContext {
  readonly instructions: Instruction[] = [];
  resultSchema: string[] = [];
  private labelCounter = 0;
  private cursorCounter = 0;

  /** When inside a project-over-aggregate, holds the aggregate node for expr resolution. */
  private currentAggNode: AggregateNode | null = null;

  emit(instr: Instruction): void {
    this.instructions.push(instr);
  }

  freshLabel(prefix: string): string {
    return `${prefix}_${this.labelCounter++}`;
  }

  freshCursor(): number {
    return this.cursorCounter++;
  }

  /** Compile the root plan node, peeling post-processing wrappers. */
  compileRoot(plan: LogicalPlan): void {
    const postOps: LogicalPlan[] = [];
    let inner = plan;
    while (
      inner.type === "sort" ||
      inner.type === "limit" ||
      inner.type === "distinct"
    ) {
      postOps.unshift(inner);
      inner = (inner as { input: LogicalPlan }).input;
    }

    this.compilePlan(inner, null);

    for (const op of postOps) {
      if (op.type === "sort") {
        this.emitSort(op);
      } else if (op.type === "limit") {
        this.emit({ op: "LimitResult", count: op.count, offset: op.offset });
      } else if (op.type === "distinct") {
        this.emit({ op: "DistinctResult" });
      }
    }
  }

  private emitSort(node: SortNode): void {
    const keys: SortSpec[] = node.keys.map((k) => {
      const baseName = sortKeyToColumnName(k);
      // When the sort key is not in the visible result schema, the planner added
      // a hidden "__sort_<key>" column to carry the sort value.  Use that name so
      // SortResult can find the column; it will strip the prefix afterward.
      const column = this.resultSchema.includes(baseName)
        ? baseName
        : `__sort_${exprKey(k.expr)}`;
      return { column, ascending: k.ascending, nullsLast: k.nullsLast };
    });
    this.emit({ op: "SortResult", keys, stripPrefix: "__sort_" });
  }

  /** Compile a plan node. innerCode is a callback invoked for each source row (scan loop body). */
  compilePlan(plan: LogicalPlan, innerCode: (() => void) | null): void {
    switch (plan.type) {
      case "scan":
        this.compileScan(plan.table, innerCode);
        break;

      case "filter":
        this.compileFilter(plan as FilterNode, innerCode);
        break;

      case "project":
        this.compileProject(plan as ProjectNode, innerCode);
        break;

      case "aggregate":
        this.compileAggregatePhase1(plan as AggregateNode);
        this.compileAggregatePhase2Default(plan as AggregateNode);
        break;

      case "having":
        this.compileHavingBare(plan as HavingNode);
        break;

      case "join":
        this.compileJoin(plan as JoinNode, innerCode);
        break;

      case "insert":
        this.compileInsert(plan as InsertNode);
        break;

      case "update":
        this.compileUpdate(plan as { type: "update"; table: string; assignments: { column: string; value: Expr }[]; predicate: Expr | null });
        break;

      case "delete":
        this.compileDelete(plan as { type: "delete"; table: string; predicate: Expr | null });
        break;

      case "create_table":
        this.emit({
          op: "CreateTable",
          table: plan.table,
          columns: (plan as { columns: ColumnDefinition[] }).columns.map(colDefToSpec),
          ifNotExists: (plan as { ifNotExists: boolean }).ifNotExists,
        });
        break;

      case "drop_table":
        this.emit({ op: "DropTable", table: plan.table, ifExists: (plan as { ifExists: boolean }).ifExists });
        break;

      case "empty_result":
        break;

      case "sort":
      case "limit":
      case "distinct": {
        const nested = (plan as { input: LogicalPlan }).input;
        this.compilePlan(nested, innerCode);
        break;
      }

      default:
        throw new CodegenError(`unsupported plan node type: ${(plan as { type: string }).type}`);
    }
  }

  // ---------------------------------------------------------------------------
  // Scan
  // ---------------------------------------------------------------------------

  private compileScan(table: string, innerCode: (() => void) | null): void {
    const curId = this.freshCursor();
    const loopLabel = this.freshLabel(`loop_${table}`);
    const doneLabel = this.freshLabel(`done_${table}`);

    this.emit({ op: "OpenScan", cursorId: curId, table });
    this.emit({ op: "Label", name: loopLabel });
    this.emit({ op: "JumpIfExhausted", cursorId: curId, label: doneLabel });

    if (innerCode) innerCode();

    this.emit({ op: "AdvanceCursor", cursorId: curId });
    this.emit({ op: "Jump", label: loopLabel });
    this.emit({ op: "Label", name: doneLabel });
    this.emit({ op: "CloseScan", cursorId: curId });
  }

  // ---------------------------------------------------------------------------
  // Filter
  // ---------------------------------------------------------------------------

  private compileFilter(node: FilterNode, innerCode: (() => void) | null): void {
    const skipLabel = this.freshLabel("skip");
    this.compilePlan(node.input, () => {
      this.compileExpr(node.predicate, -1);
      this.emit({ op: "JumpIfFalse", label: skipLabel });
      if (innerCode) innerCode();
      this.emit({ op: "Label", name: skipLabel });
    });
  }

  // ---------------------------------------------------------------------------
  // Project
  // ---------------------------------------------------------------------------

  private compileProject(plan: ProjectNode, _innerCode: (() => void) | null): void {
    const aggNode = peelToAggregate(plan.input);
    const havingPred = plan.input.type === "having" ? (plan.input as HavingNode).predicate : null;

    if (aggNode) {
      this.compileProjectOverAggregate(plan, aggNode, havingPred);
      return;
    }

    // Normal scan-based project.
    const outputCols: string[] = [];
    for (const item of plan.items) {
      outputCols.push(item.expr.kind === "star" ? "*" : (item.alias ?? exprOutputName(item.expr)));
    }
    this.resultSchema = outputCols.filter((c) => !c.startsWith("__sort_"));

    this.compilePlan(plan.input, () => {
      this.emit({ op: "BeginRow" });
      for (const item of plan.items) {
        if (item.expr.kind === "star") {
          this.emit({ op: "LoadConst", value: "*" });
          this.emit({ op: "EmitColumn", name: "__star__" });
        } else {
          const name = item.alias ?? exprOutputName(item.expr);
          this.compileExpr(item.expr, -1);
          this.emit({ op: "EmitColumn", name });
        }
      }
      this.emit({ op: "EmitRow" });
    });
  }

  // ---------------------------------------------------------------------------
  // Project + Aggregate (compiled as a single unit)
  // ---------------------------------------------------------------------------

  /**
   * Compile ProjectNode → [HavingNode →] AggregateNode → ... as a single unit.
   *
   * FinalizeAgg stores results in vm.aggBuffer (separate from rowBuffer).
   * LoadColumn cursorId=-2 reads from aggBuffer.
   * BeginRow only clears rowBuffer, so aggBuffer values survive.
   */
  private compileProjectOverAggregate(
    projNode: ProjectNode,
    aggNode: AggregateNode,
    havingPred: Expr | null,
  ): void {
    // Phase 1: accumulate.
    // currentAggNode must NOT be set here — compileExpr for GROUP BY key columns
    // must emit LoadColumn (read from the scan cursor), not LoadGroupKey (which
    // reads from groupKeys[], which is empty during accumulation).
    this.compileAggregatePhase1(aggNode);

    // Set currentAggNode only after Phase 1 so Phase 2 expr resolution works.
    this.currentAggNode = aggNode;

    // Phase 2: group emit.
    const groupLoopLabel = this.freshLabel("group_loop");
    const groupDoneLabel = this.freshLabel("group_done");
    const skipLabel = this.freshLabel("group_skip");

    this.emit({ op: "Label", name: groupLoopLabel });
    this.emit({ op: "JumpIfGroupsDone", label: groupDoneLabel });

    // Finalize all agg slots → aggBuffer (separate from rowBuffer).
    for (let i = 0; i < aggNode.aggregates.length; i++) {
      const spec = aggNode.aggregates[i];
      this.emit({ op: "FinalizeAgg", slot: i, func: spec.func, alias: spec.alias });
      this.emit({ op: "Pop" }); // discard from stack; value is in aggBuffer[spec.alias]
    }

    if (havingPred) {
      this.compileExpr(havingPred, -1);
      this.emit({ op: "JumpIfFalse", label: skipLabel });
    }

    // Emit the projected row.
    this.emit({ op: "BeginRow" });
    for (const item of projNode.items) {
      const name = item.alias ?? exprOutputName(item.expr);
      this.compileExpr(item.expr, -1);
      this.emit({ op: "EmitColumn", name });
    }
    this.emit({ op: "EmitRow" });

    this.emit({ op: "Label", name: skipLabel });
    this.emit({ op: "AdvanceGroup" });
    this.emit({ op: "Jump", label: groupLoopLabel });
    this.emit({ op: "Label", name: groupDoneLabel });

    this.currentAggNode = null;

    const outputCols = projNode.items.map((item) =>
      item.alias ?? exprOutputName(item.expr)
    );
    this.resultSchema = outputCols.filter((c) => !c.startsWith("__sort_"));
  }

  // ---------------------------------------------------------------------------
  // Aggregate helpers
  // ---------------------------------------------------------------------------

  private compileAggregatePhase1(aggNode: AggregateNode): void {
    this.emit({ op: "InitAgg", slots: aggNode.aggregates.length });

    this.compilePlan(aggNode.input, () => {
      if (aggNode.keys.length > 0) {
        for (const keyExpr of aggNode.keys) {
          this.compileExpr(keyExpr, -1);
        }
        this.emit({ op: "SaveGroupKey", arity: aggNode.keys.length });
      }
      for (let i = 0; i < aggNode.aggregates.length; i++) {
        const spec = aggNode.aggregates[i];
        if (spec.arg !== null) {
          this.compileExpr(spec.arg, -1);
        } else {
          this.emit({ op: "LoadConst", value: 1 });
        }
        this.emit({ op: "UpdateAgg", slot: i, func: spec.func });
      }
    });
  }

  private compileAggregatePhase2Default(aggNode: AggregateNode): void {
    const groupLoopLabel = this.freshLabel("group_loop");
    const groupDoneLabel = this.freshLabel("group_done");

    this.emit({ op: "Label", name: groupLoopLabel });
    this.emit({ op: "JumpIfGroupsDone", label: groupDoneLabel });

    this.emit({ op: "BeginRow" });
    for (let i = 0; i < aggNode.keys.length; i++) {
      this.emit({ op: "LoadGroupKey", slot: i });
      this.emit({ op: "EmitColumn", name: exprOutputName(aggNode.keys[i]) });
    }
    for (let i = 0; i < aggNode.aggregates.length; i++) {
      const spec = aggNode.aggregates[i];
      this.emit({ op: "FinalizeAgg", slot: i, func: spec.func, alias: spec.alias });
      this.emit({ op: "EmitColumn", name: spec.alias });
    }
    this.emit({ op: "EmitRow" });

    this.emit({ op: "AdvanceGroup" });
    this.emit({ op: "Jump", label: groupLoopLabel });
    this.emit({ op: "Label", name: groupDoneLabel });
  }

  private compileHavingBare(node: HavingNode): void {
    if (node.input.type !== "aggregate") {
      this.compilePlan(node.input, null);
      return;
    }
    const aggNode = node.input as AggregateNode;
    this.compileAggregatePhase1(aggNode);

    const groupLoopLabel = this.freshLabel("group_loop_hav");
    const groupDoneLabel = this.freshLabel("group_done_hav");
    const skipLabel = this.freshLabel("having_skip");

    this.emit({ op: "Label", name: groupLoopLabel });
    this.emit({ op: "JumpIfGroupsDone", label: groupDoneLabel });

    for (let i = 0; i < aggNode.aggregates.length; i++) {
      const spec = aggNode.aggregates[i];
      this.emit({ op: "FinalizeAgg", slot: i, func: spec.func, alias: spec.alias });
      this.emit({ op: "Pop" });
    }

    this.currentAggNode = aggNode;
    this.compileExpr(node.predicate, -1);
    this.currentAggNode = null;

    this.emit({ op: "JumpIfFalse", label: skipLabel });

    this.emit({ op: "BeginRow" });
    for (let i = 0; i < aggNode.keys.length; i++) {
      this.emit({ op: "LoadGroupKey", slot: i });
      this.emit({ op: "EmitColumn", name: exprOutputName(aggNode.keys[i]) });
    }
    for (let i = 0; i < aggNode.aggregates.length; i++) {
      const spec = aggNode.aggregates[i];
      this.emit({ op: "LoadColumn", cursorId: -2, column: spec.alias });
      this.emit({ op: "EmitColumn", name: spec.alias });
    }
    this.emit({ op: "EmitRow" });

    this.emit({ op: "Label", name: skipLabel });
    this.emit({ op: "AdvanceGroup" });
    this.emit({ op: "Jump", label: groupLoopLabel });
    this.emit({ op: "Label", name: groupDoneLabel });
  }

  // ---------------------------------------------------------------------------
  // Join
  // ---------------------------------------------------------------------------

  private compileJoin(node: JoinNode, innerCode: (() => void) | null): void {
    const rightCurId = this.freshCursor();
    const rightTable = node.right.type === "scan"
      ? (node.right as { type: "scan"; table: string }).table
      : "__join_right__";
    const loopLabel = this.freshLabel("join_inner_loop");
    const doneLabel = this.freshLabel("join_inner_done");
    const skipLabel = this.freshLabel("join_skip");

    this.compilePlan(node.left, () => {
      this.emit({ op: "OpenScan", cursorId: rightCurId, table: rightTable });
      this.emit({ op: "Label", name: loopLabel });
      this.emit({ op: "JumpIfExhausted", cursorId: rightCurId, label: doneLabel });

      if (node.condition) {
        this.compileExpr(node.condition, -1);
        this.emit({ op: "JumpIfFalse", label: skipLabel });
      }

      if (innerCode) innerCode();

      this.emit({ op: "Label", name: skipLabel });
      this.emit({ op: "AdvanceCursor", cursorId: rightCurId });
      this.emit({ op: "Jump", label: loopLabel });
      this.emit({ op: "Label", name: doneLabel });
      this.emit({ op: "CloseScan", cursorId: rightCurId });
    });
  }

  // ---------------------------------------------------------------------------
  // INSERT
  // ---------------------------------------------------------------------------

  private compileInsert(node: InsertNode): void {
    for (const row of node.rows) {
      for (const val of row) {
        this.compileExpr(val, -1);
      }
      this.emit({ op: "InsertRow", table: node.table, columns: node.columns });
    }
  }

  // ---------------------------------------------------------------------------
  // UPDATE
  // ---------------------------------------------------------------------------

  private compileUpdate(node: { type: "update"; table: string; assignments: { column: string; value: Expr }[]; predicate: Expr | null }): void {
    const curId = this.freshCursor();
    const loopLabel = this.freshLabel("upd_loop");
    const doneLabel = this.freshLabel("upd_done");
    const skipLabel = this.freshLabel("upd_skip");
    const columns = node.assignments.map((a) => a.column);

    this.emit({ op: "OpenScan", cursorId: curId, table: node.table });
    this.emit({ op: "Label", name: loopLabel });
    this.emit({ op: "JumpIfExhausted", cursorId: curId, label: doneLabel });

    if (node.predicate) {
      this.compileExpr(node.predicate, curId);
      this.emit({ op: "JumpIfFalse", label: skipLabel });
    }

    for (const assignment of node.assignments) {
      this.compileExpr(assignment.value, curId);
    }
    this.emit({ op: "UpdateRows", table: node.table, columns, cursorId: curId });

    this.emit({ op: "Label", name: skipLabel });
    this.emit({ op: "AdvanceCursor", cursorId: curId });
    this.emit({ op: "Jump", label: loopLabel });
    this.emit({ op: "Label", name: doneLabel });
    this.emit({ op: "CloseScan", cursorId: curId });
  }

  // ---------------------------------------------------------------------------
  // DELETE
  // ---------------------------------------------------------------------------

  private compileDelete(node: { type: "delete"; table: string; predicate: Expr | null }): void {
    const curId = this.freshCursor();
    const loopLabel = this.freshLabel("del_loop");
    const doneLabel = this.freshLabel("del_done");
    const skipLabel = this.freshLabel("del_skip");

    this.emit({ op: "OpenScan", cursorId: curId, table: node.table });
    this.emit({ op: "Label", name: loopLabel });
    this.emit({ op: "JumpIfExhausted", cursorId: curId, label: doneLabel });

    if (node.predicate) {
      this.compileExpr(node.predicate, curId);
      this.emit({ op: "JumpIfFalse", label: skipLabel });
    }

    this.emit({ op: "DeleteRows", table: node.table, cursorId: curId });

    this.emit({ op: "Label", name: skipLabel });
    this.emit({ op: "AdvanceCursor", cursorId: curId });
    this.emit({ op: "Jump", label: loopLabel });
    this.emit({ op: "Label", name: doneLabel });
    this.emit({ op: "CloseScan", cursorId: curId });
  }

  // ---------------------------------------------------------------------------
  // Expression compilation
  // ---------------------------------------------------------------------------

  compileExpr(expr: Expr, cursorId: number): void {
    switch (expr.kind) {
      case "literal":
        if (expr.value === null) {
          this.emit({ op: "LoadNull" });
        } else {
          this.emit({ op: "LoadConst", value: expr.value });
        }
        break;

      case "column": {
        // When inside a project-over-aggregate, check if this column is a group key.
        if (this.currentAggNode) {
          const keyIdx = this.currentAggNode.keys.findIndex((k) =>
            k.kind === "column" && k.name === expr.name
          );
          if (keyIdx >= 0) {
            this.emit({ op: "LoadGroupKey", slot: keyIdx });
            break;
          }
        }
        this.emit({ op: "LoadColumn", cursorId, column: expr.name });
        break;
      }

      case "star":
        this.emit({ op: "LoadConst", value: "*" });
        break;

      case "binary":
        this.compileExpr(expr.left, cursorId);
        this.compileExpr(expr.right, cursorId);
        this.emit({ op: "BinaryOp", operator: expr.op });
        break;

      case "unary":
        this.compileExpr(expr.operand, cursorId);
        this.emit({ op: "UnaryOp", operator: expr.op });
        break;

      case "func":
        for (const arg of expr.args) {
          this.compileExpr(arg, cursorId);
        }
        this.emit({ op: "CallFunc", name: expr.name, arity: expr.args.length });
        break;

      case "aggregate": {
        if (this.currentAggNode) {
          const spec = findAggSpec(this.currentAggNode.aggregates, expr);
          if (spec) {
            this.emit({ op: "LoadColumn", cursorId: -2, column: spec.alias });
            break;
          }
        }
        this.emit({ op: "LoadNull" });
        break;
      }

      case "between":
        this.compileExpr(expr.expr, cursorId);
        this.compileExpr(expr.low, cursorId);
        this.compileExpr(expr.high, cursorId);
        this.emit({ op: "BetweenInstr", negated: expr.negated });
        break;

      case "in_list":
        this.compileExpr(expr.expr, cursorId);
        for (const item of expr.list) {
          this.compileExpr(item, cursorId);
        }
        this.emit({ op: "InList", count: expr.list.length, negated: expr.negated });
        break;

      case "like":
        this.compileExpr(expr.expr, cursorId);
        this.compileExpr(expr.pattern, cursorId);
        this.emit({ op: "LikeInstr", negated: expr.negated });
        break;

      case "is_null":
        this.compileExpr(expr.expr, cursorId);
        this.emit(expr.negated ? { op: "IsNotNullInstr" } : { op: "IsNullInstr" });
        break;

      case "coalesce":
        for (const arg of expr.args) {
          this.compileExpr(arg, cursorId);
        }
        this.emit({ op: "Coalesce", arity: expr.args.length });
        break;

      default:
        this.emit({ op: "LoadNull" });
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export class CodegenError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CodegenError";
  }
}

function peelToAggregate(plan: LogicalPlan): AggregateNode | null {
  if (plan.type === "aggregate") return plan as AggregateNode;
  if (plan.type === "having" && plan.input.type === "aggregate") {
    return plan.input as AggregateNode;
  }
  return null;
}

function findAggSpec(
  specs: AggregateSpec[],
  expr: { func: string; arg: Expr | null; distinct: boolean },
): AggregateSpec | undefined {
  const key = `${expr.func}|${exprKey(expr.arg)}|${expr.distinct}`;
  return specs.find((s) => `${s.func}|${exprKey(s.arg)}|${s.distinct}` === key);
}

function exprKey(expr: Expr | null): string {
  if (!expr) return "__null__";
  switch (expr.kind) {
    case "literal": return `lit:${String(expr.value)}`;
    case "column": return expr.table ? `${expr.table}.${expr.name}` : expr.name;
    case "star": return "*";
    case "binary": return `(${exprKey(expr.left)}${expr.op}${exprKey(expr.right)})`;
    case "unary": return `${expr.op}(${exprKey(expr.operand)})`;
    case "func": return `${expr.name}(${expr.args.map(exprKey).join(",")})`;
    case "aggregate": return `agg:${expr.func}(${exprKey(expr.arg)})`;
    case "coalesce": return `coalesce(${expr.args.map(exprKey).join(",")})`;
    case "between": return `between(${exprKey(expr.expr)},${exprKey(expr.low)},${exprKey(expr.high)})`;
    case "in_list": return `in(${exprKey(expr.expr)},[${expr.list.map(exprKey).join(",")}])`;
    case "like": return `like(${exprKey(expr.expr)},${exprKey(expr.pattern)})`;
    case "is_null": return `is_null(${exprKey(expr.expr)})`;
    default: return "unknown";
  }
}

function exprOutputName(expr: Expr): string {
  switch (expr.kind) {
    case "column": return expr.name;
    case "literal": return String(expr.value);
    case "aggregate": {
      const arg = expr.arg ? exprOutputName(expr.arg) : "*";
      return `${expr.func}(${arg})`;
    }
    case "func": return `${expr.name}(${expr.args.map(exprOutputName).join(",")})`;
    case "binary": return `${exprOutputName(expr.left)}${expr.op}${exprOutputName(expr.right)}`;
    case "star": return "*";
    default: return "expr";
  }
}

function sortKeyToColumnName(key: SortKey): string {
  switch (key.expr.kind) {
    case "column": return key.expr.name;
    case "literal": return String(key.expr.value);
    default: return exprOutputName(key.expr);
  }
}

function colDefToSpec(col: ColumnDefinition): ColumnSpec {
  let defVal = null;
  if (col.defaultValue && col.defaultValue.kind === "literal") {
    defVal = col.defaultValue.value as (null | boolean | number | string);
  }
  return {
    name: col.name,
    dataType: col.dataType,
    notNull: col.notNull,
    primaryKey: col.primaryKey,
    unique: col.unique,
    defaultValue: defVal,
  };
}
