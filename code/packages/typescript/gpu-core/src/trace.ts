/**
 * Execution traces -- making every instruction's journey visible.
 *
 * === Why Traces? ===
 *
 * A key principle of this project is educational transparency: every operation
 * should be observable. When a GPU core executes an instruction, the trace
 * records exactly what happened:
 *
 *     Cycle 3 | PC=2 | FFMA R3, R0, R1, R2
 *     -> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
 *     -> Registers changed: {R3: 7.0}
 *     -> Next PC: 3
 *
 * This lets a student (or debugger) follow the execution step by step,
 * understanding not just *what* the GPU did but *why* -- which registers were
 * read, what computation was performed, and what state changed.
 *
 * === Trace vs Log ===
 *
 * A trace is more structured than a log message. Each field is typed and
 * accessible programmatically, which enables:
 * - Automated testing (assert trace.registersChanged["R3"] === 7.0)
 * - Visualization tools (render execution as a timeline)
 * - Performance analysis (count cycles, track register usage)
 */

import type { Instruction } from "./opcodes.js";
import { formatInstruction } from "./opcodes.js";

/**
 * A record of one instruction's execution on a GPU core.
 *
 * Every call to GPUCore.step() returns one of these, providing full
 * visibility into what the instruction did.
 *
 * Fields:
 *     cycle:             The clock cycle number (1-indexed).
 *     pc:                The program counter BEFORE this instruction executed.
 *     instruction:       The instruction that was executed.
 *     description:       Human-readable description of what happened.
 *                        Example: "R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0"
 *     registersChanged:  Which registers changed and their new values.
 *                        Example: {"R3": 7.0}
 *     memoryChanged:     Which memory addresses changed and their new values.
 *                        Example: {0: 3.14, 4: 2.71}
 *     nextPc:            The program counter AFTER this instruction.
 *     halted:            True if this instruction stopped execution.
 */
export interface GPUCoreTrace {
  readonly cycle: number;
  readonly pc: number;
  readonly instruction: Instruction;
  readonly description: string;
  readonly nextPc: number;
  readonly halted: boolean;
  readonly registersChanged: Record<string, number>;
  readonly memoryChanged: Record<number, number>;
}

/**
 * Pretty-print a trace record for educational display.
 *
 * Returns a multi-line string like:
 *
 *     [Cycle 3] PC=2: FFMA R3, R0, R1, R2
 *       -> R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0
 *       -> Registers: {R3: 7.0}
 *       -> Next PC: 3
 */
export function formatTrace(trace: GPUCoreTrace): string {
  const lines: string[] = [
    `[Cycle ${trace.cycle}] PC=${trace.pc}: ${formatInstruction(trace.instruction)}`,
  ];
  lines.push(`  -> ${trace.description}`);

  const regEntries = Object.entries(trace.registersChanged);
  if (regEntries.length > 0) {
    const regs = regEntries.map(([k, v]) => `${k}=${v}`).join(", ");
    lines.push(`  -> Registers: {${regs}}`);
  }

  const memEntries = Object.entries(trace.memoryChanged);
  if (memEntries.length > 0) {
    const mems = memEntries
      .map(([k, v]) => `0x${Number(k).toString(16).toUpperCase().padStart(4, "0")}=${v}`)
      .join(", ");
    lines.push(`  -> Memory: {${mems}}`);
  }

  if (trace.halted) {
    lines.push("  -> HALTED");
  } else {
    lines.push(`  -> Next PC: ${trace.nextPc}`);
  }

  return lines.join("\n");
}
