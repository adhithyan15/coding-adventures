/**
 * IR — Intermediate Representation bytecode instruction set.
 *
 * The code generator (codegen.ts) compiles a LogicalPlan into a flat array of
 * these instructions. The virtual machine (sql-vm) executes them.
 *
 * Design: classic stack machine.
 *
 *   State spaces:
 *     stack        — operand stack; all expressions push/pop values here
 *     cursors      — open table iterators, indexed by cursor ID (number)
 *     row_buffer   — column name → value accumulator for the current output row
 *     result       — final list of (columns, rows) pairs
 *     agg_table    — per-group aggregate accumulators
 *
 *   Execution starts at instruction 0.  Labels are resolved to indices before
 *   execution begins via a pre-built label index.
 *
 *   All values are SQL values: null | boolean | number | string.
 */

export type SqlValue = null | boolean | number | string;

// ---------------------------------------------------------------------------
// Instruction discriminated union
// ---------------------------------------------------------------------------

export type Instruction =
  // Stack
  | { op: "LoadConst"; value: SqlValue }
  | { op: "LoadNull" }
  | { op: "LoadColumn"; cursorId: number; column: string }
  | { op: "Pop" }

  // Arithmetic / logic / functions
  | { op: "BinaryOp"; operator: string }
  | { op: "UnaryOp"; operator: string }
  | { op: "CallFunc"; name: string; arity: number }
  | { op: "IsNullInstr" }
  | { op: "IsNotNullInstr" }
  | { op: "BetweenInstr"; negated: boolean }
  | { op: "LikeInstr"; negated: boolean }
  | { op: "InList"; count: number; negated: boolean }
  | { op: "Coalesce"; arity: number }

  // Table scan
  | { op: "OpenScan"; cursorId: number; table: string }
  | { op: "AdvanceCursor"; cursorId: number }
  | { op: "JumpIfExhausted"; cursorId: number; label: string }
  | { op: "CloseScan"; cursorId: number }

  // Row output
  | { op: "BeginRow" }
  | { op: "EmitColumn"; name: string }
  | { op: "EmitRow" }
  | { op: "SetResultSchema"; columns: string[] }

  // Aggregates
  | { op: "InitAgg"; slots: number }
  | { op: "UpdateAgg"; slot: number; func: string }
  | { op: "FinalizeAgg"; slot: number; func: string; alias: string }
  | { op: "SaveGroupKey"; arity: number }
  | { op: "LoadGroupKey"; slot: number }
  | { op: "AdvanceGroup" }
  | { op: "JumpIfGroupsDone"; label: string }

  // Post-processing (applied after the scan loop)
  | { op: "SortResult"; keys: SortSpec[]; stripPrefix: string }
  | { op: "DistinctResult" }
  | { op: "LimitResult"; count: number; offset: number }

  // DML
  | { op: "InsertRow"; table: string; columns: string[] | null }
  | { op: "UpdateRows"; table: string; columns: string[]; cursorId: number }
  | { op: "DeleteRows"; table: string; cursorId: number }

  // DDL
  | { op: "CreateTable"; table: string; columns: ColumnSpec[]; ifNotExists: boolean }
  | { op: "DropTable"; table: string; ifExists: boolean }

  // Transactions
  | { op: "BeginTransaction" }
  | { op: "CommitTransaction" }
  | { op: "RollbackTransaction" }

  // Control flow
  | { op: "Label"; name: string }
  | { op: "Jump"; label: string }
  | { op: "JumpIfTrue"; label: string }
  | { op: "JumpIfFalse"; label: string }
  | { op: "Halt" };

/** Sort specification for SortResult. */
export interface SortSpec {
  /** Column name in the output row to sort by. */
  column: string;
  ascending: boolean;
  nullsLast: boolean;
}

/** Column spec for CreateTable instruction. */
export interface ColumnSpec {
  name: string;
  dataType: string;
  notNull: boolean;
  primaryKey: boolean;
  unique: boolean;
  defaultValue: SqlValue;
}

// ---------------------------------------------------------------------------
// Program
// ---------------------------------------------------------------------------

/**
 * A compiled program ready for execution by sql-vm.
 *
 * `instructions` is the flat bytecode array.
 * `labels` maps label names to their index in `instructions`
 *   (the index of the Label instruction itself; the VM jumps to
 *   the instruction AFTER the label).
 * `resultSchema` lists the output column names in projection order.
 */
export interface Program {
  instructions: Instruction[];
  labels: Map<string, number>;
  resultSchema: string[];
}

/** Build the label index from a completed instruction list. */
export function buildLabelIndex(instructions: Instruction[]): Map<string, number> {
  const index = new Map<string, number>();
  for (let i = 0; i < instructions.length; i++) {
    const instr = instructions[i];
    if (instr.op === "Label") {
      index.set(instr.name, i);
    }
  }
  return index;
}
