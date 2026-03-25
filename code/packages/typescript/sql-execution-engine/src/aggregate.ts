/**
 * Aggregate functions — COUNT, SUM, AVG, MIN, MAX.
 *
 * Aggregate functions summarize a group of rows into a single value.
 *
 * NULL-skipping semantics:
 *   All aggregate functions except COUNT(*) ignore NULL values.
 *   If all values are NULL, SUM/AVG/MIN/MAX return null.
 *   COUNT returns 0 in that case.
 *
 * Aggregate key convention:
 *   Computed values are stored under keys like "_agg_COUNT(*)",
 *   "_agg_SUM(salary)", etc. so that the SELECT projection and
 *   HAVING clause can reference them.
 */

import type { Row, SqlValue } from "./types.js";

/** A `[functionName, argument]` pair, e.g. `["COUNT", "*"]`. */
export type AggSpec = [string, string];

/**
 * Compute aggregate functions over a group of rows.
 *
 * @param rows     - All rows in the current group.
 * @param aggSpecs - Array of `[funcName, arg]` pairs.
 * @returns A map from `"_agg_FUNC(arg)"` keys to their values.
 */
export function computeAggregates(
  rows: Row[],
  aggSpecs: AggSpec[]
): Record<string, SqlValue> {
  const result: Record<string, SqlValue> = {};
  for (const [funcName, arg] of aggSpecs) {
    const key = `_agg_${funcName.toUpperCase()}(${arg})`;
    result[key] = computeOne(rows, funcName.toUpperCase(), arg);
  }
  return result;
}

function computeOne(rows: Row[], func: string, arg: string): SqlValue {
  switch (func) {
    case "COUNT": {
      if (arg === "*") return rows.length;
      return rows.filter((row) => getVal(row, arg) !== null).length;
    }
    case "SUM": {
      const vals = rows.map((r) => getVal(r, arg)).filter((v): v is number => typeof v === "number");
      if (vals.length === 0) return null;
      return vals.reduce((a, b) => a + b, 0);
    }
    case "AVG": {
      const vals = rows.map((r) => getVal(r, arg)).filter((v): v is number => typeof v === "number");
      if (vals.length === 0) return null;
      return vals.reduce((a, b) => a + b, 0) / vals.length;
    }
    case "MIN": {
      const vals = rows.map((r) => getVal(r, arg)).filter((v) => v !== null) as (number | string | boolean)[];
      if (vals.length === 0) return null;
      return vals.reduce((a, b) => (a < b ? a : b));
    }
    case "MAX": {
      const vals = rows.map((r) => getVal(r, arg)).filter((v) => v !== null) as (number | string | boolean)[];
      if (vals.length === 0) return null;
      return vals.reduce((a, b) => (a > b ? a : b));
    }
    default:
      return null;
  }
}

function getVal(row: Row, col: string): SqlValue {
  if (col in row) return row[col];
  const colLower = col.toLowerCase();
  for (const [k, v] of Object.entries(row)) {
    if (k.toLowerCase() === colLower) return v;
  }
  return null;
}
