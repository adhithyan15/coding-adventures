import {
  execute as executeSelect,
  TableNotFoundError,
  type DataSource,
  type QueryResult,
  type Row,
  type SqlValue,
} from "@coding-adventures/sql-execution-engine";
import {
  IntegrityError,
  OperationalError,
  ProgrammingError,
  translateError,
} from "./errors.js";
import type {
  CreateTableStatement,
  DeleteStatement,
  DropTableStatement,
  InsertStatement,
  UpdateStatement,
} from "./sql.js";

const ROW_ID = "__mini_sqlite_rowid";

export interface StatementResult {
  columns: string[];
  rows: SqlValue[][];
  rowsAffected: number;
}

interface TableData {
  columns: string[];
  rows: Row[];
}

type Snapshot = Map<string, TableData>;

export class InMemoryDatabase implements DataSource {
  private tables = new Map<string, TableData>();

  schema(tableName: string): string[] {
    return [...this.getTable(tableName).columns];
  }

  scan(tableName: string): Row[] {
    return this.getTable(tableName).rows.map((row) => ({ ...row }));
  }

  snapshot(): Snapshot {
    const copy = new Map<string, TableData>();
    for (const [name, table] of this.tables) {
      copy.set(name, {
        columns: [...table.columns],
        rows: table.rows.map((row) => ({ ...row })),
      });
    }
    return copy;
  }

  restore(snapshot: Snapshot): void {
    this.tables = new Map<string, TableData>();
    for (const [name, table] of snapshot) {
      this.tables.set(name, {
        columns: [...table.columns],
        rows: table.rows.map((row) => ({ ...row })),
      });
    }
  }

  createTable(stmt: CreateTableStatement): StatementResult {
    const key = normalizeName(stmt.table);
    if (this.tables.has(key)) {
      if (stmt.ifNotExists) return emptyResult();
      throw new OperationalError(`table already exists: ${stmt.table}`);
    }
    const seen = new Set<string>();
    for (const column of stmt.columns) {
      const normalized = normalizeName(column);
      if (seen.has(normalized)) {
        throw new ProgrammingError(`duplicate column: ${column}`);
      }
      seen.add(normalized);
    }
    this.tables.set(key, { columns: [...stmt.columns], rows: [] });
    return emptyResult();
  }

  dropTable(stmt: DropTableStatement): StatementResult {
    const key = normalizeName(stmt.table);
    if (!this.tables.has(key)) {
      if (stmt.ifExists) return emptyResult();
      throw new OperationalError(`no such table: ${stmt.table}`);
    }
    this.tables.delete(key);
    return emptyResult();
  }

  insert(stmt: InsertStatement): StatementResult {
    const table = this.getTable(stmt.table);
    const insertColumns = stmt.columns ?? table.columns;
    this.assertKnownColumns(table, insertColumns);

    for (const values of stmt.rows) {
      if (values.length !== insertColumns.length) {
        throw new IntegrityError(
          `INSERT expected ${insertColumns.length} values, got ${values.length}`,
        );
      }
      const row = Object.fromEntries(table.columns.map((column) => [column, null])) as Row;
      insertColumns.forEach((column, index) => {
        row[column] = values[index];
      });
      table.rows.push(row);
    }

    return {
      columns: [],
      rows: [],
      rowsAffected: stmt.rows.length,
    };
  }

  update(stmt: UpdateStatement): StatementResult {
    const table = this.getTable(stmt.table);
    this.assertKnownColumns(table, [...stmt.assignments.keys()]);
    const rowIds = this.matchingRowIds(stmt.table, stmt.where);
    for (const rowId of rowIds) {
      const row = table.rows[rowId];
      for (const [column, value] of stmt.assignments) row[column] = value;
    }
    return {
      columns: [],
      rows: [],
      rowsAffected: rowIds.length,
    };
  }

  delete(stmt: DeleteStatement): StatementResult {
    const table = this.getTable(stmt.table);
    const rowIds = new Set(this.matchingRowIds(stmt.table, stmt.where));
    table.rows = table.rows.filter((_, index) => !rowIds.has(index));
    return {
      columns: [],
      rows: [],
      rowsAffected: rowIds.size,
    };
  }

  select(sql: string): StatementResult {
    try {
      const result = executeSelect(sql, this);
      return queryToStatementResult(result);
    } catch (error) {
      throw translateError(error);
    }
  }

  private matchingRowIds(tableName: string, whereSql: string | null): number[] {
    const table = this.getTable(tableName);
    if (!whereSql) return table.rows.map((_, index) => index);
    const source = new RowIdDataSource(tableName, table);
    try {
      const result = executeSelect(
        `SELECT ${ROW_ID} FROM ${tableName} WHERE ${whereSql}`,
        source,
      );
      return result.rows.map((row) => Number(row[ROW_ID]));
    } catch (error) {
      throw translateError(error);
    }
  }

  private getTable(tableName: string): TableData {
    const table = this.tables.get(normalizeName(tableName));
    if (!table) throw new OperationalError(`no such table: ${tableName}`);
    return table;
  }

  private assertKnownColumns(table: TableData, columns: string[]): void {
    const known = new Set(table.columns.map(normalizeName));
    for (const column of columns) {
      if (!known.has(normalizeName(column))) {
        throw new OperationalError(`no such column: ${column}`);
      }
    }
  }
}

class RowIdDataSource implements DataSource {
  constructor(
    private readonly tableName: string,
    private readonly table: TableData,
  ) {}

  schema(tableName: string): string[] {
    this.assertTable(tableName);
    return [...this.table.columns, ROW_ID];
  }

  scan(tableName: string): Row[] {
    this.assertTable(tableName);
    return this.table.rows.map((row, index) => ({ ...row, [ROW_ID]: index }));
  }

  private assertTable(tableName: string): void {
    if (normalizeName(tableName) !== normalizeName(this.tableName)) {
      throw new TableNotFoundError(tableName);
    }
  }
}

function queryToStatementResult(result: QueryResult): StatementResult {
  return {
    columns: result.columns,
    rows: result.rows.map((row) => result.columns.map((column) => row[column] ?? null)),
    rowsAffected: -1,
  };
}

function normalizeName(name: string): string {
  return name.toLowerCase();
}

function emptyResult(): StatementResult {
  return {
    columns: [],
    rows: [],
    rowsAffected: 0,
  };
}
