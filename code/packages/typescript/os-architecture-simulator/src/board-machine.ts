import type { BoardConfig } from "./board.ts";
import { BOARD_CONFIGS, CONSOLE_PROGRAM, CONSOLE_PROGRAM_ID, createBoard } from "./board.ts";
import type { CpuEvent, CpuState } from "./cpu.ts";

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const object = value as Record<string, unknown>;
    return `{${Object.keys(object).sort().map(key => `${JSON.stringify(key)}:${canonicalJson(object[key])}`).join(",")}}`;
  }
  const encoded = JSON.stringify(value);
  if (encoded === undefined) throw new Error("machine snapshot must be JSON data");
  return encoded;
}

export interface MachineSnapshot {
  schemaVersion: 1;
  board: BoardConfig;
  program: string;
  programBytes: number[];
  cpu: CpuState;
  consoleBytes: number[];
  consoleText: string;
  trace: CpuEvent[];
  budgets: number[];
}

/** Bounded instruction runner with deterministic board composition and replay. */
export class BoardMachine {
  readonly board;
  readonly programId: string;
  private trace: CpuEvent[] = [];
  private budgets: number[] = [];
  private readonly programBytes: Uint8Array;

  constructor(config: BoardConfig, program: Uint8Array = CONSOLE_PROGRAM, programId = CONSOLE_PROGRAM_ID) {
    this.programBytes = program.slice();
    this.board = createBoard(config, this.programBytes);
    this.programId = programId;
  }

  run(budget: number): CpuEvent[] {
    if (!Number.isSafeInteger(budget) || budget < 0) throw new Error("instruction budget must be a nonnegative safe integer");
    this.budgets.push(budget);
    const start = this.trace.length;
    for (let step = 0; step < budget && !this.board.cpu.halted; step++) {
      this.trace.push(...this.board.cpu.step(this.board.bus));
    }
    return this.trace.slice(start).map(event => structuredClone(event));
  }

  snapshot(): MachineSnapshot {
    return {
      schemaVersion: 1,
      board: { ...this.board.config },
      program: this.programId,
      programBytes: Array.from(this.programBytes),
      cpu: this.board.cpu.state,
      consoleBytes: this.board.console.outputBytes,
      consoleText: this.board.console.text,
      trace: this.trace.map(event => structuredClone(event)),
      budgets: [...this.budgets],
    };
  }

  serialize(): string { return JSON.stringify(this.snapshot()); }

  static replay(config: BoardConfig, budgets: number[], program: Uint8Array = CONSOLE_PROGRAM, programId = CONSOLE_PROGRAM_ID): BoardMachine {
    const machine = new BoardMachine(config, program, programId);
    for (const budget of budgets) machine.run(budget);
    return machine;
  }

  static restore(encoded: string): BoardMachine {
    const parsed: unknown = JSON.parse(encoded);
    if (parsed === null || typeof parsed !== "object") throw new Error("machine snapshot must be an object");
    const snapshot = parsed as Partial<MachineSnapshot>;
    if (snapshot.schemaVersion !== 1 || !snapshot.board || typeof snapshot.program !== "string" ||
        !Array.isArray(snapshot.programBytes) || !Array.isArray(snapshot.budgets)) throw new Error("machine snapshot fields are invalid");
    if (!snapshot.programBytes.every(byte => Number.isInteger(byte) && byte >= 0 && byte <= 0xff)) throw new Error("machine snapshot program bytes are invalid");
    const restored = BoardMachine.replay(snapshot.board, snapshot.budgets, new Uint8Array(snapshot.programBytes), snapshot.program);
    if (canonicalJson(restored.snapshot()) !== canonicalJson(snapshot)) throw new Error("machine snapshot does not match replayed budgets");
    return restored;
  }
}

export function runBoardExamples(): MachineSnapshot[] {
  return BOARD_CONFIGS.map(config => BoardMachine.replay(config, [1, 4, 2, 1]).snapshot());
}
