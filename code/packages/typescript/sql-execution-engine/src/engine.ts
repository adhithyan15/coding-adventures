/**
 * Engine — public entry points for SQL execution.
 *
 * Two functions:
 * - `execute(sql, source)` — parse and execute a single SELECT statement.
 * - `executeAll(sql, source)` — parse and execute all SELECT statements.
 */

import type { ASTNode, Token } from "@coding-adventures/parser";
import { isASTNode } from "@coding-adventures/parser";
import { parseSQL } from "coding-adventures-sql-parser";
import type { DataSource } from "./data-source.js";
import type { QueryResult } from "./types.js";
import { executeSelect } from "./executor.js";

/**
 * Parse and execute a single SQL SELECT statement.
 *
 * @param sql    - A SQL string containing at least one SELECT statement.
 * @param source - The data provider implementing the DataSource interface.
 * @returns A QueryResult with column names and result rows.
 * @throws ExecutionError if execution fails (table not found, column not found).
 * @throws GrammarParseError if the SQL has syntax errors.
 */
export function execute(sql: string, source: DataSource): QueryResult {
  const ast = parseSQL(sql);
  const selectNode = findFirstSelect(ast);
  if (!selectNode) return { columns: [], rows: [] };
  return executeSelect(selectNode, source);
}

/**
 * Parse and execute all SELECT statements in a SQL string.
 *
 * Non-SELECT statements are silently skipped.
 *
 * @param sql    - One or more semicolon-separated SQL statements.
 * @param source - The data provider.
 * @returns An array of QueryResult objects, one per SELECT statement.
 */
export function executeAll(sql: string, source: DataSource): QueryResult[] {
  const ast = parseSQL(sql);
  return findAllSelects(ast).map((stmt) => executeSelect(stmt, source));
}

// ---------------------------------------------------------------------------
// AST navigation helpers
// ---------------------------------------------------------------------------

function findFirstSelect(ast: ASTNode): ASTNode | null {
  if (ast.ruleName === "select_stmt") return ast;
  for (const child of ast.children) {
    if (isASTNode(child)) {
      const found = findFirstSelect(child as ASTNode);
      if (found) return found;
    }
  }
  return null;
}

function findAllSelects(ast: ASTNode): ASTNode[] {
  const results: ASTNode[] = [];
  collectSelects(ast, results);
  return results;
}

function collectSelects(node: ASTNode | Token, results: ASTNode[]): void {
  if (!isASTNode(node)) return;
  const n = node as ASTNode;
  if (n.ruleName === "select_stmt") {
    results.push(n);
    return;
  }
  for (const child of n.children) collectSelects(child, results);
}
