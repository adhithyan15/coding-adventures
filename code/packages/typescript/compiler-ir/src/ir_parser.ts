/**
 * IR Parser — text → IrProgram
 *
 * =============================================================================
 * Parsing strategy
 * =============================================================================
 *
 * The parser processes the text line by line:
 *
 *   1. Lines starting with ".version" set the program version
 *   2. Lines starting with ".data" add a data declaration
 *   3. Lines starting with ".entry" set the entry label
 *   4. Lines ending with ":" define a label
 *   5. Lines starting with whitespace are instructions
 *   6. Lines starting with ";" are standalone comments
 *   7. Blank lines are skipped
 *
 * Each instruction line is split into: opcode, operands, and optional
 * "; #N" ID comment. Operands are parsed as registers (v0, v1, ...),
 * immediates (42, -1), or labels (any other identifier).
 *
 * =============================================================================
 * Security limits
 * =============================================================================
 *
 * The parser enforces limits to prevent denial-of-service from adversarial
 * input by capping memory allocation:
 *
 *   - MAX_LINES: 1,000,000 lines per file
 *   - MAX_OPERANDS_PER_INSTR: 16 operands per instruction
 *   - MAX_REGISTER_INDEX: 65535 (v0..v65535)
 */

import { parseOp } from "./opcodes.js";
import {
  IrImmediate,
  IrInstruction,
  IrLabel,
  IrOperand,
  IrProgram,
  IrRegister,
  imm,
  lbl,
  reg,
} from "./types.js";
import { IrOp } from "./opcodes.js";

// ──────────────────────────────────────────────────────────────────────────────
// Safety limits
// ──────────────────────────────────────────────────────────────────────────────

const MAX_LINES = 1_000_000;
const MAX_OPERANDS_PER_INSTR = 16;
const MAX_REGISTER_INDEX = 65535;

/**
 * Parse IR text into an IrProgram.
 *
 * Accepts the canonical text format produced by printIr() and reconstructs
 * an equivalent IrProgram. The roundtrip invariant holds:
 *
 *   parseIr(printIr(program)) produces a structurally equivalent program.
 *
 * @param text - The IR text to parse.
 * @returns The parsed IrProgram.
 * @throws {Error} If the text is malformed, unknown opcodes are found, or
 *   safety limits are exceeded.
 *
 * @example
 *   const text = `.version 1\n\n.entry _start\n\n_start:\n  HALT  ; #0\n`;
 *   const prog = parseIr(text);
 *   prog.entryLabel;  // "_start"
 *   prog.instructions[0].opcode;  // IrOp.LABEL
 *   prog.instructions[1].opcode;  // IrOp.HALT
 */
export function parseIr(text: string): IrProgram {
  const program = new IrProgram("_start");
  program.version = 1;

  const lines = text.split("\n");
  if (lines.length > MAX_LINES) {
    throw new Error(
      `input too large: ${lines.length} lines (max ${MAX_LINES})`
    );
  }

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const lineNum = lineIdx + 1;
    const raw = lines[lineIdx];
    const trimmed = raw.trim();

    // Skip blank lines
    if (trimmed === "") continue;

    // .version directive
    if (trimmed.startsWith(".version")) {
      const parts = trimmed.split(/\s+/);
      if (parts.length !== 2) {
        throw new Error(
          `line ${lineNum}: invalid .version directive: ${JSON.stringify(raw)}`
        );
      }
      const v = parseInt(parts[1], 10);
      if (isNaN(v)) {
        throw new Error(
          `line ${lineNum}: invalid version number: ${JSON.stringify(parts[1])}`
        );
      }
      program.version = v;
      continue;
    }

    // .data declaration
    if (trimmed.startsWith(".data")) {
      const parts = trimmed.split(/\s+/);
      if (parts.length !== 4) {
        throw new Error(
          `line ${lineNum}: invalid .data directive: ${JSON.stringify(raw)}`
        );
      }
      const size = parseInt(parts[2], 10);
      if (isNaN(size)) {
        throw new Error(
          `line ${lineNum}: invalid data size: ${JSON.stringify(parts[2])}`
        );
      }
      const init = parseInt(parts[3], 10);
      if (isNaN(init)) {
        throw new Error(
          `line ${lineNum}: invalid data init: ${JSON.stringify(parts[3])}`
        );
      }
      program.addData({ label: parts[1], size, init });
      continue;
    }

    // .entry directive
    if (trimmed.startsWith(".entry")) {
      const parts = trimmed.split(/\s+/);
      if (parts.length !== 2) {
        throw new Error(
          `line ${lineNum}: invalid .entry directive: ${JSON.stringify(raw)}`
        );
      }
      program.entryLabel = parts[1];
      continue;
    }

    // Label definition: line ends with ":" (but not a semicolon comment)
    if (trimmed.endsWith(":") && !trimmed.startsWith(";")) {
      const labelName = trimmed.slice(0, -1);
      program.addInstruction({
        opcode: IrOp.LABEL,
        operands: [lbl(labelName)],
        id: -1, // labels produce no machine code — ID is not meaningful
      });
      continue;
    }

    // Standalone comment line: starts with ";"
    if (trimmed.startsWith(";")) {
      const commentText = trimmed.slice(1).trim();
      // Skip bare ID comments like "; #3" — these are instruction annotations
      if (!commentText.startsWith("#")) {
        program.addInstruction({
          opcode: IrOp.COMMENT,
          operands: [lbl(commentText)],
          id: -1,
        });
      }
      continue;
    }

    // Regular instruction line
    const instr = parseInstructionLine(trimmed, lineNum);
    program.addInstruction(instr);
  }

  return program;
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────────────────────

/**
 * Parse a single instruction line like:
 *   "LOAD_IMM   v0, 42  ; #3"
 *
 * Algorithm:
 *   1. Split off the "; #N" ID comment if present, record the ID.
 *   2. Split the remaining text into opcode + operand tokens.
 *   3. Look up the opcode by name.
 *   4. Parse each comma-separated operand.
 */
function parseInstructionLine(line: string, lineNum: number): IrInstruction {
  // Step 1: extract the "; #N" ID comment if present
  let id = -1;
  let instructionPart = line;
  const idCommentIdx = line.lastIndexOf("; #");
  if (idCommentIdx >= 0) {
    const idStr = line.slice(idCommentIdx + 3).trim();
    const parsed = parseInt(idStr, 10);
    if (!isNaN(parsed)) {
      id = parsed;
    }
    instructionPart = line.slice(0, idCommentIdx).trim();
  }

  // Step 2: split into opcode and operands
  const fields = instructionPart.trim().split(/\s+/).filter((f) => f !== "");
  if (fields.length === 0) {
    throw new Error(`line ${lineNum}: empty instruction`);
  }

  const opcodeName = fields[0];
  const opcode = parseOp(opcodeName);
  if (opcode === undefined) {
    throw new Error(`line ${lineNum}: unknown opcode ${JSON.stringify(opcodeName)}`);
  }

  // Step 3: parse operands (everything after the opcode, comma-separated)
  const operands: IrOperand[] = [];
  if (fields.length > 1) {
    const operandStr = fields.slice(1).join(" ");
    const parts = operandStr.split(",");
    if (parts.length > MAX_OPERANDS_PER_INSTR) {
      throw new Error(
        `line ${lineNum}: too many operands (${parts.length}, max ${MAX_OPERANDS_PER_INSTR})`
      );
    }
    for (const part of parts) {
      const trimmedPart = part.trim();
      if (trimmedPart === "") continue;
      operands.push(parseOperand(trimmedPart, lineNum));
    }
  }

  return { opcode, operands, id };
}

/**
 * Parse a single operand string into an IrOperand.
 *
 * Parsing rules (in order of precedence):
 *   1. Starts with "v" followed by digits → IrRegister { index: N }
 *   2. Parseable as integer               → IrImmediate { value: N }
 *   3. Anything else                      → IrLabel { name: str }
 *
 * Examples:
 *   "v0"        → IrRegister { index: 0 }
 *   "v5"        → IrRegister { index: 5 }
 *   "42"        → IrImmediate { value: 42 }
 *   "-1"        → IrImmediate { value: -1 }
 *   "_start"    → IrLabel { name: "_start" }
 *   "loop_0_end"→ IrLabel { name: "loop_0_end" }
 */
function parseOperand(s: string, lineNum: number): IrOperand {
  // Register: v0, v1, v2, ... (must be "v" followed by only digits)
  if (s.length > 1 && s[0] === "v") {
    const rest = s.slice(1);
    const idx = parseInt(rest, 10);
    if (!isNaN(idx) && String(idx) === rest) {
      // Valid integer suffix with no extra characters
      if (idx < 0 || idx > MAX_REGISTER_INDEX) {
        throw new Error(
          `line ${lineNum}: register index ${idx} out of range (max ${MAX_REGISTER_INDEX})`
        );
      }
      return reg(idx);
    }
    // Not a valid register number — fall through to label
  }

  // Immediate: 42, -1, 255, ...
  // parseInt("-1", 10) returns -1, and String(-1) === "-1", so this handles
  // negative numbers naturally without any special case.
  const val = parseInt(s, 10);
  if (!isNaN(val) && String(val) === s) {
    return imm(val);
  }

  // Label: _start, loop_0_end, tape, ...
  return lbl(s);
}

// Re-export types used by tests so they don't need to reach into types.ts directly
export type { IrRegister, IrImmediate, IrLabel };
