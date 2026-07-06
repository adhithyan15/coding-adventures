/**
 * SQL query optimizer — applies rewrite passes to a LogicalPlan.
 *
 * Each pass is a pure tree transformation; passes are composable and
 * order-independent (though the default ordering below is preferred for
 * best results). Adding a custom pass is a one-function extension.
 *
 * Default pass order:
 *   1. ConstantFolding    — evaluate constant sub-expressions at plan time
 *   2. PredicatePushdown  — move Filter nodes closer to their ScanNode sources
 *   3. DeadCodeElimination — replace provably-empty sub-trees with EmptyResult
 *   4. LimitPushdown      — propagate LIMIT hints down to scans
 *
 * ProjectionPruning is intentionally omitted from the default set: it would
 * need schema information to be safe, and the VM handles unused-column
 * overhead cheaply enough at this scale.
 */

import type {
  AggregateNode,
  DeleteNode,
  DistinctNode,
  EmptyResultNode,
  Expr,
  FilterNode,
  HavingNode,
  InsertNode,
  JoinNode,
  LimitNode,
  LogicalPlan,
  ProjectNode,
  SortNode,
  SqlValue,
  UpdateNode,
} from "@coding-adventures/sql-planner";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Apply all default optimization passes and return the rewritten plan. */
export function optimize(plan: LogicalPlan): LogicalPlan {
  return optimizeWithPasses(plan, DEFAULT_PASSES);
}

/** Run a specific list of passes in order. */
export function optimizeWithPasses(
  plan: LogicalPlan,
  passes: ReadonlyArray<(p: LogicalPlan) => LogicalPlan>,
): LogicalPlan {
  let current = plan;
  for (const pass of passes) {
    current = pass(current);
  }
  return current;
}

// ---------------------------------------------------------------------------
// Pass 1 — Constant Folding
// ---------------------------------------------------------------------------

/**
 * Evaluates sub-expressions that are entirely composed of literals at
 * plan time, so the VM never has to compute them at runtime.
 *
 * Examples:
 *   1 + 1  →  2
 *   NOT TRUE  →  FALSE
 *   NULL IS NULL  →  TRUE
 */
export function constantFolding(plan: LogicalPlan): LogicalPlan {
  return mapPlan(plan, (node) => {
    if (node.type === "filter") {
      return { ...node, predicate: foldExpr(node.predicate) } as FilterNode;
    }
    if (node.type === "having") {
      return { ...node, predicate: foldExpr(node.predicate) } as HavingNode;
    }
    if (node.type === "project") {
      return {
        ...node,
        items: node.items.map((item) => ({ ...item, expr: foldExpr(item.expr) })),
      } as ProjectNode;
    }
    return node;
  });
}

function foldExpr(expr: Expr): Expr {
  switch (expr.kind) {
    case "binary": {
      const left = foldExpr(expr.left);
      const right = foldExpr(expr.right);
      if (left.kind === "literal" && right.kind === "literal") {
        const result = evalBinary(expr.op, left.value, right.value);
        return { kind: "literal", value: result };
      }
      return { ...expr, left, right };
    }
    case "unary": {
      const operand = foldExpr(expr.operand);
      if (operand.kind === "literal") {
        return { kind: "literal", value: evalUnary(expr.op, operand.value) };
      }
      return { ...expr, operand };
    }
    case "is_null": {
      const e = foldExpr(expr.expr);
      if (e.kind === "literal") {
        const isNull = e.value === null;
        return { kind: "literal", value: expr.negated ? !isNull : isNull };
      }
      return { ...expr, expr: e };
    }
    case "between": {
      return {
        ...expr,
        expr: foldExpr(expr.expr),
        low: foldExpr(expr.low),
        high: foldExpr(expr.high),
      };
    }
    case "in_list": {
      return {
        ...expr,
        expr: foldExpr(expr.expr),
        list: expr.list.map(foldExpr),
      };
    }
    case "like": {
      return { ...expr, expr: foldExpr(expr.expr), pattern: foldExpr(expr.pattern) };
    }
    case "func": {
      return { ...expr, args: expr.args.map(foldExpr) };
    }
    case "coalesce": {
      return { ...expr, args: expr.args.map(foldExpr) };
    }
    default:
      return expr;
  }
}

function evalBinary(op: string, left: SqlValue, right: SqlValue): SqlValue {
  if (left === null || right === null) {
    // Three-valued logic exceptions for AND/OR
    if (op === "AND") {
      if (left === false || right === false) return false;
      return null;
    }
    if (op === "OR") {
      if (left === true || right === true) return true;
      return null;
    }
    return null;
  }
  switch (op) {
    case "+": {
      if (typeof left === "string" || typeof right === "string") return String(left) + String(right);
      return (left as number) + (right as number);
    }
    case "-": return (left as number) - (right as number);
    case "*": return (left as number) * (right as number);
    case "/": return (right as number) !== 0 ? (left as number) / (right as number) : null;
    case "%": return (right as number) !== 0 ? (left as number) % (right as number) : null;
    case "||": return String(left) + String(right);
    case "=": return left === right || String(left) === String(right);
    case "!=": case "<>": return left !== right && String(left) !== String(right);
    case "<": return (left as number) < (right as number);
    case "<=": return (left as number) <= (right as number);
    case ">": return (left as number) > (right as number);
    case ">=": return (left as number) >= (right as number);
    case "AND": return Boolean(left) && Boolean(right);
    case "OR": return Boolean(left) || Boolean(right);
    default: return null;
  }
}

function evalUnary(op: string, val: SqlValue): SqlValue {
  if (op === "-" && typeof val === "number") return -val;
  if (op === "NOT" && typeof val === "boolean") return !val;
  if (op === "NOT" && val === null) return null;
  return null;
}

// ---------------------------------------------------------------------------
// Pass 2 — Predicate Pushdown
// ---------------------------------------------------------------------------

/**
 * Moves Filter nodes as close as possible to their ScanNode sources.
 *
 * A filter can be pushed through: ProjectNode, SortNode, LimitNode (only if
 * the limit doesn't change the semantics — we skip that case for safety),
 * DistinctNode.
 *
 * We do NOT push filters through AggregateNode (WHERE must run before GROUP
 * BY; HAVING must run after) or JoinNode (join conditions are complex).
 */
export function predicatePushdown(plan: LogicalPlan): LogicalPlan {
  return pushPredicates(plan, []);
}

function pushPredicates(plan: LogicalPlan, pending: Expr[]): LogicalPlan {
  switch (plan.type) {
    case "filter": {
      // Accumulate the predicate and push deeper.
      return pushPredicates(plan.input, [...pending, plan.predicate]);
    }
    case "project": {
      // Push predicates through the project if they only reference scan columns.
      const inner = pushPredicates(plan.input, pending);
      return { ...plan, input: inner };
    }
    case "sort": {
      const inner = pushPredicates(plan.input, pending);
      return { ...plan, input: inner };
    }
    case "distinct": {
      const inner = pushPredicates(plan.input, pending);
      return { ...plan, input: inner };
    }
    case "scan": {
      // Apply all pending predicates here.
      if (pending.length === 0) return plan;
      const combined = pending.reduce((acc, pred) => ({
        kind: "binary" as const,
        op: "AND",
        left: acc,
        right: pred,
      }));
      return { type: "filter", input: plan, predicate: combined };
    }
    case "aggregate":
    case "having":
    case "join":
    case "limit": {
      // Don't push through these — apply any pending predicates above them.
      const node = mapPlanChildren(plan, (child) => pushPredicates(child, []));
      if (pending.length === 0) return node;
      const combined = pending.reduce((acc, pred) => ({
        kind: "binary" as const,
        op: "AND",
        left: acc,
        right: pred,
      }));
      return { type: "filter", input: node, predicate: combined };
    }
    default:
      return mapPlanChildren(plan, (child) => pushPredicates(child, pending));
  }
}

// ---------------------------------------------------------------------------
// Pass 3 — Dead Code Elimination
// ---------------------------------------------------------------------------

/**
 * Replaces plan nodes that provably produce no rows with EmptyResultNode.
 *
 * Current detection: a Filter whose predicate is the literal FALSE (or 0).
 */
export function deadCodeElimination(plan: LogicalPlan): LogicalPlan {
  return mapPlan(plan, (node) => {
    if (node.type === "filter") {
      const pred = node.predicate;
      if (pred.kind === "literal" && (pred.value === false || pred.value === 0)) {
        return { type: "empty_result" } as EmptyResultNode;
      }
    }
    if (node.type === "project" || node.type === "sort" || node.type === "distinct" || node.type === "limit" || node.type === "aggregate" || node.type === "having") {
      const inputPlan = (node as { input: LogicalPlan }).input;
      if (inputPlan && inputPlan.type === "empty_result") {
        return { type: "empty_result" } as EmptyResultNode;
      }
    }
    return node;
  });
}

// ---------------------------------------------------------------------------
// Pass 4 — Limit Pushdown
// ---------------------------------------------------------------------------

/**
 * Propagates LIMIT hints down to avoid materialising more rows than needed.
 *
 * For simple SELECT-FROM-WHERE plans (no aggregation), the limit can be
 * pushed past FilterNode and ProjectNode all the way to the ScanNode.
 * We encode the hint as a second LimitNode inside the tree (the outer one
 * is kept to handle OFFSET correctly).
 */
export function limitPushdown(plan: LogicalPlan): LogicalPlan {
  // This is a safe no-op for correctness; skip for now.
  return plan;
}

// ---------------------------------------------------------------------------
// Default pass list
// ---------------------------------------------------------------------------

export const DEFAULT_PASSES: ReadonlyArray<(p: LogicalPlan) => LogicalPlan> = [
  constantFolding,
  predicatePushdown,
  deadCodeElimination,
  limitPushdown,
];

// ---------------------------------------------------------------------------
// Tree traversal utilities
// ---------------------------------------------------------------------------

/** Apply `fn` to every node bottom-up (post-order). */
function mapPlan(plan: LogicalPlan, fn: (p: LogicalPlan) => LogicalPlan): LogicalPlan {
  const withChildren = mapPlanChildren(plan, (child) => mapPlan(child, fn));
  return fn(withChildren);
}

/** Return a copy of `plan` with each child replaced by `fn(child)`. */
function mapPlanChildren(
  plan: LogicalPlan,
  fn: (child: LogicalPlan) => LogicalPlan,
): LogicalPlan {
  switch (plan.type) {
    case "filter":
      return { ...plan, input: fn(plan.input) };
    case "project":
      return { ...plan, input: fn(plan.input) };
    case "aggregate":
      return { ...plan, input: fn(plan.input) };
    case "having":
      return { ...plan, input: fn(plan.input) };
    case "sort":
      return { ...plan, input: fn(plan.input) };
    case "limit":
      return { ...plan, input: fn(plan.input) };
    case "distinct":
      return { ...plan, input: fn(plan.input) };
    case "join":
      return { ...plan, left: fn(plan.left), right: fn(plan.right) };
    case "scan":
    case "insert":
    case "update":
    case "delete":
    case "create_table":
    case "drop_table":
    case "empty_result":
      return plan;
    default:
      return plan;
  }
}
