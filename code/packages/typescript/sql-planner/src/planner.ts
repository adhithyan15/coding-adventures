/**
 * SQL query planner — converts a raw AST from sql-parser into a LogicalPlan.
 *
 * The planner performs one-to-one structural translation: it does not optimise.
 * Given a program ASTNode (the root returned by parseSQL), it walks each
 * statement and returns a list of LogicalPlan nodes.
 *
 * SELECT planning order (matches SQL evaluation semantics):
 *   1. FROM  → ScanNode (or JoinNode)
 *   2. WHERE → FilterNode
 *   3. GROUP BY + aggregates → AggregateNode
 *   4. HAVING → HavingNode
 *   5. ORDER BY (includes hidden __sort_ cols) → SortNode
 *   6. SELECT list → ProjectNode
 *   7. DISTINCT → DistinctNode
 *   8. LIMIT → LimitNode
 *
 * Non-projected ORDER BY columns are handled by adding a hidden projection
 * item with alias "__sort_<col>" so that SortNode can reference it; the VM
 * strips "__sort_" columns from the final result.
 */

import type { ASTNode, Token } from "@coding-adventures/parser";
import { isASTNode } from "@coding-adventures/parser";
import type {
  AggregateSpec,
  ColumnDefinition,
  Expr,
  LogicalPlan,
  ProjectItem,
  SortKey,
  SqlValue,
} from "./types.js";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Translate the `program` ASTNode into a list of LogicalPlan nodes. */
export function planAll(programNode: ASTNode): LogicalPlan[] {
  const plans: LogicalPlan[] = [];
  for (const child of programNode.children) {
    if (!isASTNode(child)) continue;
    const n = child as ASTNode;
    if (n.ruleName === "statement") {
      plans.push(planStatement(n));
    } else if (
      n.ruleName === "select_stmt" ||
      n.ruleName === "insert_stmt" ||
      n.ruleName === "update_stmt" ||
      n.ruleName === "delete_stmt" ||
      n.ruleName === "create_table_stmt" ||
      n.ruleName === "drop_table_stmt"
    ) {
      plans.push(planStatement(n));
    }
  }
  return plans;
}

/** Translate a single statement ASTNode. */
export function plan(stmtNode: ASTNode): LogicalPlan {
  // If the node is a "program" or "statement" wrapper, peel it.
  if (stmtNode.ruleName === "program") {
    const stmts = planAll(stmtNode);
    if (stmts.length === 0) throw new PlanError("empty SQL program");
    return stmts[0];
  }
  if (stmtNode.ruleName === "statement") {
    return planStatement(stmtNode);
  }
  return planStatement(stmtNode);
}

export class PlanError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PlanError";
  }
}

// ---------------------------------------------------------------------------
// Statement dispatch
// ---------------------------------------------------------------------------

function planStatement(node: ASTNode): LogicalPlan {
  // Unwrap "statement" → actual statement
  if (node.ruleName === "statement") {
    const inner = node.children[0];
    if (!isASTNode(inner)) throw new PlanError(`unexpected token in statement: ${(inner as Token).value}`);
    return planStatement(inner as ASTNode);
  }
  switch (node.ruleName) {
    case "select_stmt": return planSelect(node);
    case "insert_stmt": return planInsert(node);
    case "update_stmt": return planUpdate(node);
    case "delete_stmt": return planDelete(node);
    case "create_table_stmt": return planCreateTable(node);
    case "drop_table_stmt": return planDropTable(node);
    default:
      throw new PlanError(`unsupported statement type: ${node.ruleName}`);
  }
}

// ---------------------------------------------------------------------------
// SELECT
// ---------------------------------------------------------------------------

function planSelect(node: ASTNode): LogicalPlan {
  // --- FROM clause ---
  // Grammar: SELECT [DISTINCT] select_list [FROM table_ref join*] [WHERE] ...
  // After the grammar change, FROM is optional; children are mixed keywords + rule nodes.
  const fromRef = findChild(node, "table_ref");
  const joinClauses = findChildren(node, "join_clause");
  const whereClause = findChild(node, "where_clause");
  const groupClause = findChild(node, "group_clause");
  const havingClause = findChild(node, "having_clause");
  const orderClause = findChild(node, "order_clause");
  const limitClause = findChild(node, "limit_clause");
  const selectList = findChild(node, "select_list");
  const hasDistinct = hasKeywordChild(node, "DISTINCT");

  // Build scan (or dual if no FROM).
  let plan: LogicalPlan;
  if (fromRef) {
    plan = planTableRef(fromRef);
    for (const jc of joinClauses) {
      plan = planJoin(plan, jc);
    }
  } else {
    // "SELECT expr" with no FROM — virtual single-row table __dual__.
    plan = { type: "scan", table: "__dual__", alias: "__dual__" };
  }

  // WHERE
  if (whereClause) {
    const pred = planExprFromClause(whereClause, 1); // skip "WHERE" keyword
    plan = { type: "filter", input: plan, predicate: pred };
  }

  // Collect aggregates from SELECT list and HAVING.
  const selectItems = selectList ? parseSelectItems(selectList) : [];
  const havingExpr = havingClause ? planExprFromClause(havingClause, 1) : null;

  const aggSpecs: AggregateSpec[] = [];
  for (const item of selectItems) {
    collectAggregates(item.expr, aggSpecs, "sel");
  }
  if (havingExpr) {
    collectAggregates(havingExpr, aggSpecs, "hav");
  }

  // GROUP BY keys.
  const groupKeys: Expr[] = [];
  if (groupClause) {
    for (const child of groupClause.children) {
      if (!isASTNode(child)) continue;
      const n = child as ASTNode;
      if (n.ruleName === "column_ref" || n.ruleName === "expr" || n.ruleName === "or_expr") {
        groupKeys.push(planExpr(n));
      }
    }
  }

  const needsAggregate = groupKeys.length > 0 || aggSpecs.length > 0;
  if (needsAggregate) {
    plan = { type: "aggregate", input: plan, keys: groupKeys, aggregates: aggSpecs };
    if (havingExpr) {
      plan = { type: "having", input: plan, predicate: havingExpr };
    }
  }

  // ORDER BY — add __sort_ sentinel columns for non-projected expressions.
  const sortKeys: SortKey[] = [];
  const extraProjectItems: ProjectItem[] = [];
  if (orderClause) {
    const orderItems = findChildren(orderClause, "order_item");
    for (const oi of orderItems) {
      const { expr, ascending } = parseOrderItem(oi);
      sortKeys.push({ expr, ascending, nullsLast: ascending });

      // If this expression is not already in the SELECT list, add a hidden column.
      const colText = exprKey(expr);
      const alreadyProjected = selectItems.some((si) => {
        if (si.alias && si.alias === colText) return true;
        return exprKey(si.expr) === colText;
      });
      if (!alreadyProjected && expr.kind !== "literal") {
        extraProjectItems.push({ expr, alias: `__sort_${colText}` });
      }
    }
  }

  // PROJECT: build the wide projection including hidden sort columns.
  const wideItems: ProjectItem[] = [...selectItems, ...extraProjectItems];
  plan = buildProject(plan, wideItems, fromRef);

  // Sort wraps the projection.
  if (sortKeys.length > 0) {
    plan = { type: "sort", input: plan, keys: sortKeys };
  }

  if (hasDistinct) {
    plan = { type: "distinct", input: plan };
  }

  if (limitClause) {
    const { count, offset } = parseLimitClause(limitClause);
    plan = { type: "limit", input: plan, count, offset };
  }

  return plan;
}

// ---------------------------------------------------------------------------
// INSERT
// ---------------------------------------------------------------------------

function planInsert(node: ASTNode): LogicalPlan {
  // Grammar: INSERT INTO NAME [(cols)] VALUES row_value {, row_value}
  const children = node.children;
  let i = 0;
  while (i < children.length && tokenVal(children[i]).toUpperCase() !== "INTO") i++;
  i++; // skip INTO
  const tableName = tokenVal(children[i++]);

  // Optional column list: "(" NAME {"," NAME} ")"
  let columns: string[] | null = null;
  if (i < children.length && tokenVal(children[i]) === "(") {
    i++; // skip "("
    columns = [];
    while (i < children.length && tokenVal(children[i]) !== ")") {
      const v = tokenVal(children[i]);
      if (v !== ",") columns.push(v);
      i++;
    }
    i++; // skip ")"
  }

  // Skip VALUES keyword
  while (i < children.length && tokenVal(children[i]).toUpperCase() !== "VALUES") i++;
  i++;

  // Parse row_value nodes.
  const rows: Expr[][] = [];
  for (let j = i; j < children.length; j++) {
    const child = children[j];
    if (!isASTNode(child)) continue;
    const n = child as ASTNode;
    if (n.ruleName === "row_value") {
      rows.push(parseRowValue(n));
    }
  }

  return { type: "insert", table: tableName, columns, rows };
}

// ---------------------------------------------------------------------------
// UPDATE
// ---------------------------------------------------------------------------

function planUpdate(node: ASTNode): LogicalPlan {
  // Grammar: UPDATE NAME SET assignment {, assignment} [where_clause]
  const children = node.children;
  let i = 0;
  while (i < children.length && tokenVal(children[i]).toUpperCase() !== "UPDATE") i++;
  i++; // skip UPDATE
  const tableName = tokenVal(children[i++]);
  // skip SET
  while (i < children.length && tokenVal(children[i]).toUpperCase() !== "SET") i++;
  i++;

  const assignments: { column: string; value: Expr }[] = [];
  let whereClause: ASTNode | null = null;
  while (i < children.length) {
    const child = children[i];
    if (isASTNode(child)) {
      const n = child as ASTNode;
      if (n.ruleName === "assignment") {
        assignments.push(parseAssignment(n));
      } else if (n.ruleName === "where_clause") {
        whereClause = n;
      }
    }
    i++;
  }

  const predicate = whereClause ? planExprFromClause(whereClause, 1) : null;
  return { type: "update", table: tableName, assignments, predicate };
}

// ---------------------------------------------------------------------------
// DELETE
// ---------------------------------------------------------------------------

function planDelete(node: ASTNode): LogicalPlan {
  // Grammar: DELETE FROM NAME [where_clause]
  const children = node.children;
  let tableName = "";
  let whereClause: ASTNode | null = null;
  for (const child of children) {
    if (!isASTNode(child)) {
      const kw = tokenVal(child).toUpperCase();
      if (kw !== "DELETE" && kw !== "FROM") {
        tableName = tokenVal(child);
      }
    } else {
      const n = child as ASTNode;
      if (n.ruleName === "where_clause") whereClause = n;
    }
  }
  const predicate = whereClause ? planExprFromClause(whereClause, 1) : null;
  return { type: "delete", table: tableName, predicate };
}

// ---------------------------------------------------------------------------
// CREATE TABLE
// ---------------------------------------------------------------------------

function planCreateTable(node: ASTNode): LogicalPlan {
  // Grammar: CREATE TABLE [IF NOT EXISTS] NAME "(" col_def {, col_def} ")"
  let ifNotExists = false;
  let tableName = "";
  const columns: ColumnDefinition[] = [];
  let seenTable = false;

  for (const child of node.children) {
    if (!isASTNode(child)) {
      const v = tokenVal(child).toUpperCase();
      if (v === "CREATE" || v === "TABLE" || v === "(" || v === ")" || v === "," || v === "IF" || v === "NOT" || v === "EXISTS") {
        if (v === "EXISTS") ifNotExists = true;
        continue;
      }
      if (!seenTable) {
        tableName = tokenVal(child);
        seenTable = true;
      }
    } else {
      const n = child as ASTNode;
      if (n.ruleName === "col_def") {
        columns.push(parseColDef(n));
      }
    }
  }

  return { type: "create_table", table: tableName, columns, ifNotExists };
}

// ---------------------------------------------------------------------------
// DROP TABLE
// ---------------------------------------------------------------------------

function planDropTable(node: ASTNode): LogicalPlan {
  let ifExists = false;
  let tableName = "";
  for (const child of node.children) {
    if (!isASTNode(child)) {
      const v = tokenVal(child).toUpperCase();
      if (v === "DROP" || v === "TABLE" || v === "IF" || v === "EXISTS") {
        if (v === "EXISTS") ifExists = true;
        continue;
      }
      tableName = tokenVal(child);
    }
  }
  return { type: "drop_table", table: tableName, ifExists };
}

// ---------------------------------------------------------------------------
// JOIN
// ---------------------------------------------------------------------------

function planJoin(left: LogicalPlan, joinClause: ASTNode): LogicalPlan {
  // Grammar: join_type JOIN table_ref ON expr
  const joinTypeNode = findChild(joinClause, "join_type");
  const tableRef = findChild(joinClause, "table_ref");
  const onExprNode = findChildAfterKeyword(joinClause, "ON");

  let joinType: "inner" | "left" | "right" | "full" | "cross" = "inner";
  if (joinTypeNode) {
    const jt = nodeText(joinTypeNode).toUpperCase();
    if (jt.includes("LEFT")) joinType = "left";
    else if (jt.includes("RIGHT")) joinType = "right";
    else if (jt.includes("FULL")) joinType = "full";
    else if (jt.includes("CROSS")) joinType = "cross";
  }

  const right: LogicalPlan = tableRef ? planTableRef(tableRef) : { type: "scan", table: "", alias: "" };
  const condition = onExprNode ? planExpr(onExprNode) : null;

  return { type: "join", left, right, joinType, condition };
}

// ---------------------------------------------------------------------------
// Expression parsing
// ---------------------------------------------------------------------------

/** Convert an AST expression node or Token to a planner Expr. */
export function planExpr(node: ASTNode | Token): Expr {
  if (!isASTNode(node)) {
    return planToken(node as Token);
  }
  return planExprNode(node as ASTNode);
}

function planToken(token: Token): Expr {
  const v = token.value;
  const t = token.type;
  switch (t) {
    case "NUMBER":
      return { kind: "literal", value: v.includes(".") ? parseFloat(v) : parseInt(v, 10) };
    case "STRING":
      return { kind: "literal", value: v };
    case "KEYWORD": {
      const kw = v.toUpperCase();
      if (kw === "NULL") return { kind: "literal", value: null };
      if (kw === "TRUE") return { kind: "literal", value: true };
      if (kw === "FALSE") return { kind: "literal", value: false };
      return { kind: "column", table: null, name: v };
    }
    case "NAME":
      return { kind: "column", table: null, name: v };
    default:
      return { kind: "literal", value: v };
  }
}

function planExprNode(node: ASTNode): Expr {
  const rule = node.ruleName;
  switch (rule) {
    case "expr":
      return planExpr(node.children[0]);

    case "or_expr": {
      // and_expr { "OR" and_expr }
      const parts = skipKeywords(node.children, ["OR"]);
      if (parts.length === 1) return planExpr(parts[0]);
      return parts.slice(1).reduce(
        (acc, cur) => ({ kind: "binary", op: "OR", left: acc, right: planExpr(cur) } as Expr),
        planExpr(parts[0]) as Expr,
      );
    }

    case "and_expr": {
      // not_expr { "AND" not_expr }
      const parts = skipKeywords(node.children, ["AND"]);
      if (parts.length === 1) return planExpr(parts[0]);
      return parts.slice(1).reduce(
        (acc, cur) => ({ kind: "binary", op: "AND", left: acc, right: planExpr(cur) } as Expr),
        planExpr(parts[0]) as Expr,
      );
    }

    case "not_expr": {
      if (isKeyword(node.children[0], "NOT")) {
        return { kind: "unary", op: "NOT", operand: planExpr(node.children[1]) };
      }
      return planExpr(node.children[0]);
    }

    case "comparison": {
      return planComparison(node);
    }

    case "additive": {
      // multiplicative { ("+"|"-"|"||") multiplicative }
      let result = planExpr(node.children[0]);
      let i = 1;
      while (i < node.children.length) {
        const opRaw = tokenVal(node.children[i]);
        i++;
        const right = planExpr(node.children[i]);
        i++;
        result = { kind: "binary", op: opRaw, left: result, right };
      }
      return result;
    }

    case "multiplicative": {
      // unary { ("*"|"/"|"%") unary }
      let result = planExpr(node.children[0]);
      let i = 1;
      while (i < node.children.length) {
        const opRaw = tokenVal(node.children[i]);
        i++;
        const right = planExpr(node.children[i]);
        i++;
        result = { kind: "binary", op: opRaw, left: result, right };
      }
      return result;
    }

    case "unary": {
      if (tokenVal(node.children[0]) === "-") {
        return { kind: "unary", op: "-", operand: planExpr(node.children[1]) };
      }
      return planExpr(node.children[0]);
    }

    case "primary": {
      return planPrimary(node);
    }

    case "column_ref": {
      const ch = node.children;
      if (ch.length === 1) {
        return { kind: "column", table: null, name: tokenVal(ch[0]) };
      }
      // NAME "." NAME
      return { kind: "column", table: tokenVal(ch[0]), name: tokenVal(ch[2]) };
    }

    case "function_call": {
      return planFunctionCall(node);
    }

    case "value_list": {
      // Used as single-element expression pass-through
      return planExpr(node.children[0]);
    }

    default:
      // Pass-through for intermediate wrapper nodes.
      if (node.children.length > 0) return planExpr(node.children[0]);
      return { kind: "literal", value: null };
  }
}

function planComparison(node: ASTNode): Expr {
  const children = node.children;
  const left = planExpr(children[0]);
  if (children.length === 1) return left;

  const secondKw = tokenVal(children[1]).toUpperCase();

  // IS NULL / IS NOT NULL
  if (secondKw === "IS") {
    const thirdKw = tokenVal(children[2]).toUpperCase();
    const negated = thirdKw === "NOT";
    return { kind: "is_null", expr: left, negated };
  }

  // BETWEEN low AND high
  if (secondKw === "BETWEEN") {
    const low = planExpr(children[2]);
    const high = planExpr(children[4]); // children[3] is "AND"
    return { kind: "between", expr: left, low, high, negated: false };
  }

  // NOT BETWEEN / NOT IN / NOT LIKE
  if (secondKw === "NOT") {
    const thirdKw = tokenVal(children[2]).toUpperCase();
    if (thirdKw === "BETWEEN") {
      const low = planExpr(children[3]);
      const high = planExpr(children[5]);
      return { kind: "between", expr: left, low, high, negated: true };
    }
    if (thirdKw === "IN") {
      // NOT IN "(" value_list ")"
      const list = parseValueList(children[4]);
      return { kind: "in_list", expr: left, list, negated: true };
    }
    if (thirdKw === "LIKE") {
      const pattern = planExpr(children[3]);
      return { kind: "like", expr: left, pattern, negated: true };
    }
  }

  // IN "(" value_list ")"
  if (secondKw === "IN") {
    const list = parseValueList(children[3]);
    return { kind: "in_list", expr: left, list, negated: false };
  }

  // LIKE pattern
  if (secondKw === "LIKE") {
    const pattern = planExpr(children[2]);
    return { kind: "like", expr: left, pattern, negated: false };
  }

  // Standard binary comparison: cmp_op additive
  const op = nodeText(children[1]);
  const right = planExpr(children[2]);
  return { kind: "binary", op, left, right };
}

function planPrimary(node: ASTNode): Expr {
  const children = node.children;
  if (children.length === 0) return { kind: "literal", value: null };
  const first = children[0];
  if (!isASTNode(first)) {
    const tok = first as Token;
    if (tok.value === "(") {
      return planExpr(children[1]);
    }
    return planToken(tok);
  }
  return planExprNode(first as ASTNode);
}

function planFunctionCall(node: ASTNode): Expr {
  // Grammar: NAME "(" (STAR | value_list?) ")"
  const funcName = tokenVal(node.children[0]).toUpperCase();

  // Find the argument part: skip NAME, "(", ... ")"
  const argsNode = node.children[2];

  // COUNT(*) or func(*)
  if (!isASTNode(argsNode) && tokenVal(argsNode) === "*") {
    const isAgg = isAggregateFunc(funcName);
    if (isAgg) {
      return { kind: "aggregate", func: funcName.toLowerCase(), arg: null, distinct: false };
    }
    return { kind: "func", name: funcName.toLowerCase(), args: [{ kind: "star" }] };
  }

  // No args: func()
  if (!isASTNode(argsNode) && tokenVal(argsNode) === ")") {
    if (funcName === "COALESCE") return { kind: "coalesce", args: [] };
    return { kind: "func", name: funcName.toLowerCase(), args: [] };
  }

  // value_list or single expr
  const args: Expr[] = [];
  if (isASTNode(argsNode)) {
    const vl = argsNode as ASTNode;
    if (vl.ruleName === "value_list") {
      args.push(...parseValueList(vl));
    } else {
      args.push(planExpr(vl));
    }
  }

  if (funcName === "COALESCE") return { kind: "coalesce", args };

  if (isAggregateFunc(funcName)) {
    return {
      kind: "aggregate",
      func: funcName.toLowerCase(),
      arg: args[0] ?? null,
      distinct: false,
    };
  }

  return { kind: "func", name: funcName.toLowerCase(), args };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function planTableRef(node: ASTNode): LogicalPlan {
  // Grammar: table_name [AS NAME]
  const tableNameNode = findChild(node, "table_name");
  const tableName = tableNameNode ? firstTokenVal(tableNameNode) : firstTokenVal(node);
  let alias = tableName;
  // Look for AS alias: children are [table_name, "AS", NAME]
  let seenAs = false;
  for (const child of node.children) {
    if (!isASTNode(child)) {
      const v = tokenVal(child).toUpperCase();
      if (v === "AS") { seenAs = true; continue; }
      if (seenAs) { alias = tokenVal(child); break; }
    }
  }
  return { type: "scan", table: tableName, alias };
}

function buildProject(input: LogicalPlan, items: ProjectItem[], fromRef: ASTNode | null): LogicalPlan {
  // If SELECT *, expand based on nothing (the VM handles *).
  if (items.length === 1 && items[0].expr.kind === "star") {
    return { type: "project", input, items };
  }
  return { type: "project", input, items };
}

function parseSelectItems(selectList: ASTNode): ProjectItem[] {
  // select_list = STAR | select_item {, select_item}
  const items: ProjectItem[] = [];
  for (const child of selectList.children) {
    if (!isASTNode(child)) {
      const v = tokenVal(child);
      if (v === "*") {
        items.push({ expr: { kind: "star" }, alias: null });
        return items;
      }
      continue;
    }
    const n = child as ASTNode;
    if (n.ruleName === "select_item") {
      items.push(parseSelectItem(n));
    }
  }
  return items;
}

function parseSelectItem(node: ASTNode): ProjectItem {
  // select_item = expr [AS NAME]
  let expr: Expr = { kind: "literal", value: null };
  let alias: string | null = null;
  let seenAs = false;
  for (const child of node.children) {
    if (!isASTNode(child)) {
      const v = tokenVal(child).toUpperCase();
      if (v === "AS") { seenAs = true; continue; }
      if (seenAs) { alias = tokenVal(child); break; }
    } else {
      const n = child as ASTNode;
      if (!seenAs) expr = planExpr(n);
    }
  }
  return { expr, alias };
}

function parseOrderItem(node: ASTNode): { expr: Expr; ascending: boolean } {
  // order_item = expr [ASC | DESC]
  let expr: Expr = { kind: "literal", value: null };
  let ascending = true;
  for (const child of node.children) {
    if (!isASTNode(child)) {
      const kw = tokenVal(child).toUpperCase();
      if (kw === "ASC") ascending = true;
      else if (kw === "DESC") ascending = false;
    } else {
      expr = planExpr(child as ASTNode);
    }
  }
  return { expr, ascending };
}

function parseLimitClause(node: ASTNode): { count: number; offset: number } {
  let count = -1;
  let offset = 0;
  const children = node.children;
  for (let i = 0; i < children.length; i++) {
    const kw = tokenVal(children[i]).toUpperCase();
    if (kw === "LIMIT") {
      i++;
      count = parseInt(tokenVal(children[i]), 10);
    } else if (kw === "OFFSET") {
      i++;
      offset = parseInt(tokenVal(children[i]), 10);
    }
  }
  return { count, offset };
}

function parseRowValue(node: ASTNode): Expr[] {
  // row_value = "(" expr {, expr} ")"
  const values: Expr[] = [];
  for (const child of node.children) {
    if (!isASTNode(child)) continue;
    values.push(planExpr(child as ASTNode));
  }
  return values;
}

function parseAssignment(node: ASTNode): { column: string; value: Expr } {
  // assignment = NAME "=" expr
  const col = tokenVal(node.children[0]);
  // children[1] is "=", children[2] is expr
  const expr = planExpr(node.children[2]);
  return { column: col, value: expr };
}

function parseColDef(node: ASTNode): ColumnDefinition {
  // col_def = NAME NAME {col_constraint}
  const children = node.children;
  const name = isASTNode(children[0]) ? firstTokenVal(children[0] as ASTNode) : tokenVal(children[0]);
  const dataType = isASTNode(children[1]) ? firstTokenVal(children[1] as ASTNode) : tokenVal(children[1]);
  let notNull = false;
  let primaryKey = false;
  let unique = false;
  let defaultValue: Expr | null = null;

  for (let i = 2; i < children.length; i++) {
    const child = children[i];
    if (!isASTNode(child)) continue;
    const n = child as ASTNode;
    if (n.ruleName === "col_constraint") {
      const text = nodeText(n).toUpperCase();
      if (text.includes("NOT NULL")) notNull = true;
      else if (text.includes("PRIMARY KEY")) primaryKey = true;
      else if (text.includes("UNIQUE")) unique = true;
      else if (text.includes("DEFAULT")) {
        // Find the primary node child for the default value.
        const primaryNode = findChild(n, "primary");
        if (primaryNode) defaultValue = planExpr(primaryNode);
      }
    }
  }

  return { name, dataType, notNull, primaryKey, unique, defaultValue };
}

function parseValueList(node: ASTNode | (readonly (ASTNode | Token)[])): Expr[] {
  if (Array.isArray(node) || (node && typeof (node as ASTNode).ruleName === "undefined")) {
    // It's already children array
    const exprs: Expr[] = [];
    const arr = node as readonly (ASTNode | Token)[];
    for (const child of arr) {
      if (!isASTNode(child) && tokenVal(child) === ",") continue;
      exprs.push(planExpr(child));
    }
    return exprs;
  }
  const n = node as ASTNode;
  if (n.ruleName === "value_list") {
    const exprs: Expr[] = [];
    for (const child of n.children) {
      if (!isASTNode(child) && tokenVal(child) === ",") continue;
      exprs.push(planExpr(child));
    }
    return exprs;
  }
  return [planExpr(n)];
}

function collectAggregates(expr: Expr, specs: AggregateSpec[], prefix: string): void {
  if (expr.kind === "aggregate") {
    const alias = `__agg_${prefix}_${expr.func}_${specs.length}`;
    // Avoid exact duplicates within the same list.
    const dup = specs.find(
      (s) => s.func === expr.func && exprKey(s.arg) === exprKey(expr.arg) && s.distinct === expr.distinct,
    );
    if (!dup) {
      specs.push({ func: expr.func, arg: expr.arg, distinct: expr.distinct, alias });
    }
  } else if (expr.kind === "binary") {
    collectAggregates(expr.left, specs, prefix);
    collectAggregates(expr.right, specs, prefix);
  } else if (expr.kind === "unary") {
    collectAggregates(expr.operand, specs, prefix);
  } else if (expr.kind === "func" || expr.kind === "coalesce") {
    const args = expr.kind === "func" ? expr.args : expr.args;
    for (const arg of args) collectAggregates(arg, specs, prefix);
  } else if (expr.kind === "between") {
    collectAggregates(expr.expr, specs, prefix);
    collectAggregates(expr.low, specs, prefix);
    collectAggregates(expr.high, specs, prefix);
  } else if (expr.kind === "is_null" || expr.kind === "like") {
    collectAggregates(expr.expr, specs, prefix);
  } else if (expr.kind === "in_list") {
    collectAggregates(expr.expr, specs, prefix);
    for (const item of expr.list) collectAggregates(item, specs, prefix);
  }
}

/** Canonical string key for an expression, used for deduplication. */
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
    case "between": return `between(${exprKey(expr.expr)},${exprKey(expr.low)},${exprKey(expr.high)})`;
    case "in_list": return `in(${exprKey(expr.expr)},[${expr.list.map(exprKey).join(",")}])`;
    case "like": return `like(${exprKey(expr.expr)},${exprKey(expr.pattern)})`;
    case "is_null": return `isnull(${exprKey(expr.expr)},${expr.negated})`;
    case "coalesce": return `coalesce(${expr.args.map(exprKey).join(",")})`;
  }
}

function planExprFromClause(clause: ASTNode, skipCount: number): Expr {
  // Skip skipCount keyword tokens at the front of the clause.
  let skipped = 0;
  for (const child of clause.children) {
    if (!isASTNode(child)) {
      if (skipped < skipCount) { skipped++; continue; }
    }
    if (skipped >= skipCount) {
      return planExpr(child);
    }
    skipped++;
  }
  return { kind: "literal", value: null };
}

function isAggregateFunc(name: string): boolean {
  return ["COUNT", "SUM", "AVG", "MIN", "MAX"].includes(name.toUpperCase());
}

// ---------------------------------------------------------------------------
// Generic AST traversal helpers
// ---------------------------------------------------------------------------

function findChild(node: ASTNode, ruleName: string): ASTNode | null {
  for (const child of node.children) {
    if (isASTNode(child) && (child as ASTNode).ruleName === ruleName) {
      return child as ASTNode;
    }
  }
  return null;
}

function findChildren(node: ASTNode, ruleName: string): ASTNode[] {
  return node.children.filter(
    (c): c is ASTNode => isASTNode(c) && (c as ASTNode).ruleName === ruleName,
  );
}

function findChildAfterKeyword(node: ASTNode, keyword: string): ASTNode | Token | null {
  let found = false;
  for (const child of node.children) {
    if (found) return child;
    if (!isASTNode(child) && tokenVal(child).toUpperCase() === keyword) found = true;
  }
  return null;
}

function firstTokenVal(node: ASTNode): string {
  for (const child of node.children) {
    if (!isASTNode(child)) return (child as Token).value;
    const inner = firstTokenVal(child as ASTNode);
    if (inner) return inner;
  }
  return "";
}

function tokenVal(child: ASTNode | Token): string {
  if (!isASTNode(child)) return (child as Token).value;
  return firstTokenVal(child as ASTNode);
}

function nodeText(node: ASTNode | Token): string {
  if (!isASTNode(node)) return (node as Token).value;
  return (node as ASTNode).children.map(nodeText).join(" ");
}

function isKeyword(child: ASTNode | Token, kw: string): boolean {
  if (isASTNode(child)) return false;
  return (child as Token).value.toUpperCase() === kw.toUpperCase();
}

function hasKeywordChild(node: ASTNode, kw: string): boolean {
  return node.children.some((c) => !isASTNode(c) && (c as Token).value.toUpperCase() === kw);
}

function skipKeywords(children: ReadonlyArray<ASTNode | Token>, keywords: string[]): ReadonlyArray<ASTNode | Token> {
  return children.filter((c) => {
    if (isASTNode(c)) return true;
    return !keywords.includes((c as Token).value.toUpperCase());
  });
}
