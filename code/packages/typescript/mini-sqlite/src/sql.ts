import type { SqlValue } from "@coding-adventures/sql-execution-engine";
import { ProgrammingError } from "./errors.js";

export interface CreateTableStatement {
  kind: "create";
  table: string;
  columns: string[];
  ifNotExists: boolean;
}

export interface DropTableStatement {
  kind: "drop";
  table: string;
  ifExists: boolean;
}

export interface InsertStatement {
  kind: "insert";
  table: string;
  columns: string[] | null;
  rows: SqlValue[][];
}

export interface UpdateStatement {
  kind: "update";
  table: string;
  assignments: Map<string, SqlValue>;
  where: string | null;
}

export interface DeleteStatement {
  kind: "delete";
  table: string;
  where: string | null;
}

export type MutatingStatement =
  | CreateTableStatement
  | DropTableStatement
  | InsertStatement
  | UpdateStatement
  | DeleteStatement;

export function firstKeyword(sql: string): string {
  const trimmed = sql.trimStart();
  const match = /^[A-Za-z]+/.exec(trimmed);
  return match ? match[0].toUpperCase() : "";
}

export function parseMutatingStatement(sql: string): MutatingStatement {
  const keyword = firstKeyword(sql);
  switch (keyword) {
    case "CREATE":
      return parseCreate(sql);
    case "DROP":
      return parseDrop(sql);
    case "INSERT":
      return parseInsert(sql);
    case "UPDATE":
      return parseUpdate(sql);
    case "DELETE":
      return parseDelete(sql);
    default:
      throw new ProgrammingError(`unsupported SQL statement: ${keyword || sql}`);
  }
}

function parseCreate(sql: string): CreateTableStatement {
  const match = /^\s*CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\(([\s\S]*)\)\s*;?\s*$/i.exec(sql);
  if (!match) throw new ProgrammingError("invalid CREATE TABLE statement");
  const columns = splitTopLevel(match[3], ",")
    .map((part) => identifierAtStart(part.trim()))
    .filter((name) => name.length > 0);
  if (columns.length === 0) {
    throw new ProgrammingError("CREATE TABLE requires at least one column");
  }
  return {
    kind: "create",
    table: match[2],
    columns,
    ifNotExists: Boolean(match[1]),
  };
}

function parseDrop(sql: string): DropTableStatement {
  const match = /^\s*DROP\s+TABLE\s+(IF\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*;?\s*$/i.exec(sql);
  if (!match) throw new ProgrammingError("invalid DROP TABLE statement");
  return {
    kind: "drop",
    table: match[2],
    ifExists: Boolean(match[1]),
  };
}

function parseInsert(sql: string): InsertStatement {
  const match = /^\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*\(([^)]*)\))?\s+VALUES\s+([\s\S]*?)\s*;?\s*$/i.exec(sql);
  if (!match) throw new ProgrammingError("invalid INSERT statement");
  const columns = match[2]
    ? splitTopLevel(match[2], ",").map((part) => normalizeIdentifier(part.trim()))
    : null;
  return {
    kind: "insert",
    table: match[1],
    columns,
    rows: parseValueRows(match[3]),
  };
}

function parseUpdate(sql: string): UpdateStatement {
  const match = /^\s*UPDATE\s+([A-Za-z_][A-Za-z0-9_]*)\s+SET\s+([\s\S]*?)\s*;?\s*$/i.exec(sql);
  if (!match) throw new ProgrammingError("invalid UPDATE statement");
  const [assignmentSql, whereSql] = splitTopLevelKeyword(match[2], "WHERE");
  const assignments = new Map<string, SqlValue>();
  for (const assignment of splitTopLevel(assignmentSql, ",")) {
    const parts = splitTopLevel(assignment, "=");
    if (parts.length !== 2) {
      throw new ProgrammingError(`invalid assignment: ${assignment.trim()}`);
    }
    assignments.set(normalizeIdentifier(parts[0].trim()), parseLiteral(parts[1].trim()));
  }
  if (assignments.size === 0) {
    throw new ProgrammingError("UPDATE requires at least one assignment");
  }
  return {
    kind: "update",
    table: match[1],
    assignments,
    where: whereSql,
  };
}

function parseDelete(sql: string): DeleteStatement {
  const match = /^\s*DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+WHERE\s+([\s\S]*?))?\s*;?\s*$/i.exec(sql);
  if (!match) throw new ProgrammingError("invalid DELETE statement");
  return {
    kind: "delete",
    table: match[1],
    where: match[2]?.trim() || null,
  };
}

function parseValueRows(sql: string): SqlValue[][] {
  const rows: SqlValue[][] = [];
  let rest = sql.trim();

  while (rest.length > 0) {
    if (!rest.startsWith("(")) {
      throw new ProgrammingError("INSERT VALUES rows must be parenthesized");
    }
    const end = findMatchingParen(rest, 0);
    if (end < 0) throw new ProgrammingError("unterminated INSERT VALUES row");
    const inner = rest.slice(1, end);
    rows.push(splitTopLevel(inner, ",").map((part) => parseLiteral(part.trim())));
    rest = rest.slice(end + 1).trim();
    if (rest.startsWith(",")) rest = rest.slice(1).trim();
    else if (rest.length > 0) throw new ProgrammingError("invalid text after INSERT row");
  }

  if (rows.length === 0) throw new ProgrammingError("INSERT requires at least one row");
  return rows;
}

export function parseLiteral(text: string): SqlValue {
  const value = text.trim();
  if (/^NULL$/i.test(value)) return null;
  if (/^TRUE$/i.test(value)) return true;
  if (/^FALSE$/i.test(value)) return false;
  if (/^-?\d+(?:\.\d+)?$/.test(value)) return Number(value);
  if (value.startsWith("'") && value.endsWith("'")) {
    return value.slice(1, -1).replace(/''/g, "'");
  }
  throw new ProgrammingError(`expected literal value, got: ${text}`);
}

export function splitTopLevel(text: string, delimiter: string): string[] {
  const parts: string[] = [];
  let start = 0;
  let depth = 0;
  let quote: string | null = null;

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (quote) {
      if (ch === quote && text[i + 1] === quote) {
        i += 1;
      } else if (ch === quote) {
        quote = null;
      }
      continue;
    }
    if (ch === "'" || ch === "\"") {
      quote = ch;
      continue;
    }
    if (ch === "(") depth += 1;
    else if (ch === ")") depth -= 1;
    else if (depth === 0 && text.startsWith(delimiter, i)) {
      parts.push(text.slice(start, i).trim());
      i += delimiter.length - 1;
      start = i + 1;
    }
  }
  parts.push(text.slice(start).trim());
  return parts.filter((part) => part.length > 0);
}

function splitTopLevelKeyword(text: string, keyword: string): [string, string | null] {
  let depth = 0;
  let quote: string | null = null;
  const upper = text.toUpperCase();
  const needle = keyword.toUpperCase();

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (quote) {
      if (ch === quote && text[i + 1] === quote) i += 1;
      else if (ch === quote) quote = null;
      continue;
    }
    if (ch === "'" || ch === "\"") {
      quote = ch;
      continue;
    }
    if (ch === "(") depth += 1;
    else if (ch === ")") depth -= 1;
    else if (
      depth === 0
      && upper.startsWith(needle, i)
      && isBoundary(text[i - 1])
      && isBoundary(text[i + needle.length])
    ) {
      return [text.slice(0, i).trim(), text.slice(i + needle.length).trim()];
    }
  }
  return [text.trim(), null];
}

function findMatchingParen(text: string, openIndex: number): number {
  let depth = 0;
  let quote: string | null = null;
  for (let i = openIndex; i < text.length; i += 1) {
    const ch = text[i];
    if (quote) {
      if (ch === quote && text[i + 1] === quote) i += 1;
      else if (ch === quote) quote = null;
      continue;
    }
    if (ch === "'" || ch === "\"") {
      quote = ch;
      continue;
    }
    if (ch === "(") depth += 1;
    if (ch === ")") {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function identifierAtStart(text: string): string {
  const match = /^([A-Za-z_][A-Za-z0-9_]*)/.exec(text);
  return match ? match[1] : "";
}

function normalizeIdentifier(text: string): string {
  const match = /^([A-Za-z_][A-Za-z0-9_]*)$/.exec(text);
  if (!match) throw new ProgrammingError(`invalid identifier: ${text}`);
  return match[1];
}

function isBoundary(ch: string | undefined): boolean {
  return ch === undefined || !/[A-Za-z0-9_]/.test(ch);
}
