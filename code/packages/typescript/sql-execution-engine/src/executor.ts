/**
 * Executor — the relational pipeline for a SELECT statement.
 *
 * Execution order (logical, not syntactic):
 *
 *   1. FROM        — scan source table
 *   2. JOIN        — combine tables
 *   3. WHERE       — filter rows
 *   4. GROUP BY    — group rows for aggregation
 *   5. HAVING      — filter groups
 *   6. SELECT      — project columns
 *   7. DISTINCT    — deduplicate
 *   8. ORDER BY    — sort
 *   9. LIMIT       — paginate
 */

import type { ASTNode, Token } from "@coding-adventures/parser";
import { isASTNode } from "@coding-adventures/parser";
import type { DataSource } from "./data-source.js";
import type { Row, QueryResult, SqlValue } from "./types.js";
import { evalExpr, nodeText } from "./expression.js";
import { performJoin } from "./join.js";
import { computeAggregates, type AggSpec } from "./aggregate.js";
import { ColumnNotFoundError } from "./errors.js";

/** Execute a `select_stmt` AST node against a data source. */
export function executeSelect(stmt: ASTNode, source: DataSource): QueryResult {
  // --- Phase 1: FROM ---
  const tableRef = findChild(stmt, "table_ref");
  const [tableName, tableAlias] = extractTableRef(tableRef);
  const effectiveAlias = tableAlias || tableName;
  let rows: Row[] = source.scan(tableName);
  rows = qualifyRows(rows, effectiveAlias);

  // --- Phase 2: JOIN ---
  for (const jc of findChildren(stmt, "join_clause")) {
    rows = processJoin(rows, jc, source);
  }

  // --- Phase 3: WHERE ---
  const whereClause = findChild(stmt, "where_clause");
  if (whereClause) {
    rows = applyWhere(rows, whereClause);
  }

  // --- Phase 4 & 5: GROUP BY / HAVING ---
  const groupClause = findChild(stmt, "group_clause");
  const havingClause = findChild(stmt, "having_clause");
  const selectList = findChild(stmt, "select_list");

  const allAggSpecs: AggSpec[] = [];
  if (selectList) collectAggSpecs(selectList, allAggSpecs);
  if (havingClause) collectAggSpecs(havingClause, allAggSpecs);
  // Deduplicate
  const uniqueSpecs: AggSpec[] = allAggSpecs.filter(
    ([f, a], i) => allAggSpecs.findIndex(([ff, aa]) => ff === f && aa === a) === i
  );

  if (groupClause || uniqueSpecs.length > 0) {
    rows = applyGroupByAndAggregate(rows, groupClause, havingClause, uniqueSpecs);
  }

  // --- Phase 6: ORDER BY (before projection so non-selected cols are accessible) ---
  const orderClause = findChild(stmt, "order_clause");
  if (orderClause) {
    rows = applyOrderBy(rows, orderClause);
  }

  // --- Phase 7: SELECT projection ---
  const hasDistinct = hasDistinctQualifier(stmt);
  let { columns, rows: projectedRows } = applySelect(selectList, rows, tableName, effectiveAlias, source);
  rows = projectedRows;

  // --- Phase 8: DISTINCT ---
  if (hasDistinct) {
    rows = applyDistinct(rows);
  }

  // --- Phase 9: LIMIT ---
  const limitClause = findChild(stmt, "limit_clause");
  if (limitClause) {
    rows = applyLimit(rows, limitClause);
  }

  return { columns, rows };
}

// ---------------------------------------------------------------------------
// Phase 1: FROM
// ---------------------------------------------------------------------------

function extractTableRef(tableRef: ASTNode | null): [string, string] {
  if (!tableRef) return ["", ""];

  const tableNameNode = findChild(tableRef, "table_name");
  const tableName = tableNameNode
    ? firstTokenValue(tableNameNode)
    : firstTokenValue(tableRef);

  let alias = "";
  const children = tableRef.children;
  for (let i = 0; i < children.length; i++) {
    if (keywordValueOf(children[i]) === "AS") {
      const next = children[i + 1];
      if (next && !isASTNode(next)) alias = (next as Token).value;
    }
  }

  return [tableName, alias];
}

function qualifyRows(rows: Row[], alias: string): Row[] {
  return rows.map((row) => {
    const qrow: Row = { ...row };
    for (const [key, val] of Object.entries(row)) {
      if (!key.includes(".")) qrow[`${alias}.${key}`] = val;
    }
    return qrow;
  });
}

// ---------------------------------------------------------------------------
// Phase 2: JOIN
// ---------------------------------------------------------------------------

function processJoin(leftRows: Row[], joinClause: ASTNode, source: DataSource): Row[] {
  const joinTypeNode = findChild(joinClause, "join_type");
  const joinType = extractJoinType(joinTypeNode);

  const tableRef = findChild(joinClause, "table_ref");
  const [rightTable, rightAlias] = extractTableRef(tableRef);
  const effectiveAlias = rightAlias || rightTable;

  const rightRows = qualifyRows(source.scan(rightTable), effectiveAlias);
  const onCondition = findOnCondition(joinClause);

  return performJoin(leftRows, rightRows, effectiveAlias, joinType, onCondition);
}

function extractJoinType(node: ASTNode | null): string {
  if (!node) return "INNER";
  const keywords = node.children
    .filter((c): c is Token => !isASTNode(c))
    .map((c) => c.value.toUpperCase());
  return keywords.length > 0 ? keywords.join(" ") : "INNER";
}

function findOnCondition(joinClause: ASTNode): ASTNode | Token | null {
  const children = joinClause.children;
  for (let i = 0; i < children.length; i++) {
    if (keywordValueOf(children[i]) === "ON") {
      return children[i + 1] ?? null;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Phase 3: WHERE
// ---------------------------------------------------------------------------

function applyWhere(rows: Row[], whereClause: ASTNode): Row[] {
  const expr = findExprInClause(whereClause);
  if (!expr) return rows;
  return rows.filter((row) => {
    const val = evalExpr(expr, row);
    return isTruthy(val);
  });
}

// ---------------------------------------------------------------------------
// Phase 4 & 5: GROUP BY / HAVING
// ---------------------------------------------------------------------------

function applyGroupByAndAggregate(
  rows: Row[],
  groupClause: ASTNode | null,
  havingClause: ASTNode | null,
  aggSpecs: AggSpec[]
): Row[] {
  const groupKeys: string[] = groupClause ? extractGroupKeys(groupClause) : [];

  // Partition into groups preserving order
  const groups = new Map<string, Row[]>();
  const groupOrder: string[] = [];

  for (const row of rows) {
    const keyValues = groupKeys.map((k) => row[k] ?? null);
    const keyStr = JSON.stringify(keyValues);
    if (!groups.has(keyStr)) {
      groups.set(keyStr, []);
      groupOrder.push(keyStr);
    }
    groups.get(keyStr)!.push(row);
  }

  const result: Row[] = [];
  for (const keyStr of groupOrder) {
    const groupRows = groups.get(keyStr)!;
    const repRow: Row = { ...groupRows[0] };

    // Restore group key values
    const keyValues: SqlValue[] = JSON.parse(keyStr);
    groupKeys.forEach((k, i) => { repRow[k] = keyValues[i]; });

    // Compute aggregates
    if (aggSpecs.length > 0) {
      Object.assign(repRow, computeAggregates(groupRows, aggSpecs));
    }

    // HAVING filter
    if (havingClause) {
      const havingExpr = findExprInClause(havingClause);
      if (havingExpr && !isTruthy(evalExpr(havingExpr, repRow))) continue;
    }

    result.push(repRow);
  }
  return result;
}

function extractGroupKeys(groupClause: ASTNode): string[] {
  return groupClause.children
    .filter(isASTNode)
    .map((c) => nodeText(c).trim());
}

function collectAggSpecs(node: ASTNode, specs: AggSpec[]): void {
  if (node.ruleName === "function_call") {
    const nameToken = node.children[0];
    if (!isASTNode(nameToken)) {
      const funcName = (nameToken as Token).value.toUpperCase();
      if (["COUNT", "SUM", "AVG", "MIN", "MAX"].includes(funcName)) {
        const argParts = node.children
          .slice(2, node.children.length - 1)
          .filter((c) => !(!isASTNode(c) && (c as Token).value === ","))
          .map((c) => (!isASTNode(c) ? (c as Token).value : nodeText(c as ASTNode)))
          .join("");
        const pair: AggSpec = [funcName, argParts];
        if (!specs.some(([f, a]) => f === pair[0] && a === pair[1])) {
          specs.push(pair);
        }
      }
    }
  }
  for (const child of node.children) {
    if (isASTNode(child)) collectAggSpecs(child, specs);
  }
}

// ---------------------------------------------------------------------------
// Phase 6: SELECT projection
// ---------------------------------------------------------------------------

function applySelect(
  selectList: ASTNode | null,
  rows: Row[],
  primaryTable: string,
  primaryAlias: string,
  source: DataSource
): QueryResult {
  if (!selectList) return { columns: [], rows };

  // Star select
  if (isStarSelect(selectList)) {
    if (rows.length === 0) {
      const cols = source.schema(primaryAlias || primaryTable);
      return { columns: cols, rows: [] };
    }
    const cols = allBareColumns(rows[0]);
    const projected = rows.map((row) =>
      Object.fromEntries(cols.map((c) => [c, row[c] ?? null]))
    );
    return { columns: cols, rows: projected };
  }

  const items = findChildren(selectList, "select_item");
  const itemList = items.length > 0 ? items : [selectList];

  const columns = itemList.map((item) => extractItemName(item)[0]);
  const projectedRows = rows.map((row) => {
    const proj: Row = {};
    for (const item of itemList) {
      const [colName, exprNode] = extractItemName(item);
      let val: SqlValue = null;
      if (exprNode) {
        try {
          val = evalExpr(exprNode, row);
        } catch (e) {
          if (e instanceof ColumnNotFoundError) val = null;
          else throw e;
        }
      }
      proj[colName] = val;
    }
    return proj;
  });

  return { columns, rows: projectedRows };
}

function isStarSelect(sl: ASTNode): boolean {
  return sl.children.some((c) => {
    if (!isASTNode(c)) return (c as Token).value === "*";
    return (c as ASTNode).children.some(
      (gc) => !isASTNode(gc) && (gc as Token).value === "*"
    );
  });
}

function extractItemName(item: ASTNode): [string, ASTNode | Token | null] {
  let alias: string | null = null;
  let exprNode: ASTNode | Token | null = null;

  const children = item.children;
  for (let i = 0; i < children.length; i++) {
    const child = children[i];
    if (keywordValueOf(child) === "AS") {
      const next = children[i + 1];
      if (next && !isASTNode(next)) alias = (next as Token).value;
      i++;
      continue;
    }
    if (keywordValueOf(child) !== "AS" && exprNode === null) {
      exprNode = child;
    }
  }

  const name = alias ?? (exprNode ? inferColName(exprNode) : "?");
  return [name, exprNode];
}

function inferColName(node: ASTNode | Token): string {
  if (!isASTNode(node)) return (node as Token).value;
  const ast = node as ASTNode;
  if (ast.ruleName === "column_ref") {
    const tokens = ast.children.filter((c): c is Token => !isASTNode(c));
    return tokens.length > 0 ? tokens[tokens.length - 1].value : "?";
  }
  return nodeText(ast);
}

function allBareColumns(row: Row): string[] {
  return Object.keys(row).filter((k) => !k.includes(".") && !k.startsWith("_"));
}

// ---------------------------------------------------------------------------
// Phase 7: DISTINCT
// ---------------------------------------------------------------------------

function applyDistinct(rows: Row[]): Row[] {
  const seen: string[] = [];
  const result: Row[] = [];
  for (const row of rows) {
    const key = JSON.stringify(Object.values(row));
    if (!seen.includes(key)) {
      seen.push(key);
      result.push(row);
    }
  }
  return result;
}

// ---------------------------------------------------------------------------
// Phase 8: ORDER BY
// ---------------------------------------------------------------------------

function applyOrderBy(rows: Row[], orderClause: ASTNode): Row[] {
  const items = findChildren(orderClause, "order_item");
  if (items.length === 0) return rows;

  const result = [...rows];
  // Apply from last to first (stable sort)
  for (let i = items.length - 1; i >= 0; i--) {
    const [exprNode, ascending] = extractOrderItem(items[i]);
    result.sort((a, b) => {
      let va: SqlValue = null;
      let vb: SqlValue = null;
      try { va = evalExpr(exprNode, a); } catch (_) { va = null; }
      try { vb = evalExpr(exprNode, b); } catch (_) { vb = null; }
      const cmp = compareForSort(va, vb);
      return ascending ? cmp : -cmp;
    });
  }
  return result;
}

function extractOrderItem(item: ASTNode): [ASTNode | Token, boolean] {
  let ascending = true;
  const exprNode = item.children[0];
  for (const child of item.children.slice(1)) {
    const kw = keywordValueOf(child);
    if (kw === "DESC") ascending = false;
    if (kw === "ASC") ascending = true;
  }
  return [exprNode, ascending];
}

function compareForSort(a: SqlValue, b: SqlValue): number {
  if (a === null && b === null) return 0;
  if (a === null) return 1; // NULL sorts last
  if (b === null) return -1;
  if (typeof a === "number" && typeof b === "number") return a < b ? -1 : a > b ? 1 : 0;
  if (typeof a === "string" && typeof b === "string") {
    return a.toLowerCase() < b.toLowerCase() ? -1 : a.toLowerCase() > b.toLowerCase() ? 1 : 0;
  }
  return 0;
}

// ---------------------------------------------------------------------------
// Phase 9: LIMIT
// ---------------------------------------------------------------------------

function applyLimit(rows: Row[], limitClause: ASTNode): Row[] {
  let limit: number | null = null;
  let offset = 0;

  const children = limitClause.children;
  for (let i = 0; i < children.length; i++) {
    const kw = keywordValueOf(children[i]);
    if (kw === "LIMIT") {
      i++;
      if (i < children.length) limit = parseInt(tokenValueOf(children[i]), 10);
    } else if (kw === "OFFSET") {
      i++;
      if (i < children.length) offset = parseInt(tokenValueOf(children[i]), 10);
    }
  }

  const sliced = rows.slice(offset);
  return limit !== null ? sliced.slice(0, limit) : sliced;
}

// ---------------------------------------------------------------------------
// DISTINCT qualifier
// ---------------------------------------------------------------------------

function hasDistinctQualifier(stmt: ASTNode): boolean {
  return stmt.children.some((c) => keywordValueOf(c) === "DISTINCT");
}

// ---------------------------------------------------------------------------
// Generic AST helpers
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
    (c): c is ASTNode => isASTNode(c) && (c as ASTNode).ruleName === ruleName
  );
}

function findExprInClause(clause: ASTNode): ASTNode | Token | null {
  for (const child of clause.children) {
    const kw = keywordValueOf(child);
    if (["WHERE", "HAVING", "ON"].includes(kw)) continue;
    return child;
  }
  return null;
}

function firstTokenValue(node: ASTNode): string {
  for (const child of node.children) {
    if (!isASTNode(child)) return (child as Token).value;
  }
  return "";
}

function keywordValueOf(child: ASTNode | Token): string {
  if (!isASTNode(child)) return (child as Token).value.toUpperCase();
  const first = (child as ASTNode).children[0];
  if (first && !isASTNode(first)) return (first as Token).value.toUpperCase();
  return "";
}

function tokenValueOf(child: ASTNode | Token): string {
  if (!isASTNode(child)) return (child as Token).value;
  const first = (child as ASTNode).children[0];
  if (first && !isASTNode(first)) return (first as Token).value;
  return "";
}

function isTruthy(val: SqlValue): boolean {
  if (val === null) return false;
  if (typeof val === "boolean") return val;
  if (typeof val === "number") return val !== 0;
  if (typeof val === "string") return val.length > 0;
  return false;
}
