/**
 * Expression evaluator — evaluate AST expression nodes against a row.
 *
 * The SQL grammar produces a recursive tree of expression nodes.
 * This module walks that tree and computes a `SqlValue`.
 *
 * Expression grammar (abbreviated):
 *
 *   expr            = or_expr
 *   or_expr         = and_expr { "OR" and_expr }
 *   and_expr        = not_expr { "AND" not_expr }
 *   not_expr        = [ "NOT" ] comparison
 *   comparison      = additive { cmp_op additive }
 *                   | additive "BETWEEN" additive "AND" additive
 *                   | additive "IN" "(" value_list ")"
 *                   | additive "LIKE" additive
 *                   | additive "IS" "NULL"
 *                   | additive "IS" "NOT" "NULL"
 *   additive        = multiplicative { ("+" | "-") multiplicative }
 *   multiplicative  = unary { ("*" | "/" | "%") unary }
 *   unary           = [ "-" ] primary
 *   primary         = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
 *                   | column_ref | function_call | "(" expr ")"
 *   column_ref      = NAME [ "." NAME ]
 *   function_call   = NAME "(" ( "*" | expr { "," expr } ) ")"
 *
 * Row context:
 *   `rowCtx` is a `Record<string, SqlValue>` mapping column names
 *   (and optionally "table.column" qualified names) to their values.
 *
 * SQL NULL:
 *   `null` represents SQL NULL. NULL propagates through arithmetic
 *   and comparisons. IS NULL / IS NOT NULL are the only tests for NULL.
 */

import type { ASTNode, Token } from "@coding-adventures/parser";
import { isASTNode } from "@coding-adventures/parser";
import type { Row, SqlValue } from "./types.js";
import { ColumnNotFoundError } from "./errors.js";

/** Evaluate an AST expression node or Token against a row context. */
export function evalExpr(node: ASTNode | Token, rowCtx: Row): SqlValue {
  if (!isASTNode(node)) {
    return evalToken(node as Token, rowCtx);
  }
  return evalNode(node, rowCtx);
}

function evalNode(node: ASTNode, rowCtx: Row): SqlValue {
  const rule = node.ruleName;
  switch (rule) {
    case "expr":
    case "statement":
    case "program":
      return evalExpr(node.children[0], rowCtx);
    case "or_expr":
      return evalOr(node, rowCtx);
    case "and_expr":
      return evalAnd(node, rowCtx);
    case "not_expr":
      return evalNot(node, rowCtx);
    case "comparison":
      return evalComparison(node, rowCtx);
    case "additive":
      return evalAdditive(node, rowCtx);
    case "multiplicative":
      return evalMultiplicative(node, rowCtx);
    case "unary":
      return evalUnary(node, rowCtx);
    case "primary":
      return evalPrimary(node, rowCtx);
    case "column_ref":
      return evalColumnRef(node, rowCtx);
    case "function_call":
      return evalFunctionCall(node, rowCtx);
    default:
      // Pass-through for intermediate nodes
      if (node.children.length > 0) {
        return evalExpr(node.children[0], rowCtx);
      }
      return null;
  }
}

// ---------------------------------------------------------------------------
// Token evaluation
// ---------------------------------------------------------------------------

function evalToken(token: Token, rowCtx: Row): SqlValue {
  const { type, value } = token;
  switch (type) {
    case "NUMBER":
      return value.includes(".") ? parseFloat(value) : parseInt(value, 10);
    case "STRING":
      // The lexer already strips surrounding quotes from string literals
      return value;
    case "KEYWORD": {
      const kw = value.toUpperCase();
      if (kw === "NULL") return null;
      if (kw === "TRUE") return true;
      if (kw === "FALSE") return false;
      return value;
    }
    case "NAME":
      return resolveColumn(value, rowCtx);
    default:
      return value;
  }
}

// ---------------------------------------------------------------------------
// Boolean operators
// ---------------------------------------------------------------------------

function evalOr(node: ASTNode, rowCtx: Row): SqlValue {
  // Grammar: and_expr { "OR" and_expr }
  let result: SqlValue = evalExpr(node.children[0], rowCtx);
  let i = 1;
  while (i < node.children.length) {
    i++; // skip "OR" token
    if (i >= node.children.length) break;
    const right = evalExpr(node.children[i], rowCtx);
    i++;
    // SQL three-valued logic
    if (result === true || right === true) {
      result = true;
    } else if (result === null || right === null) {
      result = null;
    } else {
      result = false;
    }
  }
  return result;
}

function evalAnd(node: ASTNode, rowCtx: Row): SqlValue {
  // Grammar: not_expr { "AND" not_expr }
  let result: SqlValue = evalExpr(node.children[0], rowCtx);
  let i = 1;
  while (i < node.children.length) {
    i++; // skip "AND" token
    if (i >= node.children.length) break;
    const right = evalExpr(node.children[i], rowCtx);
    i++;
    if (result === false || right === false) {
      result = false;
    } else if (result === null || right === null) {
      result = null;
    } else {
      result = isTruthy(result) && isTruthy(right);
    }
  }
  return result;
}

function evalNot(node: ASTNode, rowCtx: Row): SqlValue {
  // Grammar: [ "NOT" ] comparison
  if (isKeyword(node.children[0], "NOT")) {
    const val = evalExpr(node.children[1], rowCtx);
    if (val === null) return null;
    return !isTruthy(val);
  }
  return evalExpr(node.children[0], rowCtx);
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

function evalComparison(node: ASTNode, rowCtx: Row): SqlValue {
  const children = node.children;
  if (children.length === 1) return evalExpr(children[0], rowCtx);

  const left = evalExpr(children[0], rowCtx);
  const secondKw = keywordValue(children[1]);

  // IS NULL / IS NOT NULL
  if (secondKw === "IS") {
    const thirdKw = keywordValue(children[2]);
    return thirdKw === "NOT" ? left !== null : left === null;
  }

  // BETWEEN
  if (secondKw === "BETWEEN") {
    const low = evalExpr(children[2], rowCtx);
    const high = evalExpr(children[4], rowCtx); // children[3] is AND
    if (left === null || low === null || high === null) return null;
    return compareValues(low, left) <= 0 && compareValues(left, high) <= 0;
  }

  // IN (value_list)
  if (secondKw === "IN") {
    const valueList = children[3]; // children[2]="(", [3]=value_list, [4]=")"
    const values = evalValueList(valueList, rowCtx);
    if (left === null) return null;
    return values.includes(left);
  }

  // LIKE
  if (secondKw === "LIKE") {
    const pattern = evalExpr(children[2], rowCtx);
    if (left === null || pattern === null) return null;
    return likeMatch(String(left), String(pattern));
  }

  // NOT IN / NOT LIKE / NOT BETWEEN
  if (secondKw === "NOT") {
    const thirdKw = keywordValue(children[2]);
    if (thirdKw === "BETWEEN") {
      const low = evalExpr(children[3], rowCtx);
      const high = evalExpr(children[5], rowCtx);
      if (left === null || low === null || high === null) return null;
      return !(compareValues(low, left) <= 0 && compareValues(left, high) <= 0);
    }
    if (thirdKw === "IN") {
      const values = evalValueList(children[4], rowCtx);
      if (left === null) return null;
      return !values.includes(left);
    }
    if (thirdKw === "LIKE") {
      const pattern = evalExpr(children[3], rowCtx);
      if (left === null || pattern === null) return null;
      return !likeMatch(String(left), String(pattern));
    }
  }

  // Standard binary operator
  const op = getOpString(children[1]);
  const right = evalExpr(children[2], rowCtx);
  if (left === null || right === null) return null;
  return applyCmp(op, left, right);
}

function applyCmp(op: string, left: SqlValue, right: SqlValue): boolean {
  const cmp = compareValues(left, right);
  switch (op) {
    case "=": return cmp === 0;
    case "!=": case "<>": return cmp !== 0;
    case "<": return cmp < 0;
    case ">": return cmp > 0;
    case "<=": return cmp <= 0;
    case ">=": return cmp >= 0;
    default: return false;
  }
}

function compareValues(a: SqlValue, b: SqlValue): number {
  if (a === null || b === null) return 0;
  if (typeof a === "number" && typeof b === "number") return a < b ? -1 : a > b ? 1 : 0;
  if (typeof a === "string" && typeof b === "string") {
    return a.toLowerCase() < b.toLowerCase() ? -1 : a.toLowerCase() > b.toLowerCase() ? 1 : 0;
  }
  if (typeof a === "boolean" && typeof b === "boolean") return a < b ? -1 : a > b ? 1 : 0;
  return 0;
}

function evalValueList(node: ASTNode | Token, rowCtx: Row): SqlValue[] {
  if (!isASTNode(node)) {
    return [evalToken(node as Token, rowCtx)];
  }
  const values: SqlValue[] = [];
  for (const child of node.children) {
    if (!isASTNode(child) && (child as Token).value === ",") continue;
    values.push(evalExpr(child, rowCtx));
  }
  return values;
}

function likeMatch(value: string, pattern: string): boolean {
  // Convert SQL LIKE pattern to regex
  const regexStr = pattern
    .split("%")
    .map((part) => escapeRegex(part))
    .join(".*");
  return new RegExp(`^${regexStr}$`, "i").test(value);
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

function evalAdditive(node: ASTNode, rowCtx: Row): SqlValue {
  let result: SqlValue = evalExpr(node.children[0], rowCtx);
  let i = 1;
  while (i < node.children.length) {
    const opToken = node.children[i];
    i++;
    const right = evalExpr(node.children[i], rowCtx);
    i++;
    if (result === null || right === null) { result = null; continue; }
    const op = isASTNode(opToken) ? nodeText(opToken) : (opToken as Token).value;
    result = op === "+" ? (result as number) + (right as number) : (result as number) - (right as number);
  }
  return result;
}

function evalMultiplicative(node: ASTNode, rowCtx: Row): SqlValue {
  let result: SqlValue = evalExpr(node.children[0], rowCtx);
  let i = 1;
  while (i < node.children.length) {
    const opToken = node.children[i];
    i++;
    const right = evalExpr(node.children[i], rowCtx);
    i++;
    if (result === null || right === null) { result = null; continue; }
    const op = isASTNode(opToken) ? nodeText(opToken) : (opToken as Token).value;
    const lv = result as number;
    const rv = right as number;
    switch (op) {
      case "*": result = lv * rv; break;
      case "/": result = rv !== 0 ? lv / rv : null; break;
      case "%": result = rv !== 0 ? lv % rv : null; break;
      default: result = null;
    }
  }
  return result;
}

function evalUnary(node: ASTNode, rowCtx: Row): SqlValue {
  const first = node.children[0];
  if (!isASTNode(first) && (first as Token).value === "-") {
    const val = evalExpr(node.children[1], rowCtx);
    return val === null ? null : -(val as number);
  }
  return evalExpr(first, rowCtx);
}

// ---------------------------------------------------------------------------
// Primary
// ---------------------------------------------------------------------------

function evalPrimary(node: ASTNode, rowCtx: Row): SqlValue {
  const children = node.children;
  if (children.length === 0) return null;

  const first = children[0];
  if (!isASTNode(first)) {
    const tok = first as Token;
    if (tok.value === "(") {
      return evalExpr(children[1], rowCtx);
    }
    return evalToken(tok, rowCtx);
  }
  return evalNode(first, rowCtx);
}

// ---------------------------------------------------------------------------
// Column reference
// ---------------------------------------------------------------------------

function evalColumnRef(node: ASTNode, rowCtx: Row): SqlValue {
  const children = node.children;
  if (children.length === 1) {
    const name = (children[0] as Token).value;
    return resolveColumn(name, rowCtx);
  }
  if (children.length >= 3) {
    const table = (children[0] as Token).value;
    const col = (children[2] as Token).value;
    const qualified = `${table}.${col}`;
    if (qualified in rowCtx) return rowCtx[qualified];
    return resolveColumn(col, rowCtx);
  }
  return null;
}

function resolveColumn(name: string, rowCtx: Row): SqlValue {
  if (name in rowCtx) return rowCtx[name];
  const nameLower = name.toLowerCase();
  for (const [k, v] of Object.entries(rowCtx)) {
    if (k.toLowerCase() === nameLower) return v;
  }
  throw new ColumnNotFoundError(name);
}

// ---------------------------------------------------------------------------
// Function call
// ---------------------------------------------------------------------------

function evalFunctionCall(node: ASTNode, rowCtx: Row): SqlValue {
  const funcName = (node.children[0] as Token).value.toUpperCase();

  // Check for pre-computed aggregate value
  const argChildren = node.children.slice(2, node.children.length - 1);
  const aggKey = makeAggKey(funcName, argChildren);
  if (aggKey in rowCtx) return rowCtx[aggKey];

  // Scalar functions
  const argChild = node.children[2];
  const argVal = argChild ? evalExpr(argChild, rowCtx) : null;

  switch (funcName) {
    case "UPPER": return typeof argVal === "string" ? argVal.toUpperCase() : argVal;
    case "LOWER": return typeof argVal === "string" ? argVal.toLowerCase() : argVal;
    case "LENGTH": return typeof argVal === "string" ? argVal.length : null;
    case "ABS": return typeof argVal === "number" ? Math.abs(argVal) : argVal;
    default: return null;
  }
}

function makeAggKey(funcName: string, argChildren: ReadonlyArray<ASTNode | Token>): string {
  const argText = argChildren
    .filter((c) => isASTNode(c) || (c as Token).value !== ",")
    .map((c) => (isASTNode(c) ? nodeText(c as ASTNode) : (c as Token).value))
    .join("");
  return `_agg_${funcName}(${argText})`;
}

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

function isKeyword(child: ASTNode | Token, value: string): boolean {
  if (isASTNode(child)) return false;
  return (child as Token).value.toUpperCase() === value.toUpperCase();
}

function keywordValue(child: ASTNode | Token): string {
  if (!isASTNode(child)) return (child as Token).value.toUpperCase();
  const first = (child as ASTNode).children[0];
  if (first && !isASTNode(first)) return (first as Token).value.toUpperCase();
  return "";
}

function getOpString(child: ASTNode | Token): string {
  if (!isASTNode(child)) return (child as Token).value;
  return (child as ASTNode).children
    .filter((c): c is Token => !isASTNode(c))
    .map((c) => c.value)
    .join("");
}

export function nodeText(node: ASTNode | Token): string {
  if (!isASTNode(node)) return (node as Token).value;
  return node.children.map(nodeText).join("");
}

function isTruthy(val: SqlValue): boolean {
  if (val === null) return false;
  if (typeof val === "boolean") return val;
  if (typeof val === "number") return val !== 0;
  if (typeof val === "string") return val.length > 0;
  return false;
}
