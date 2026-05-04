import type { SqlValue } from "@coding-adventures/sql-execution-engine";
import { ProgrammingError } from "./errors.js";

export type ParameterValue = SqlValue;

export function bindParameters(
  sql: string,
  parameters: ReadonlyArray<ParameterValue> = [],
): string {
  let index = 0;
  let output = "";
  let i = 0;

  while (i < sql.length) {
    const ch = sql[i];
    const next = sql[i + 1];

    if (ch === "'" || ch === "\"") {
      const [literal, end] = readQuoted(sql, i, ch);
      output += literal;
      i = end;
      continue;
    }

    if (ch === "-" && next === "-") {
      const end = readLineComment(sql, i);
      output += sql.slice(i, end);
      i = end;
      continue;
    }

    if (ch === "/" && next === "*") {
      const end = readBlockComment(sql, i);
      output += sql.slice(i, end);
      i = end;
      continue;
    }

    if (ch === "?") {
      if (index >= parameters.length) {
        throw new ProgrammingError("not enough parameters for SQL statement");
      }
      output += toSqlLiteral(parameters[index]);
      index += 1;
      i += 1;
      continue;
    }

    output += ch;
    i += 1;
  }

  if (index !== parameters.length) {
    throw new ProgrammingError("too many parameters for SQL statement");
  }

  return output;
}

function readQuoted(sql: string, start: number, quote: string): [string, number] {
  let i = start + 1;
  while (i < sql.length) {
    if (sql[i] === quote) {
      if (sql[i + 1] === quote) {
        i += 2;
        continue;
      }
      return [sql.slice(start, i + 1), i + 1];
    }
    i += 1;
  }
  return [sql.slice(start), sql.length];
}

function readLineComment(sql: string, start: number): number {
  let i = start + 2;
  while (i < sql.length && sql[i] !== "\n") i += 1;
  return i;
}

function readBlockComment(sql: string, start: number): number {
  let i = start + 2;
  while (i + 1 < sql.length) {
    if (sql[i] === "*" && sql[i + 1] === "/") return i + 2;
    i += 1;
  }
  return sql.length;
}

function toSqlLiteral(value: ParameterValue): string {
  if (value === null) return "NULL";
  if (typeof value === "boolean") return value ? "TRUE" : "FALSE";
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new ProgrammingError("non-finite numeric parameter is not supported");
    }
    return String(value);
  }
  if (typeof value === "string") return `'${value.replace(/'/g, "''")}'`;
  throw new ProgrammingError(`unsupported parameter type: ${typeof value}`);
}
