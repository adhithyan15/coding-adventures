/**
 * Join logic — five SQL join types.
 *
 * SQL join types:
 *
 *   INNER       — only rows matching the ON condition on both sides
 *   LEFT OUTER  — all left rows; unmatched left rows get null right columns
 *   RIGHT OUTER — all right rows; unmatched right rows get null left columns
 *   FULL OUTER  — all rows from both sides; nulls for unmatched sides
 *   CROSS       — Cartesian product (no ON condition needed)
 *
 * All joins use nested-loop algorithm O(|left| × |right|).
 */

import type { ASTNode, Token } from "@coding-adventures/parser";
import type { Row } from "./types.js";
import { evalExpr } from "./expression.js";

/**
 * Perform a SQL join between two sets of rows.
 *
 * @param leftRows   - Rows from the left table (already qualified).
 * @param rightRows  - Rows from the right table.
 * @param rightAlias - Table alias for the right side.
 * @param joinType   - "INNER", "LEFT", "RIGHT", "FULL", or "CROSS".
 * @param onCondition - The ON expression node, or null for CROSS JOIN.
 */
export function performJoin(
  leftRows: Row[],
  rightRows: Row[],
  rightAlias: string,
  joinType: string,
  onCondition: ASTNode | Token | null
): Row[] {
  if (joinType === "CROSS" || onCondition === null) {
    return crossJoin(leftRows, rightAlias, rightRows);
  }

  const nullRight = nullRowFor(rightRows, rightAlias);
  const nullLeft = nullRowFor(leftRows, "");

  switch (joinType) {
    case "INNER":
      return innerJoin(leftRows, rightRows, rightAlias, onCondition);
    case "LEFT":
    case "LEFT OUTER":
      return leftJoin(leftRows, rightRows, rightAlias, nullRight, onCondition);
    case "RIGHT":
    case "RIGHT OUTER":
      return rightJoin(leftRows, rightRows, rightAlias, nullLeft, onCondition);
    case "FULL":
    case "FULL OUTER":
      return fullJoin(leftRows, rightRows, rightAlias, nullLeft, nullRight, onCondition);
    default:
      return innerJoin(leftRows, rightRows, rightAlias, onCondition);
  }
}

function innerJoin(
  leftRows: Row[], rightRows: Row[], rightAlias: string,
  condition: ASTNode | Token
): Row[] {
  const result: Row[] = [];
  for (const lrow of leftRows) {
    for (const rrow of rightRows) {
      const merged = mergeRows(lrow, rightAlias, rrow);
      if (testCondition(condition, merged)) result.push(merged);
    }
  }
  return result;
}

function leftJoin(
  leftRows: Row[], rightRows: Row[], rightAlias: string,
  nullRight: Row, condition: ASTNode | Token
): Row[] {
  const result: Row[] = [];
  for (const lrow of leftRows) {
    let matched = false;
    for (const rrow of rightRows) {
      const merged = mergeRows(lrow, rightAlias, rrow);
      if (testCondition(condition, merged)) { result.push(merged); matched = true; }
    }
    if (!matched) result.push(mergeRows(lrow, rightAlias, nullRight));
  }
  return result;
}

function rightJoin(
  leftRows: Row[], rightRows: Row[], rightAlias: string,
  nullLeft: Row, condition: ASTNode | Token
): Row[] {
  const result: Row[] = [];
  for (const rrow of rightRows) {
    let matched = false;
    for (const lrow of leftRows) {
      const merged = mergeRows(lrow, rightAlias, rrow);
      if (testCondition(condition, merged)) { result.push(merged); matched = true; }
    }
    if (!matched) result.push(mergeRows(nullLeft, rightAlias, rrow));
  }
  return result;
}

function fullJoin(
  leftRows: Row[], rightRows: Row[], rightAlias: string,
  nullLeft: Row, nullRight: Row, condition: ASTNode | Token
): Row[] {
  const result: Row[] = [];
  const matchedRight = new Set<number>();

  for (const lrow of leftRows) {
    let leftMatched = false;
    rightRows.forEach((rrow, i) => {
      const merged = mergeRows(lrow, rightAlias, rrow);
      if (testCondition(condition, merged)) {
        result.push(merged);
        leftMatched = true;
        matchedRight.add(i);
      }
    });
    if (!leftMatched) result.push(mergeRows(lrow, rightAlias, nullRight));
  }

  rightRows.forEach((rrow, i) => {
    if (!matchedRight.has(i)) result.push(mergeRows(nullLeft, rightAlias, rrow));
  });

  return result;
}

function crossJoin(leftRows: Row[], rightAlias: string, rightRows: Row[]): Row[] {
  const result: Row[] = [];
  for (const lrow of leftRows) {
    for (const rrow of rightRows) {
      result.push(mergeRows(lrow, rightAlias, rrow));
    }
  }
  return result;
}

/** Merge a left row and a right row. Right keys get qualified names. */
export function mergeRows(left: Row, rightAlias: string, right: Row): Row {
  const merged: Row = { ...left };
  for (const [key, val] of Object.entries(right)) {
    if (key.includes(".")) {
      merged[key] = val;
    } else {
      merged[`${rightAlias}.${key}`] = val;
      merged[key] = val;
    }
  }
  return merged;
}

/** Build a NULL-value row with the same keys as rows[0]. */
function nullRowFor(rows: Row[], _alias: string): Row {
  if (rows.length === 0) return {};
  const nullRow: Row = {};
  for (const key of Object.keys(rows[0])) {
    nullRow[key] = null;
  }
  return nullRow;
}

function testCondition(condition: ASTNode | Token, merged: Row): boolean {
  const val = evalExpr(condition, merged);
  return val === true;
}
