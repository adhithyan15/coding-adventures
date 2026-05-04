export type SqlValue = null | number | string | boolean;

export type Row = Record<string, SqlValue>;

export interface QueryResult {
  columns: string[];
  rows: Row[];
}

export interface DataSource {
  schema(tableName: string): string[];
  scan(tableName: string): Row[];
}

export declare class ExecutionError extends Error {}

export declare class TableNotFoundError extends ExecutionError {
  readonly tableName: string;
  constructor(tableName: string);
}

export declare class ColumnNotFoundError extends ExecutionError {
  readonly columnName: string;
  constructor(columnName: string);
}

export declare function execute(sql: string, source: DataSource): QueryResult;
