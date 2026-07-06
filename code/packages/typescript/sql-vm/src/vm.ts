/**
 * SQL Virtual Machine — executes IR bytecode Programs against an in-memory
 * table store.
 *
 * Architecture: classic stack machine with a cursor-based scan model.
 *
 *   State:
 *     stack      — operand stack (SqlValue[])
 *     cursors    — map from cursorId → table row iterator
 *     rowBuffer  — { [col]: SqlValue } accumulating the current output row
 *     aggBuffer  — { [alias]: SqlValue } holding finalized aggregate results;
 *                  separate from rowBuffer so BeginRow does not clobber them
 *     result     — { columns: string[]; rows: SqlValue[][] } output
 *
 *   Cursor positioning:
 *     cursors[id].pos is the index of the CURRENT row.  AdvanceCursor
 *     increments pos.  JumpIfExhausted checks pos >= rows.length.  All
 *     row reads in the loop body use rows[pos] (not pos-1).
 *
 *   Special tables:
 *     __dual__   — a virtual single-row, zero-column table for FROM-less SELECT
 *
 *   LoadColumn semantics by cursorId:
 *     >= 0       — read from cursor rows[pos][column]
 *     -1         — resolve from active cursors (most-recent first), then rowBuffer
 *     -2         — read from aggBuffer (finalized aggregate results)
 */

import type { Instruction, Program, SortSpec, SqlValue } from "./ir.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export type Database = Map<string, TableData>;

export interface TableData {
  columns: string[];
  rows: Array<Record<string, SqlValue>>;
}

export interface QueryResult {
  columns: string[];
  rows: SqlValue[][];
  /** Rows affected by INSERT/UPDATE/DELETE; -1 for SELECT and DDL. */
  rowsAffected: number;
}

// ---------------------------------------------------------------------------
// VM error
// ---------------------------------------------------------------------------

export class VmError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "VmError";
  }
}

/** Execute a compiled Program against the given Database. */
export function execute(program: Program, db: Database): QueryResult {
  return new SqlVm(program, db).run();
}

// ---------------------------------------------------------------------------
// Internal VM
// ---------------------------------------------------------------------------

interface AggSlot {
  func: string;
  alias: string;
  accum: SqlValue;
  count: number;
}

interface GroupEntry {
  key: SqlValue[];
  slots: AggSlot[];
}

class SqlVm {
  private readonly instructions: Instruction[];
  private readonly labels: Map<string, number>;
  private readonly resultSchema: string[];
  private readonly db: Database;

  private stack: SqlValue[] = [];
  private ip = 0;

  private cursors: Map<number, { rows: Array<Record<string, SqlValue>>; pos: number }> = new Map();

  private rowBuffer: Record<string, SqlValue> = {};
  /** Holds finalized aggregate results.  Separate from rowBuffer so BeginRow won't clobber them. */
  private aggBuffer: Record<string, SqlValue> = {};

  private resultColumns: string[] = [];
  private resultRows: SqlValue[][] = [];
  /** -1 until the first DML instruction runs, then tracks affected row count. */
  private rowsAffected = -1;

  // Aggregate state.
  private aggSlots: AggSlot[] = [];
  private groupMap: Map<string, GroupEntry> = new Map();
  private groupKeys: SqlValue[][] = [];
  private groupCursor = 0;
  private currentGroupKey = "__global__";

  constructor(program: Program, db: Database) {
    this.instructions = program.instructions;
    this.labels = program.labels;
    this.resultSchema = program.resultSchema;
    this.db = db;
  }

  run(): QueryResult {
    while (this.ip < this.instructions.length) {
      this.step(this.instructions[this.ip]);
    }
    return {
      columns: this.resultColumns.length > 0 ? this.resultColumns : this.resultSchema,
      rows: this.resultRows,
      rowsAffected: this.rowsAffected,
    };
  }

  private step(instr: Instruction): void {
    switch (instr.op) {
      // -----------------------------------------------------------------------
      // Stack
      // -----------------------------------------------------------------------
      case "LoadConst":
        this.push(instr.value);
        this.ip++;
        break;

      case "LoadNull":
        this.push(null);
        this.ip++;
        break;

      case "LoadColumn":
        this.push(this.loadColumn(instr.cursorId, instr.column));
        this.ip++;
        break;

      case "Pop":
        this.pop();
        this.ip++;
        break;

      // -----------------------------------------------------------------------
      // Arithmetic / logic / functions
      // -----------------------------------------------------------------------
      case "BinaryOp": {
        const right = this.pop();
        const left = this.pop();
        this.push(this.evalBinaryOp(instr.operator, left, right));
        this.ip++;
        break;
      }

      case "UnaryOp":
        this.push(this.evalUnaryOp(instr.operator, this.pop()));
        this.ip++;
        break;

      case "CallFunc": {
        const args: SqlValue[] = [];
        for (let i = 0; i < instr.arity; i++) args.unshift(this.pop());
        this.push(this.callBuiltinFunc(instr.name, args));
        this.ip++;
        break;
      }

      case "IsNullInstr":
        this.push(this.pop() === null);
        this.ip++;
        break;

      case "IsNotNullInstr":
        this.push(this.pop() !== null);
        this.ip++;
        break;

      case "BetweenInstr": {
        const high = this.pop();
        const low = this.pop();
        const val = this.pop();
        if (val === null || low === null || high === null) {
          this.push(null);
        } else {
          const r = this.sqlCmp(val, low) >= 0 && this.sqlCmp(val, high) <= 0;
          this.push(instr.negated ? !r : r);
        }
        this.ip++;
        break;
      }

      case "LikeInstr": {
        const pattern = this.pop();
        const val = this.pop();
        if (val === null || pattern === null) {
          this.push(null);
        } else {
          const m = sqlLike(String(val), String(pattern));
          this.push(instr.negated ? !m : m);
        }
        this.ip++;
        break;
      }

      case "InList": {
        const items: SqlValue[] = [];
        for (let i = 0; i < instr.count; i++) items.unshift(this.pop());
        const subject = this.pop();
        const found = items.some((v) => sqlEquals(subject, v));
        this.push(instr.negated ? !found : found);
        this.ip++;
        break;
      }

      case "Coalesce": {
        const args: SqlValue[] = [];
        for (let i = 0; i < instr.arity; i++) args.unshift(this.pop());
        this.push(args.find((v) => v !== null) ?? null);
        this.ip++;
        break;
      }

      // -----------------------------------------------------------------------
      // Table scan
      // -----------------------------------------------------------------------
      case "OpenScan":
        this.cursors.set(instr.cursorId, { rows: this.openTable(instr.table), pos: 0 });
        this.ip++;
        break;

      case "AdvanceCursor": {
        const cur = this.cursors.get(instr.cursorId);
        if (cur) cur.pos++;
        this.ip++;
        break;
      }

      case "JumpIfExhausted": {
        const cur = this.cursors.get(instr.cursorId);
        if (!cur || cur.pos >= cur.rows.length) {
          this.jumpTo(instr.label);
        } else {
          this.ip++;
        }
        break;
      }

      case "CloseScan":
        this.cursors.delete(instr.cursorId);
        this.ip++;
        break;

      // -----------------------------------------------------------------------
      // Row output
      // -----------------------------------------------------------------------
      case "BeginRow":
        this.rowBuffer = {};
        this.ip++;
        break;

      case "EmitColumn": {
        const val = this.pop();
        if (instr.name === "__star__") {
          // Star: expand all columns from current active cursors.
          for (const cur of this.cursors.values()) {
            const row = cur.rows[cur.pos];
            if (row) {
              for (const [k, v] of Object.entries(row)) {
                this.rowBuffer[k] = v;
              }
            }
          }
        } else {
          this.rowBuffer[instr.name] = val;
        }
        this.ip++;
        break;
      }

      case "EmitRow": {
        // Include ALL columns (including hidden __sort_ ones) so SortResult can
        // find them for ordering.  SortResult strips the __sort_ prefix afterward.
        const cols = Object.keys(this.rowBuffer);
        if (this.resultColumns.length === 0) {
          this.resultColumns = cols;
        }
        this.resultRows.push(cols.map((c) => this.rowBuffer[c] ?? null));
        this.ip++;
        break;
      }

      case "SetResultSchema":
        this.resultColumns = instr.columns;
        this.ip++;
        break;

      // -----------------------------------------------------------------------
      // Aggregates
      // -----------------------------------------------------------------------
      case "InitAgg":
        this.aggSlots = Array.from({ length: instr.slots }, () => ({
          func: "",
          alias: "",
          accum: null,
          count: 0,
        }));
        this.groupMap = new Map();
        this.groupKeys = [];
        this.groupCursor = 0;
        this.currentGroupKey = "__global__";
        this.aggBuffer = {};
        this.ip++;
        break;

      case "SaveGroupKey": {
        const keyVals: SqlValue[] = [];
        for (let i = 0; i < instr.arity; i++) keyVals.unshift(this.pop());
        const keyStr = JSON.stringify(keyVals);
        if (!this.groupMap.has(keyStr)) {
          const slots: AggSlot[] = this.aggSlots.map((s) => ({ func: s.func, alias: s.alias, accum: null, count: 0 }));
          this.groupMap.set(keyStr, { key: keyVals, slots });
          this.groupKeys.push(keyVals);
        }
        this.currentGroupKey = keyStr;
        this.ip++;
        break;
      }

      case "UpdateAgg": {
        const val = this.pop();
        let group = this.groupMap.get(this.currentGroupKey);
        if (!group) {
          // No GROUP BY — use global group.
          if (!this.groupMap.has("__global__")) {
            const slots: AggSlot[] = this.aggSlots.map(() => ({ func: "", alias: "", accum: null, count: 0 }));
            this.groupMap.set("__global__", { key: [], slots });
            this.groupKeys.push([]);
          }
          group = this.groupMap.get("__global__")!;
        }
        const slot = group.slots[instr.slot];
        if (slot) {
          slot.func = instr.func;
          this.accumulate(slot, val);
        }
        this.ip++;
        break;
      }

      case "FinalizeAgg": {
        const groupKey = this.groupKeys[this.groupCursor];
        const keyStr = JSON.stringify(groupKey ?? []);
        const group = this.groupMap.get(keyStr) ?? this.groupMap.get("__global__");
        if (group) {
          const slot = group.slots[instr.slot];
          if (slot) {
            slot.func = instr.func;
            slot.alias = instr.alias;
            const finalVal = this.finalize(slot);
            this.push(finalVal);
            // Store in aggBuffer (NOT rowBuffer) so BeginRow won't clobber it.
            this.aggBuffer[instr.alias] = finalVal;
          } else {
            this.push(null);
          }
        } else {
          this.push(null);
        }
        this.ip++;
        break;
      }

      case "LoadGroupKey": {
        const groupKey = this.groupKeys[this.groupCursor];
        this.push(groupKey?.[instr.slot] ?? null);
        this.ip++;
        break;
      }

      case "AdvanceGroup":
        this.groupCursor++;
        this.aggBuffer = {}; // clear per-group aggregate buffer
        this.ip++;
        break;

      case "JumpIfGroupsDone":
        if (this.groupCursor >= this.groupKeys.length) {
          this.jumpTo(instr.label);
        } else {
          this.ip++;
        }
        break;

      // -----------------------------------------------------------------------
      // Post-processing
      // -----------------------------------------------------------------------
      case "SortResult": {
        this.resultRows = this.sortRows(this.resultRows, this.resultColumns, instr.keys);
        if (instr.stripPrefix) {
          const stripIndices = this.resultColumns
            .map((c, i) => ({ c, i }))
            .filter(({ c }) => c.startsWith(instr.stripPrefix))
            .map(({ i }) => i);
          if (stripIndices.length > 0) {
            const keepIdx = new Set(stripIndices);
            this.resultColumns = this.resultColumns.filter((_, i) => !keepIdx.has(i));
            this.resultRows = this.resultRows.map((row) => row.filter((_, i) => !keepIdx.has(i)));
          }
        }
        this.ip++;
        break;
      }

      case "DistinctResult": {
        const seen = new Set<string>();
        this.resultRows = this.resultRows.filter((row) => {
          const key = JSON.stringify(row);
          if (seen.has(key)) return false;
          seen.add(key);
          return true;
        });
        this.ip++;
        break;
      }

      case "LimitResult":
        this.resultRows = this.resultRows.slice(instr.offset, instr.offset + instr.count);
        this.ip++;
        break;

      // -----------------------------------------------------------------------
      // DML
      // -----------------------------------------------------------------------
      case "InsertRow": {
        const tableData = this.getOrCreateTable(instr.table);
        const cols = instr.columns ?? tableData.columns;
        const vals: SqlValue[] = [];
        for (let i = 0; i < cols.length; i++) vals.unshift(this.pop());
        const row: Record<string, SqlValue> = {};
        for (let i = 0; i < cols.length; i++) {
          row[cols[i]] = vals[i];
          if (!tableData.columns.includes(cols[i])) tableData.columns.push(cols[i]);
        }
        tableData.rows.push(row);
        if (this.rowsAffected < 0) this.rowsAffected = 0;
        this.rowsAffected++;
        this.ip++;
        break;
      }

      case "UpdateRows": {
        const cur = this.cursors.get(instr.cursorId);
        if (cur) {
          const row = cur.rows[cur.pos]; // current row (AdvanceCursor hasn't run yet)
          if (row) {
            const vals: SqlValue[] = [];
            for (let i = 0; i < instr.columns.length; i++) vals.unshift(this.pop());
            for (let i = 0; i < instr.columns.length; i++) row[instr.columns[i]] = vals[i];
            if (this.rowsAffected < 0) this.rowsAffected = 0;
            this.rowsAffected++;
          }
        }
        this.ip++;
        break;
      }

      case "DeleteRows": {
        const cur = this.cursors.get(instr.cursorId);
        const tableData = this.db.get(instr.table);
        if (cur && tableData) {
          const row = cur.rows[cur.pos]; // current row
          if (row) {
            const idx = tableData.rows.indexOf(row);
            if (idx !== -1) {
              tableData.rows.splice(idx, 1);
              // Rewind pos since we removed the current element; the loop will
              // AdvanceCursor to the same index (now the next row).
              cur.pos--;
              cur.rows = tableData.rows;
              if (this.rowsAffected < 0) this.rowsAffected = 0;
              this.rowsAffected++;
            }
          }
        }
        this.ip++;
        break;
      }

      // -----------------------------------------------------------------------
      // DDL
      // -----------------------------------------------------------------------
      case "CreateTable":
        if (!instr.ifNotExists || !this.db.has(instr.table)) {
          this.db.set(instr.table, { columns: instr.columns.map((c) => c.name), rows: [] });
        }
        this.ip++;
        break;

      case "DropTable":
        if (!instr.ifExists || this.db.has(instr.table)) {
          this.db.delete(instr.table);
        }
        this.ip++;
        break;

      // -----------------------------------------------------------------------
      // Transactions (no-ops in this in-memory engine)
      // -----------------------------------------------------------------------
      case "BeginTransaction":
      case "CommitTransaction":
      case "RollbackTransaction":
        this.ip++;
        break;

      // -----------------------------------------------------------------------
      // Control flow
      // -----------------------------------------------------------------------
      case "Label":
        this.ip++;
        break;

      case "Jump":
        this.jumpTo(instr.label);
        break;

      case "JumpIfTrue":
        if (isTruthy(this.pop())) this.jumpTo(instr.label);
        else this.ip++;
        break;

      case "JumpIfFalse":
        if (!isTruthy(this.pop())) this.jumpTo(instr.label);
        else this.ip++;
        break;

      case "Halt":
        this.ip = this.instructions.length;
        break;

      default:
        throw new VmError(`unknown instruction: ${(instr as { op: string }).op}`);
    }
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  private push(v: SqlValue): void { this.stack.push(v); }
  private pop(): SqlValue {
    if (this.stack.length === 0) throw new VmError("stack underflow");
    return this.stack.pop()!;
  }

  private jumpTo(label: string): void {
    const idx = this.labels.get(label);
    if (idx === undefined) throw new VmError(`unknown label: ${label}`);
    this.ip = idx + 1; // instruction AFTER the Label
  }

  private openTable(table: string): Array<Record<string, SqlValue>> {
    if (table === "__dual__") return [{}]; // single empty row
    const tableData = this.db.get(table);
    if (!tableData) throw new VmError(`no such table: ${table}`);
    return tableData.rows;
  }

  private getOrCreateTable(table: string): TableData {
    if (!this.db.has(table)) this.db.set(table, { columns: [], rows: [] });
    return this.db.get(table)!;
  }

  private loadColumn(cursorId: number, column: string): SqlValue {
    // cursorId -2: read from aggBuffer (finalized aggregate results).
    if (cursorId === -2) return this.aggBuffer[column] ?? null;

    // cursorId -1: search active cursors (reverse order = most recent), then rowBuffer.
    if (cursorId === -1) {
      for (const cur of [...this.cursors.values()].reverse()) {
        const row = cur.rows[cur.pos];
        if (row && column in row) return row[column];
      }
      return this.rowBuffer[column] ?? null;
    }

    // Specific cursor.
    const cur = this.cursors.get(cursorId);
    if (!cur) return null;
    const row = cur.rows[cur.pos];
    return row?.[column] ?? null;
  }

  private evalBinaryOp(op: string, left: SqlValue, right: SqlValue): SqlValue {
    // NULL propagation.
    if (left === null || right === null) {
      // AND/OR have special NULL semantics.
      const uop = op.toUpperCase();
      if (uop === "AND") {
        if (left === false || right === false) return false;
        return null;
      }
      if (uop === "OR") {
        if (left === true || right === true) return true;
        return null;
      }
      return null;
    }

    switch (op) {
      case "+": return typeof left === "number" && typeof right === "number" ? left + right : null;
      case "-": return (left as number) - (right as number);
      case "*": return (left as number) * (right as number);
      case "/": return (right as number) === 0 ? null : (left as number) / (right as number);
      case "%": return (left as number) % (right as number);
      case "||": return String(left) + String(right);
      case "=": return sqlEquals(left, right);
      case "!=":
      case "<>": return !sqlEquals(left, right);
      case "<": return this.sqlCmp(left, right) < 0;
      case "<=": return this.sqlCmp(left, right) <= 0;
      case ">": return this.sqlCmp(left, right) > 0;
      case ">=": return this.sqlCmp(left, right) >= 0;
      case "AND":
      case "and": return !!(left) && !!(right);
      case "OR":
      case "or": return !!(left) || !!(right);
      default:
        throw new VmError(`unknown binary op: ${op}`);
    }
  }

  private evalUnaryOp(op: string, val: SqlValue): SqlValue {
    switch (op) {
      case "-": return val === null ? null : -(val as number);
      case "+": return val;
      case "NOT":
      case "not": return val === null ? null : !val;
      default: throw new VmError(`unknown unary op: ${op}`);
    }
  }

  private callBuiltinFunc(name: string, args: SqlValue[]): SqlValue {
    const lname = name.toLowerCase();
    const a0 = args[0];
    switch (lname) {
      case "upper": return a0 === null ? null : String(a0).toUpperCase();
      case "lower": return a0 === null ? null : String(a0).toLowerCase();
      case "length": return a0 === null ? null : String(a0).length;
      case "abs": return a0 === null ? null : Math.abs(a0 as number);
      case "round": {
        if (a0 === null) return null;
        const places = args[1] !== undefined && args[1] !== null ? (args[1] as number) : 0;
        const factor = Math.pow(10, places);
        return Math.round((a0 as number) * factor) / factor;
      }
      case "substr":
      case "substring": {
        if (a0 === null) return null;
        const s = String(a0);
        const start = ((args[1] as number) ?? 1) - 1;
        const len = args[2] !== undefined && args[2] !== null ? (args[2] as number) : s.length;
        return s.substr(start, len);
      }
      case "trim": return a0 === null ? null : String(a0).trim();
      case "ltrim": return a0 === null ? null : String(a0).trimStart();
      case "rtrim": return a0 === null ? null : String(a0).trimEnd();
      case "replace":
        return a0 === null ? null : String(a0).replaceAll(String(args[1] ?? ""), String(args[2] ?? ""));
      case "coalesce":
        return args.find((v) => v !== null) ?? null;
      case "nullif":
        return sqlEquals(args[0], args[1]) ? null : args[0];
      case "ifnull":
        return args[0] !== null ? args[0] : args[1];
      case "iif":
        return isTruthy(args[0]) ? args[1] : args[2];
      case "typeof":
        if (a0 === null) return "null";
        if (typeof a0 === "number") return Number.isInteger(a0) ? "integer" : "real";
        if (typeof a0 === "string") return "text";
        return "integer"; // boolean
      default:
        throw new VmError(`unknown function: ${name}/${args.length}`);
    }
  }

  private accumulate(slot: AggSlot, val: SqlValue): void {
    switch (slot.func.toUpperCase()) {
      case "COUNT":
        if (val !== null) { slot.count++; slot.accum = slot.count; }
        break;
      case "SUM":
        if (val !== null) slot.accum = slot.accum === null ? (val as number) : (slot.accum as number) + (val as number);
        break;
      case "MIN":
        if (val !== null && (slot.accum === null || this.sqlCmp(val, slot.accum) < 0)) slot.accum = val;
        break;
      case "MAX":
        if (val !== null && (slot.accum === null || this.sqlCmp(val, slot.accum) > 0)) slot.accum = val;
        break;
      case "AVG":
        if (val !== null) { slot.count++; slot.accum = slot.accum === null ? (val as number) : (slot.accum as number) + (val as number); }
        break;
      case "GROUP_CONCAT":
        if (val !== null) slot.accum = slot.accum === null ? String(val) : `${slot.accum},${val}`;
        break;
    }
  }

  private finalize(slot: AggSlot): SqlValue {
    switch (slot.func.toUpperCase()) {
      case "COUNT": return slot.count;
      case "AVG": return slot.count === 0 || slot.accum === null ? null : (slot.accum as number) / slot.count;
      default: return slot.accum;
    }
  }

  private sortRows(rows: SqlValue[][], columns: string[], keys: SortSpec[]): SqlValue[][] {
    const colIndex = new Map(columns.map((c, i) => [c, i]));
    return [...rows].sort((a, b) => {
      for (const key of keys) {
        const idx = colIndex.get(key.column);
        if (idx === undefined) continue;
        const av = a[idx], bv = b[idx];
        if (av === null && bv === null) continue;
        if (av === null) return key.nullsLast ? 1 : -1;
        if (bv === null) return key.nullsLast ? -1 : 1;
        const cmp = this.sqlCmp(av, bv);
        if (cmp !== 0) return key.ascending ? cmp : -cmp;
      }
      return 0;
    });
  }

  sqlCmp(a: SqlValue, b: SqlValue): number {
    if (a === b) return 0;
    if (a === null) return -1;
    if (b === null) return 1;
    if (typeof a === "number" && typeof b === "number") return a - b;
    return String(a) < String(b) ? -1 : 1;
  }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

function isTruthy(v: SqlValue): boolean {
  if (v === null) return false;
  if (typeof v === "boolean") return v;
  if (typeof v === "number") return v !== 0;
  if (typeof v === "string") return v !== "" && v !== "0";
  return true;
}

function sqlEquals(a: SqlValue, b: SqlValue): boolean {
  if (a === null || b === null) return false;
  if (typeof a === "boolean") a = a ? 1 : 0;
  if (typeof b === "boolean") b = b ? 1 : 0;
  return a === b;
}

function sqlLike(value: string, pattern: string): boolean {
  const re = pattern
    .replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
    .replace(/%/g, ".*")
    .replace(/_/g, ".");
  return new RegExp(`^${re}$`, "i").test(value);
}
