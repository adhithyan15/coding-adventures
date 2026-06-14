import type { SqlValue } from "@coding-adventures/sql-execution-engine";
import type { ParameterValue } from "./binding.js";
import type { Connection, RowTuple } from "./connection.js";
import { ProgrammingError } from "./errors.js";

export type DescriptionItem = [string, null, null, null, null, null, null];

export class Cursor implements Iterable<RowTuple> {
  description: DescriptionItem[] | null = null;
  rowcount = -1;
  lastrowid: number | null = null;
  arraysize = 1;

  private rows: RowTuple[] = [];
  private offset = 0;
  private closed = false;

  constructor(private readonly connection: Connection) {}

  execute(sql: string, parameters: ReadonlyArray<ParameterValue> = []): Cursor {
    this.assertOpen();
    const result = this.connection._executeBound(sql, parameters);
    this.rows = result.rows;
    this.offset = 0;
    this.rowcount = result.rowsAffected;
    this.description =
      result.columns.length > 0
        ? result.columns.map((column) => [
            column,
            null,
            null,
            null,
            null,
            null,
            null,
          ] as DescriptionItem)
        : null;
    return this;
  }

  executemany(
    sql: string,
    sequenceOfParameters: ReadonlyArray<ReadonlyArray<ParameterValue>>,
  ): Cursor {
    this.assertOpen();
    let total = 0;
    let last: Cursor = this;
    for (const parameters of sequenceOfParameters) {
      last = this.execute(sql, parameters);
      if (this.rowcount > 0) total += this.rowcount;
    }
    if (sequenceOfParameters.length > 0) this.rowcount = total;
    return last;
  }

  fetchone(): RowTuple | null {
    this.assertOpen();
    if (this.offset >= this.rows.length) return null;
    const row = this.rows[this.offset];
    this.offset += 1;
    return row;
  }

  fetchmany(size: number = this.arraysize): RowTuple[] {
    this.assertOpen();
    const take = size < 0 ? this.arraysize : size;
    const out = this.rows.slice(this.offset, this.offset + take);
    this.offset += out.length;
    return out;
  }

  fetchall(): RowTuple[] {
    this.assertOpen();
    const out = this.rows.slice(this.offset);
    this.offset = this.rows.length;
    return out;
  }

  close(): void {
    this.closed = true;
    this.rows = [];
    this.description = null;
  }

  [Symbol.iterator](): Iterator<RowTuple> {
    return {
      next: (): IteratorResult<RowTuple> => {
        const value = this.fetchone();
        return value === null ? { done: true, value: undefined } : { done: false, value };
      },
    };
  }

  private assertOpen(): void {
    if (this.closed) throw new ProgrammingError("cursor is closed");
  }
}

export type { SqlValue };
