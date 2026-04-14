/**
 * IR Printer — IrProgram → human-readable text
 *
 * =============================================================================
 * Text Format
 * =============================================================================
 *
 * The printer produces a canonical text format:
 *
 *   .version 1
 *
 *   .data tape 30000 0
 *
 *   .entry _start
 *
 *   _start:
 *     LOAD_ADDR   v0, tape          ; #0
 *     LOAD_IMM    v1, 0             ; #1
 *     HALT                          ; #2
 *
 * Key rules:
 *   - .version N is always the first non-comment line
 *   - .data declarations come before .entry
 *   - Labels are on their own unindented line with a trailing colon
 *   - Instructions are indented with two spaces
 *   - Operands are comma-separated
 *   - ; #N comments show instruction IDs (informational)
 *   - COMMENT instructions emit as "; <text>" on their own line
 *   - Opcode names are left-padded to 11 characters for alignment
 *
 * =============================================================================
 * Purposes
 * =============================================================================
 *
 *   1. Debugging — humans can read the IR to understand what the compiler did
 *   2. Golden-file tests — expected IR output is committed as .ir text files
 *   3. Roundtrip — parse(print(program)) == program is a testable invariant
 */

import { IrOp, opToString } from "./opcodes.js";
import { IrProgram, operandToString } from "./types.js";

/**
 * Convert an IrProgram to its canonical text representation.
 *
 * The output can be parsed back by parseIr() to recover an equivalent
 * IrProgram (roundtrip fidelity).
 *
 * @param program - The IR program to print.
 * @returns The canonical text representation.
 *
 * @example
 *   const prog = new IrProgram("_start");
 *   prog.addData({ label: "tape", size: 30000, init: 0 });
 *   prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 0 });
 *   console.log(printIr(prog));
 *   // .version 1
 *   //
 *   // .data tape 30000 0
 *   //
 *   // .entry _start
 *   //   HALT                          ; #0
 */
export function printIr(program: IrProgram): string {
  const lines: string[] = [];

  // Version directive — always first
  lines.push(`.version ${program.version}`);

  // Data declarations — one per line, each preceded by a blank line
  for (const d of program.data) {
    lines.push("");
    lines.push(`.data ${d.label} ${d.size} ${d.init}`);
  }

  // Entry point
  lines.push("");
  lines.push(`.entry ${program.entryLabel}`);

  // Instructions
  for (const instr of program.instructions) {
    if (instr.opcode === IrOp.LABEL) {
      // Labels get their own unindented line with a trailing colon
      // A blank line before the label improves readability.
      lines.push("");
      lines.push(`${instr.operands[0] ? operandToString(instr.operands[0]) : ""}:`);
      continue;
    }

    if (instr.opcode === IrOp.COMMENT) {
      // Comments emit as "  ; <text>" — indented but no opcode column
      const text =
        instr.operands.length > 0 ? operandToString(instr.operands[0]) : "";
      lines.push(`  ; ${text}`);
      continue;
    }

    // Regular instruction: "  OPCODE     operands  ; #ID"
    // The opcode name is left-padded to 11 characters for column alignment.
    const opName = opToString(instr.opcode).padEnd(11);
    const operandStr = instr.operands.map(operandToString).join(", ");
    const idComment = `; #${instr.id}`;

    // Build the line with some whitespace between operands and ID comment
    const instrPart = operandStr.length > 0
      ? `  ${opName}${operandStr}`
      : `  ${opName.trimEnd()}`;
    lines.push(`${instrPart}  ${idComment}`);
  }

  return lines.join("\n") + "\n";
}
