/**
 * Level 1 InMemoryDatabase — routes ALL SQL through the full pipeline:
 *
 *   sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
 *
 * This replaces the Level 0 implementation that used sql-execution-engine for
 * SELECT and a hand-rolled regex parser for DML/DDL.
 */

import { parseSQL } from "coding-adventures-sql-parser";
import { plan, PlanError } from "@coding-adventures/sql-planner";
import { optimize } from "@coding-adventures/sql-optimizer";
import { compile, CodegenError } from "@coding-adventures/sql-codegen";
import { execute as vmExecute, VmError } from "@coding-adventures/sql-vm";
import type { Database } from "@coding-adventures/sql-vm";
import type { SqlValue } from "@coding-adventures/sql-codegen";
import {
  IntegrityError,
  OperationalError,
  ProgrammingError,
  translateError,
} from "./errors.js";

export type { SqlValue };

export interface StatementResult {
  columns: string[];
  rows: SqlValue[][];
  rowsAffected: number;
}

// ---------------------------------------------------------------------------
// Snapshot type for transaction rollback
// ---------------------------------------------------------------------------

type TableSnapshot = {
  columns: string[];
  rows: Array<Record<string, SqlValue>>;
};
type Snapshot = Map<string, TableSnapshot>;

// ---------------------------------------------------------------------------
// InMemoryDatabase
// ---------------------------------------------------------------------------

export class InMemoryDatabase {
  /** The live table store passed to sql-vm. */
  private db: Database = new Map();

  execute(sql: string): StatementResult {
    try {
      const ast = parseSQL(sql);
      const logical = plan(ast);
      const optimized = optimize(logical);
      const program = compile(optimized);
      const result = vmExecute(program, this.db);
      return {
        columns: result.columns,
        rows: result.rows,
        rowsAffected: result.rowsAffected,
      };
    } catch (error) {
      throw translateLevel1Error(error);
    }
  }

  snapshot(): Snapshot {
    const copy: Snapshot = new Map();
    for (const [name, table] of this.db) {
      copy.set(name, {
        columns: [...table.columns],
        rows: table.rows.map((row) => ({ ...row })),
      });
    }
    return copy;
  }

  restore(snap: Snapshot): void {
    this.db = new Map();
    for (const [name, table] of snap) {
      this.db.set(name, {
        columns: [...table.columns],
        rows: table.rows.map((row) => ({ ...row })),
      });
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function translateLevel1Error(error: unknown): OperationalError | ProgrammingError | IntegrityError {
  if (error instanceof OperationalError) return error;
  if (error instanceof ProgrammingError) return error;
  if (error instanceof IntegrityError) return error;
  if (error instanceof VmError || error instanceof PlanError || error instanceof CodegenError) {
    const msg = (error as globalThis.Error).message.toLowerCase();
    if (msg.includes("table") || msg.includes("no such")) {
      return new OperationalError((error as globalThis.Error).message);
    }
    if (msg.includes("column")) {
      return new OperationalError((error as globalThis.Error).message);
    }
    return new ProgrammingError((error as globalThis.Error).message);
  }
  return translateError(error) as OperationalError;
}
