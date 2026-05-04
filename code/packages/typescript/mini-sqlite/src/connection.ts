import type { SqlValue } from "@coding-adventures/sql-execution-engine";
import { bindParameters, type ParameterValue } from "./binding.js";
import { Cursor } from "./cursor.js";
import { InMemoryDatabase, type StatementResult } from "./database.js";
import { NotSupportedError, ProgrammingError } from "./errors.js";
import { firstKeyword, parseMutatingStatement } from "./sql.js";

export interface ConnectOptions {
  autocommit?: boolean;
}

export class Connection {
  private readonly database = new InMemoryDatabase();
  private closed = false;
  private snapshot: ReturnType<InMemoryDatabase["snapshot"]> | null = null;

  constructor(private readonly autocommit: boolean = false) {}

  cursor(): Cursor {
    this.assertOpen();
    return new Cursor(this);
  }

  execute(sql: string, parameters: ReadonlyArray<ParameterValue> = []): Cursor {
    return this.cursor().execute(sql, parameters);
  }

  executemany(
    sql: string,
    sequenceOfParameters: ReadonlyArray<ReadonlyArray<ParameterValue>>,
  ): Cursor {
    return this.cursor().executemany(sql, sequenceOfParameters);
  }

  commit(): void {
    this.assertOpen();
    this.snapshot = null;
  }

  rollback(): void {
    this.assertOpen();
    if (this.snapshot) {
      this.database.restore(this.snapshot);
      this.snapshot = null;
    }
  }

  close(): void {
    if (this.closed) return;
    if (this.snapshot) this.database.restore(this.snapshot);
    this.snapshot = null;
    this.closed = true;
  }

  _executeBound(sql: string, parameters: ReadonlyArray<ParameterValue>): StatementResult {
    this.assertOpen();
    const bound = bindParameters(sql, parameters);
    const keyword = firstKeyword(bound);

    if (keyword === "BEGIN") {
      this.ensureSnapshot();
      return { columns: [], rows: [], rowsAffected: 0 };
    }
    if (keyword === "COMMIT") {
      this.commit();
      return { columns: [], rows: [], rowsAffected: 0 };
    }
    if (keyword === "ROLLBACK") {
      this.rollback();
      return { columns: [], rows: [], rowsAffected: 0 };
    }
    if (keyword === "SELECT") return this.database.select(bound);

    this.ensureSnapshot();
    const stmt = parseMutatingStatement(bound);
    switch (stmt.kind) {
      case "create":
        return this.database.createTable(stmt);
      case "drop":
        return this.database.dropTable(stmt);
      case "insert":
        return this.database.insert(stmt);
      case "update":
        return this.database.update(stmt);
      case "delete":
        return this.database.delete(stmt);
    }
  }

  private ensureSnapshot(): void {
    if (!this.autocommit && this.snapshot === null) {
      this.snapshot = this.database.snapshot();
    }
  }

  private assertOpen(): void {
    if (this.closed) throw new ProgrammingError("connection is closed");
  }
}

export function connect(
  database: string = ":memory:",
  options: ConnectOptions = {},
): Connection {
  if (database !== ":memory:") {
    throw new NotSupportedError("TypeScript mini-sqlite supports only :memory: in Level 0");
  }
  return new Connection(options.autocommit ?? false);
}

export type RowTuple = SqlValue[];
