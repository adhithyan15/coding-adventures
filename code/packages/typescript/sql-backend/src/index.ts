export type SqlValue = null | boolean | number | string | Uint8Array;
export type Row = Record<string, SqlValue>;

export function isSqlValue(value: unknown): value is SqlValue {
  return (
    value === null ||
    typeof value === "boolean" ||
    (typeof value === "number" && Number.isFinite(value)) ||
    typeof value === "string" ||
    value instanceof Uint8Array
  );
}

export function sqlTypeName(value: SqlValue): string {
  if (value === null) {
    return "NULL";
  }
  if (typeof value === "boolean") {
    return "BOOLEAN";
  }
  if (typeof value === "number") {
    return Number.isInteger(value) ? "INTEGER" : "REAL";
  }
  if (typeof value === "string") {
    return "TEXT";
  }
  if (value instanceof Uint8Array) {
    return "BLOB";
  }
  throw new TypeError(`not a SQL value: ${String(value)}`);
}

export function compareSqlValues(left: SqlValue, right: SqlValue): number {
  const rankCompare = valueRank(left) - valueRank(right);
  if (rankCompare !== 0) {
    return Math.sign(rankCompare);
  }
  if (left === null || right === null) {
    return 0;
  }
  if (typeof left === "boolean" && typeof right === "boolean") {
    return Number(left) - Number(right);
  }
  if (typeof left === "number" && typeof right === "number") {
    return Math.sign(left - right);
  }
  if (typeof left === "string" && typeof right === "string") {
    return left < right ? -1 : left > right ? 1 : 0;
  }
  if (left instanceof Uint8Array && right instanceof Uint8Array) {
    const limit = Math.min(left.length, right.length);
    for (let i = 0; i < limit; i += 1) {
      const delta = left[i] - right[i];
      if (delta !== 0) {
        return Math.sign(delta);
      }
    }
    return Math.sign(left.length - right.length);
  }
  return 0;
}

function valueRank(value: SqlValue): number {
  if (value === null) {
    return 0;
  }
  if (typeof value === "boolean") {
    return 1;
  }
  if (typeof value === "number") {
    return 2;
  }
  if (typeof value === "string") {
    return 3;
  }
  return 4;
}

export interface RowIterator {
  next(): Row | null;
  close(): void;
}

export interface Cursor extends RowIterator {
  currentRow(): Row | null;
}

export class ListRowIterator implements RowIterator {
  private readonly rows: Row[];
  private index = 0;
  private closed = false;

  constructor(rows: Iterable<Row>) {
    this.rows = Array.from(rows, copyRow);
  }

  next(): Row | null {
    if (this.closed || this.index >= this.rows.length) {
      return null;
    }
    const row = this.rows[this.index];
    this.index += 1;
    return copyRow(row);
  }

  close(): void {
    this.closed = true;
  }
}

export class ListCursor implements Cursor {
  private readonly rows: Row[];
  private index = -1;
  private current: Row | null = null;
  private closed = false;

  constructor(rows: Row[]) {
    this.rows = rows;
  }

  get currentIndex(): number {
    return this.index;
  }

  isBackedBy(rows: Row[]): boolean {
    return this.rows === rows;
  }

  adjustAfterDelete(): void {
    this.index -= 1;
    this.current = null;
  }

  currentRow(): Row | null {
    return this.current === null ? null : copyRow(this.current);
  }

  next(): Row | null {
    if (this.closed) {
      return null;
    }
    this.index += 1;
    if (this.index >= this.rows.length) {
      this.current = null;
      return null;
    }
    this.current = this.rows[this.index];
    return copyRow(this.current);
  }

  close(): void {
    this.closed = true;
  }
}

export interface ColumnDefOptions {
  readonly notNull?: boolean;
  readonly primaryKey?: boolean;
  readonly unique?: boolean;
  readonly autoincrement?: boolean;
  readonly defaultValue?: SqlValue;
  readonly hasDefault?: boolean;
  readonly checkExpression?: unknown;
  readonly foreignKey?: unknown;
}

export class ColumnDef {
  readonly name: string;
  readonly typeName: string;
  readonly notNull: boolean;
  readonly primaryKey: boolean;
  readonly unique: boolean;
  readonly autoincrement: boolean;
  readonly defaultValue: SqlValue;
  readonly hasDefault: boolean;
  readonly checkExpression: unknown;
  readonly foreignKey: unknown;

  constructor(name: string, typeName: string, options: ColumnDefOptions = {}) {
    this.name = name;
    this.typeName = typeName;
    this.notNull = options.notNull ?? false;
    this.primaryKey = options.primaryKey ?? false;
    this.unique = options.unique ?? false;
    this.autoincrement = options.autoincrement ?? false;
    this.hasDefault =
      options.hasDefault ?? Object.hasOwn(options, "defaultValue");
    this.defaultValue = copyValue(options.defaultValue ?? null);
    this.checkExpression = options.checkExpression;
    this.foreignKey = options.foreignKey;
  }

  static withDefault(
    name: string,
    typeName: string,
    defaultValue: SqlValue,
    options: Omit<ColumnDefOptions, "defaultValue" | "hasDefault"> = {}
  ): ColumnDef {
    return new ColumnDef(name, typeName, {
      ...options,
      defaultValue,
      hasDefault: true,
    });
  }

  get effectiveNotNull(): boolean {
    return this.notNull || this.primaryKey;
  }

  get effectiveUnique(): boolean {
    return this.unique || this.primaryKey;
  }

  clone(): ColumnDef {
    return new ColumnDef(this.name, this.typeName, {
      notNull: this.notNull,
      primaryKey: this.primaryKey,
      unique: this.unique,
      autoincrement: this.autoincrement,
      defaultValue: this.defaultValue,
      hasDefault: this.hasDefault,
      checkExpression: this.checkExpression,
      foreignKey: this.foreignKey,
    });
  }
}

export class TriggerDef {
  readonly name: string;
  readonly table: string;
  readonly timing: string;
  readonly event: string;
  readonly body: string;

  constructor(name: string, table: string, timing: string, event: string, body: string) {
    this.name = name;
    this.table = table;
    this.timing = timing;
    this.event = event;
    this.body = body;
  }

  clone(): TriggerDef {
    return new TriggerDef(this.name, this.table, this.timing, this.event, this.body);
  }
}

export class IndexDef {
  readonly name: string;
  readonly table: string;
  readonly columns: string[];
  readonly unique: boolean;
  readonly auto: boolean;

  constructor(
    name: string,
    table: string,
    options: { readonly columns?: readonly string[]; readonly unique?: boolean; readonly auto?: boolean } = {}
  ) {
    this.name = name;
    this.table = table;
    this.columns = [...(options.columns ?? [])];
    this.unique = options.unique ?? false;
    this.auto = options.auto ?? false;
  }

  clone(): IndexDef {
    return new IndexDef(this.name, this.table, {
      columns: this.columns,
      unique: this.unique,
      auto: this.auto,
    });
  }
}

export class TransactionHandle {
  readonly value: number;

  constructor(value: number) {
    this.value = value;
  }
}

export class BackendError extends Error {
  constructor(message: string) {
    super(message);
    this.name = new.target.name;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export class TableNotFound extends BackendError {
  readonly table: string;

  constructor(table: string) {
    super(`table not found: '${table}'`);
    this.table = table;
  }
}

export class TableAlreadyExists extends BackendError {
  readonly table: string;

  constructor(table: string) {
    super(`table already exists: '${table}'`);
    this.table = table;
  }
}

export class ColumnNotFound extends BackendError {
  readonly table: string;
  readonly column: string;

  constructor(table: string, column: string) {
    super(`column not found: '${table}.${column}'`);
    this.table = table;
    this.column = column;
  }
}

export class ColumnAlreadyExists extends BackendError {
  readonly table: string;
  readonly column: string;

  constructor(table: string, column: string) {
    super(`column already exists: '${table}.${column}'`);
    this.table = table;
    this.column = column;
  }
}

export class ConstraintViolation extends BackendError {
  readonly table: string;
  readonly column: string;

  constructor(table: string, column: string, message: string) {
    super(message);
    this.table = table;
    this.column = column;
  }
}

export class Unsupported extends BackendError {
  readonly operation: string;

  constructor(operation: string) {
    super(`operation not supported: ${operation}`);
    this.operation = operation;
  }
}

export class Internal extends BackendError {
  constructor(message: string) {
    super(message);
  }
}

export class IndexAlreadyExists extends BackendError {
  readonly index: string;

  constructor(index: string) {
    super(`index already exists: '${index}'`);
    this.index = index;
  }
}

export class IndexNotFound extends BackendError {
  readonly index: string;

  constructor(index: string) {
    super(`index not found: '${index}'`);
    this.index = index;
  }
}

export class TriggerAlreadyExists extends BackendError {
  readonly trigger: string;

  constructor(trigger: string) {
    super(`trigger already exists: '${trigger}'`);
    this.trigger = trigger;
  }
}

export class TriggerNotFound extends BackendError {
  readonly trigger: string;

  constructor(trigger: string) {
    super(`trigger not found: '${trigger}'`);
    this.trigger = trigger;
  }
}

export abstract class Backend {
  abstract tables(): string[];
  abstract columns(table: string): ColumnDef[];
  abstract scan(table: string): RowIterator;
  abstract insert(table: string, row: Row): void;
  abstract update(table: string, cursor: Cursor, assignments: Record<string, SqlValue>): void;
  abstract delete(table: string, cursor: Cursor): void;
  abstract createTable(table: string, columns: ColumnDef[], ifNotExists: boolean): void;
  abstract dropTable(table: string, ifExists: boolean): void;
  abstract addColumn(table: string, column: ColumnDef): void;
  abstract createIndex(index: IndexDef): void;
  abstract dropIndex(name: string, options?: { readonly ifExists?: boolean }): void;
  abstract listIndexes(table?: string): IndexDef[];
  abstract scanIndex(
    indexName: string,
    lo: readonly SqlValue[] | null,
    hi: readonly SqlValue[] | null,
    options?: { readonly loInclusive?: boolean; readonly hiInclusive?: boolean }
  ): Iterable<number>;
  abstract scanByRowids(table: string, rowids: readonly number[]): RowIterator;
  abstract beginTransaction(): TransactionHandle;
  abstract commit(handle: TransactionHandle): void;
  abstract rollback(handle: TransactionHandle): void;

  currentTransaction(): TransactionHandle | null {
    return null;
  }

  createSavepoint(_name: string): void {
    throw new Unsupported("savepoints");
  }

  releaseSavepoint(_name: string): void {
    throw new Unsupported("savepoints");
  }

  rollbackToSavepoint(_name: string): void {
    throw new Unsupported("savepoints");
  }

  createTrigger(_defn: TriggerDef): void {
    throw new Unsupported("triggers");
  }

  dropTrigger(_name: string, _options: { readonly ifExists?: boolean } = {}): void {
    throw new Unsupported("triggers");
  }

  listTriggers(_table: string): TriggerDef[] {
    return [];
  }

  getUserVersion(): number {
    return 0;
  }

  setUserVersion(_value: number): void {
    throw new Unsupported("user_version");
  }

  getSchemaVersion(): number {
    return 0;
  }
}

export interface SchemaProvider {
  columns(table: string): string[];
}

export function backendAsSchemaProvider(backend: Pick<Backend, "columns">): SchemaProvider {
  return {
    columns(table: string): string[] {
      return backend.columns(table).map((column) => column.name);
    },
  };
}

export class InMemoryBackend extends Backend {
  private tablesByName = new Map<string, TableState>();
  private indexesByName = new Map<string, IndexDef>();
  private triggersByName = new Map<string, TriggerDef>();
  private triggersByTable = new Map<string, TriggerDef[]>();
  private transactionSnapshot: Snapshot | null = null;
  private savepoints: Savepoint[] = [];
  private activeHandle: TransactionHandle | null = null;
  private nextHandle = 1;
  private userVersion = 0;
  private schemaVersion = 0;

  static fromTables(tables: Record<string, { columns: ColumnDef[]; rows: Row[] }>): InMemoryBackend {
    const backend = new InMemoryBackend();
    for (const [name, data] of Object.entries(tables)) {
      backend.tablesByName.set(normalizeName(name), new TableState(name, data.columns, data.rows));
    }
    return backend;
  }

  tables(): string[] {
    return Array.from(this.tablesByName.values(), (table) => table.name);
  }

  columns(table: string): ColumnDef[] {
    return this.requireTable(table).columns.map((column) => column.clone());
  }

  scan(table: string): RowIterator {
    return new ListRowIterator(this.requireTable(table).rows);
  }

  openCursor(table: string): ListCursor {
    return new ListCursor(this.requireTable(table).rows);
  }

  insert(table: string, row: Row): void {
    const state = this.requireTable(table);
    const normalized = this.normalizeRow(table, state, row);
    this.checkNotNull(table, state, normalized);
    this.checkUnique(table, state, normalized, null);
    state.rows.push(normalized);
  }

  update(table: string, cursor: Cursor, assignments: Record<string, SqlValue>): void {
    const state = this.requireTable(table);
    const listCursor = this.requireListCursor(table, state, cursor);
    const index = listCursor.currentIndex;
    if (index < 0 || index >= state.rows.length) {
      throw new Unsupported("update without current row");
    }

    const updated = copyRow(state.rows[index]);
    for (const [name, value] of Object.entries(assignments)) {
      assertSqlValue(value);
      updated[this.canonicalColumn(table, state, name)] = copyValue(value);
    }
    this.checkNotNull(table, state, updated);
    this.checkUnique(table, state, updated, index);
    state.rows[index] = updated;
  }

  delete(table: string, cursor: Cursor): void {
    const state = this.requireTable(table);
    const listCursor = this.requireListCursor(table, state, cursor);
    const index = listCursor.currentIndex;
    if (index < 0 || index >= state.rows.length) {
      throw new Unsupported("delete without current row");
    }

    state.rows.splice(index, 1);
    listCursor.adjustAfterDelete();
  }

  createTable(table: string, columns: ColumnDef[], ifNotExists: boolean): void {
    const key = normalizeName(table);
    if (this.tablesByName.has(key)) {
      if (ifNotExists) {
        return;
      }
      throw new TableAlreadyExists(table);
    }

    const seen = new Set<string>();
    for (const column of columns) {
      const columnKey = normalizeName(column.name);
      if (seen.has(columnKey)) {
        throw new ColumnAlreadyExists(table, column.name);
      }
      seen.add(columnKey);
    }

    this.tablesByName.set(key, new TableState(table, columns));
    this.bumpSchemaVersion();
  }

  dropTable(table: string, ifExists: boolean): void {
    const removed = this.tablesByName.delete(normalizeName(table));
    if (!removed) {
      if (ifExists) {
        return;
      }
      throw new TableNotFound(table);
    }

    this.deleteIndexesForTable(table);
    this.deleteTriggersForTable(table);
    this.bumpSchemaVersion();
  }

  addColumn(table: string, column: ColumnDef): void {
    const state = this.requireTable(table);
    if (state.columns.some((existing) => sameName(existing.name, column.name))) {
      throw new ColumnAlreadyExists(table, column.name);
    }
    if (state.rows.length > 0 && column.effectiveNotNull && !column.hasDefault) {
      throw new ConstraintViolation(
        table,
        column.name,
        `NOT NULL constraint failed: ${table}.${column.name}`
      );
    }

    const cloned = column.clone();
    state.columns.push(cloned);
    for (const row of state.rows) {
      row[cloned.name] = cloned.hasDefault ? copyValue(cloned.defaultValue) : null;
    }
    this.bumpSchemaVersion();
  }

  createIndex(index: IndexDef): void {
    if (this.indexesByName.has(normalizeName(index.name))) {
      throw new IndexAlreadyExists(index.name);
    }
    const state = this.requireTable(index.table);
    for (const column of index.columns) {
      this.canonicalColumn(index.table, state, column);
    }
    this.indexesByName.set(normalizeName(index.name), index.clone());
    this.bumpSchemaVersion();
  }

  dropIndex(name: string, options: { readonly ifExists?: boolean } = {}): void {
    const removed = this.indexesByName.delete(normalizeName(name));
    if (!removed) {
      if (options.ifExists) {
        return;
      }
      throw new IndexNotFound(name);
    }
    this.bumpSchemaVersion();
  }

  listIndexes(table?: string): IndexDef[] {
    return Array.from(this.indexesByName.values())
      .filter((index) => table === undefined || sameName(index.table, table))
      .map((index) => index.clone());
  }

  *scanIndex(
    indexName: string,
    lo: readonly SqlValue[] | null,
    hi: readonly SqlValue[] | null,
    options: { readonly loInclusive?: boolean; readonly hiInclusive?: boolean } = {}
  ): Iterable<number> {
    const index = this.indexesByName.get(normalizeName(indexName));
    if (index === undefined) {
      throw new IndexNotFound(indexName);
    }
    const state = this.requireTable(index.table);
    const keyed = state.rows.map((row, rowid) => ({
      key: index.columns.map((column) => row[this.canonicalColumn(index.table, state, column)]),
      rowid,
    }));
    keyed.sort((left, right) => {
      const keyCompare = compareKey(left.key, right.key);
      return keyCompare === 0 ? left.rowid - right.rowid : keyCompare;
    });

    const loInclusive = options.loInclusive ?? true;
    const hiInclusive = options.hiInclusive ?? true;

    for (const row of keyed) {
      if (lo !== null) {
        const cmp = comparePrefix(row.key, lo);
        if (cmp < 0 || (cmp === 0 && !loInclusive)) {
          continue;
        }
      }
      if (hi !== null) {
        const cmp = comparePrefix(row.key, hi);
        if (cmp > 0 || (cmp === 0 && !hiInclusive)) {
          break;
        }
      }
      yield row.rowid;
    }
  }

  scanByRowids(table: string, rowids: readonly number[]): RowIterator {
    const state = this.requireTable(table);
    return new ListRowIterator(rowids.flatMap((rowid) => {
      if (rowid < 0 || rowid >= state.rows.length) {
        return [];
      }
      return [state.rows[rowid]];
    }));
  }

  beginTransaction(): TransactionHandle {
    if (this.activeHandle !== null) {
      throw new Unsupported("nested transactions");
    }
    const handle = new TransactionHandle(this.nextHandle);
    this.nextHandle += 1;
    this.transactionSnapshot = this.captureSnapshot();
    this.activeHandle = handle;
    return handle;
  }

  commit(handle: TransactionHandle): void {
    this.requireActive(handle);
    this.transactionSnapshot = null;
    this.activeHandle = null;
    this.savepoints = [];
  }

  rollback(handle: TransactionHandle): void {
    this.requireActive(handle);
    if (this.transactionSnapshot !== null) {
      this.restoreSnapshot(this.transactionSnapshot);
    }
    this.transactionSnapshot = null;
    this.activeHandle = null;
    this.savepoints = [];
  }

  currentTransaction(): TransactionHandle | null {
    return this.activeHandle;
  }

  createSavepoint(name: string): void {
    if (this.activeHandle === null) {
      this.beginTransaction();
    }
    this.savepoints.push({ name, snapshot: this.captureSnapshot() });
  }

  releaseSavepoint(name: string): void {
    const index = this.findSavepoint(name);
    if (index < 0) {
      throw new Unsupported(`RELEASE '${name}': no such savepoint`);
    }
    this.savepoints.splice(index);
  }

  rollbackToSavepoint(name: string): void {
    const index = this.findSavepoint(name);
    if (index < 0) {
      throw new Unsupported(`ROLLBACK TO '${name}': no such savepoint`);
    }
    this.restoreSnapshot(this.savepoints[index].snapshot);
    this.savepoints.splice(index + 1);
  }

  createTrigger(defn: TriggerDef): void {
    if (this.triggersByName.has(normalizeName(defn.name))) {
      throw new TriggerAlreadyExists(defn.name);
    }
    const cloned = defn.clone();
    this.triggersByName.set(normalizeName(defn.name), cloned);
    const tableKey = normalizeName(defn.table);
    this.triggersByTable.set(tableKey, [
      ...(this.triggersByTable.get(tableKey) ?? []),
      cloned,
    ]);
  }

  dropTrigger(name: string, options: { readonly ifExists?: boolean } = {}): void {
    const key = normalizeName(name);
    const existing = this.triggersByName.get(key);
    if (existing === undefined) {
      if (options.ifExists) {
        return;
      }
      throw new TriggerNotFound(name);
    }
    this.triggersByName.delete(key);
    const tableKey = normalizeName(existing.table);
    const remaining = (this.triggersByTable.get(tableKey) ?? []).filter(
      (trigger) => !sameName(trigger.name, name)
    );
    if (remaining.length === 0) {
      this.triggersByTable.delete(tableKey);
    } else {
      this.triggersByTable.set(tableKey, remaining);
    }
  }

  listTriggers(table: string): TriggerDef[] {
    return (this.triggersByTable.get(normalizeName(table)) ?? []).map((trigger) => trigger.clone());
  }

  getUserVersion(): number {
    return this.userVersion;
  }

  setUserVersion(value: number): void {
    if (!Number.isInteger(value) || value < 0 || value > 0xffffffff) {
      throw new RangeError(`user_version must fit in u32, got ${value}`);
    }
    this.userVersion = value;
  }

  getSchemaVersion(): number {
    return this.schemaVersion;
  }

  private requireTable(table: string): TableState {
    const state = this.tablesByName.get(normalizeName(table));
    if (state === undefined) {
      throw new TableNotFound(table);
    }
    return state;
  }

  private requireListCursor(table: string, state: TableState, cursor: Cursor): ListCursor {
    if (cursor instanceof ListCursor && cursor.isBackedBy(state.rows)) {
      return cursor;
    }
    throw new Unsupported(`foreign cursor for table ${table}`);
  }

  private normalizeRow(table: string, state: TableState, row: Row): Row {
    const normalized: Row = {};
    for (const [name, value] of Object.entries(row)) {
      assertSqlValue(value);
      normalized[this.canonicalColumn(table, state, name)] = copyValue(value);
    }
    for (const column of state.columns) {
      if (!Object.hasOwn(normalized, column.name)) {
        normalized[column.name] = column.hasDefault ? copyValue(column.defaultValue) : null;
      }
    }
    return normalized;
  }

  private checkNotNull(table: string, state: TableState, row: Row): void {
    for (const column of state.columns) {
      if (column.effectiveNotNull && row[column.name] === null) {
        throw new ConstraintViolation(
          table,
          column.name,
          `NOT NULL constraint failed: ${table}.${column.name}`
        );
      }
    }
  }

  private checkUnique(table: string, state: TableState, row: Row, ignoreIndex: number | null): void {
    for (const column of state.columns) {
      if (!column.effectiveUnique) {
        continue;
      }
      const value = row[column.name];
      if (value === null) {
        continue;
      }
      for (let i = 0; i < state.rows.length; i += 1) {
        if (ignoreIndex === i) {
          continue;
        }
        if (compareSqlValues(state.rows[i][column.name], value) === 0) {
          const label = column.primaryKey ? "PRIMARY KEY" : "UNIQUE";
          throw new ConstraintViolation(
            table,
            column.name,
            `${label} constraint failed: ${table}.${column.name}`
          );
        }
      }
    }
  }

  private canonicalColumn(table: string, state: TableState, column: string): string {
    const found = state.columns.find((candidate) => sameName(candidate.name, column));
    if (found === undefined) {
      throw new ColumnNotFound(table, column);
    }
    return found.name;
  }

  private requireActive(handle: TransactionHandle): void {
    if (this.activeHandle === null) {
      throw new Unsupported("no active transaction");
    }
    if (this.activeHandle.value !== handle.value) {
      throw new Unsupported("stale transaction handle");
    }
  }

  private captureSnapshot(): Snapshot {
    return {
      tables: cloneTableMap(this.tablesByName),
      indexes: cloneIndexMap(this.indexesByName),
      triggers: cloneTriggerMap(this.triggersByName),
      triggersByTable: cloneTriggersByTableMap(this.triggersByTable),
      userVersion: this.userVersion,
      schemaVersion: this.schemaVersion,
    };
  }

  private restoreSnapshot(snapshot: Snapshot): void {
    this.tablesByName = cloneTableMap(snapshot.tables);
    this.indexesByName = cloneIndexMap(snapshot.indexes);
    this.triggersByName = cloneTriggerMap(snapshot.triggers);
    this.triggersByTable = cloneTriggersByTableMap(snapshot.triggersByTable);
    this.userVersion = snapshot.userVersion;
    this.schemaVersion = snapshot.schemaVersion;
  }

  private findSavepoint(name: string): number {
    for (let i = this.savepoints.length - 1; i >= 0; i -= 1) {
      if (this.savepoints[i].name === name) {
        return i;
      }
    }
    return -1;
  }

  private deleteIndexesForTable(table: string): void {
    for (const [key, index] of this.indexesByName.entries()) {
      if (sameName(index.table, table)) {
        this.indexesByName.delete(key);
      }
    }
  }

  private deleteTriggersForTable(table: string): void {
    const tableKey = normalizeName(table);
    for (const [key, trigger] of this.triggersByName.entries()) {
      if (sameName(trigger.table, table)) {
        this.triggersByName.delete(key);
      }
    }
    this.triggersByTable.delete(tableKey);
  }

  private bumpSchemaVersion(): void {
    this.schemaVersion = (this.schemaVersion + 1) >>> 0;
  }
}

class TableState {
  readonly name: string;
  readonly columns: ColumnDef[];
  readonly rows: Row[];

  constructor(name: string, columns: readonly ColumnDef[], rows: readonly Row[] = []) {
    this.name = name;
    this.columns = columns.map((column) => column.clone());
    this.rows = rows.map(copyRow);
  }

  copy(): TableState {
    return new TableState(this.name, this.columns, this.rows);
  }
}

interface Snapshot {
  readonly tables: Map<string, TableState>;
  readonly indexes: Map<string, IndexDef>;
  readonly triggers: Map<string, TriggerDef>;
  readonly triggersByTable: Map<string, TriggerDef[]>;
  readonly userVersion: number;
  readonly schemaVersion: number;
}

interface Savepoint {
  readonly name: string;
  readonly snapshot: Snapshot;
}

function assertSqlValue(value: unknown): asserts value is SqlValue {
  if (!isSqlValue(value)) {
    throw new TypeError(`not a SQL value: ${String(value)}`);
  }
}

function copyRow(row: Row): Row {
  const copy: Row = {};
  for (const [name, value] of Object.entries(row)) {
    copy[name] = copyValue(value);
  }
  return copy;
}

function copyValue(value: SqlValue): SqlValue {
  return value instanceof Uint8Array ? new Uint8Array(value) : value;
}

function normalizeName(name: string): string {
  return name.toLowerCase();
}

function sameName(left: string, right: string): boolean {
  return normalizeName(left) === normalizeName(right);
}

function compareKey(left: readonly SqlValue[], right: readonly SqlValue[]): number {
  const limit = Math.min(left.length, right.length);
  for (let i = 0; i < limit; i += 1) {
    const cmp = compareSqlValues(left[i], right[i]);
    if (cmp !== 0) {
      return cmp;
    }
  }
  return Math.sign(left.length - right.length);
}

function comparePrefix(key: readonly SqlValue[], bound: readonly SqlValue[]): number {
  for (let i = 0; i < bound.length; i += 1) {
    const value = i < key.length ? key[i] : null;
    const cmp = compareSqlValues(value, bound[i]);
    if (cmp !== 0) {
      return cmp;
    }
  }
  return 0;
}

function cloneTableMap(source: Map<string, TableState>): Map<string, TableState> {
  return new Map(Array.from(source.entries(), ([key, table]) => [key, table.copy()]));
}

function cloneIndexMap(source: Map<string, IndexDef>): Map<string, IndexDef> {
  return new Map(Array.from(source.entries(), ([key, index]) => [key, index.clone()]));
}

function cloneTriggerMap(source: Map<string, TriggerDef>): Map<string, TriggerDef> {
  return new Map(Array.from(source.entries(), ([key, trigger]) => [key, trigger.clone()]));
}

function cloneTriggersByTableMap(source: Map<string, TriggerDef[]>): Map<string, TriggerDef[]> {
  return new Map(
    Array.from(source.entries(), ([key, triggers]) => [
      key,
      triggers.map((trigger) => trigger.clone()),
    ])
  );
}
