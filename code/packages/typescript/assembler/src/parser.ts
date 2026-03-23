/**
 * Assembly Parser — turns raw text into structured data.
 *
 * === What does an assembly parser do? ===
 *
 * Assembly language is a textual representation of machine instructions.
 * Before we can translate it to binary, we need to understand the structure
 * of each line. The parser's job is to take raw text like:
 *
 *   MOV R0, #1      ; load 1 into register 0
 *   loop:
 *       ADD R0, R0, #1
 *       CMP R0, #10
 *       BNE loop
 *   HLT
 *
 * And produce structured data like:
 *
 *   [
 *     { kind: "instruction", mnemonic: "MOV", operands: [reg(0), imm(1)] },
 *     { kind: "label", name: "loop" },
 *     { kind: "instruction", mnemonic: "ADD", operands: [reg(0), reg(0), imm(1)] },
 *     { kind: "instruction", mnemonic: "CMP", operands: [reg(0), imm(10)] },
 *     { kind: "instruction", mnemonic: "BNE", condition: "NE", operands: [label("loop")] },
 *     { kind: "instruction", mnemonic: "HLT", operands: [] },
 *   ]
 *
 * === Why is parsing separate from assembling? ===
 *
 * Separation of concerns. The parser only understands syntax (how the text
 * is structured). The assembler understands semantics (what the instructions
 * mean and how to encode them). This makes both easier to test and modify.
 *
 * === Grammar of ARM assembly ===
 *
 * Each line is one of:
 *   - Empty or whitespace-only → skip
 *   - Comment-only (starts with ; or //) → skip
 *   - Label definition: identifier followed by colon (e.g., "loop:")
 *   - Directive: starts with . (e.g., ".global _start")
 *   - Instruction: mnemonic [condition] [operands] [; comment]
 *
 * An instruction line has the form:
 *   MNEMONIC[COND] [Rd, ] [Rn, ] operand2 [; comment]
 *
 * Operands are separated by commas and can be:
 *   - Register: R0-R15, SP, LR, PC
 *   - Immediate: #decimal, #0xHex, #0bBinary
 *   - Label reference: identifier (for branch targets)
 *   - Memory: [Rn] or [Rn, #offset] (for LDR/STR)
 */

import type { Operand, ParsedLine, AssemblyError } from "./types.js";
import {
  CONDITION_CODES,
  OPCODES,
  REGISTER_ALIASES,
  BRANCH_INSTRUCTIONS,
  MEMORY_INSTRUCTIONS,
} from "./types.js";

// ---------------------------------------------------------------------------
// Known mnemonics
// ---------------------------------------------------------------------------
// We need to know all valid mnemonics so we can separate the mnemonic
// from a possible condition suffix. For example, "ADDNE" is mnemonic "ADD"
// with condition "NE", not a mnemonic called "ADDNE".

const ALL_MNEMONICS = new Set([
  ...OPCODES.keys(),
  ...BRANCH_INSTRUCTIONS,
  ...MEMORY_INSTRUCTIONS,
  "HLT",
  "NOP",
]);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Parse assembly source text into an array of structured lines.
 *
 * Processes the source line by line, stripping comments, and classifying
 * each line as a label, directive, or instruction. Returns both the
 * parsed lines and any errors encountered.
 *
 * Lines that are empty or contain only comments are silently skipped.
 *
 * @param source  The assembly source text (may contain \n or \r\n)
 * @returns       An object with `lines` (the parsed lines) and `errors`
 *
 * @example
 *   const { lines, errors } = parse("MOV R0, #1\nloop:\n  ADD R0, R0, #1\n");
 *   // lines = [
 *   //   { kind: "instruction", mnemonic: "MOV", ... },
 *   //   { kind: "label", name: "loop", ... },
 *   //   { kind: "instruction", mnemonic: "ADD", ... },
 *   // ]
 *   // errors = []
 */
export function parse(source: string): { lines: ParsedLine[]; errors: AssemblyError[] } {
  const rawLines = source.split(/\r?\n/);
  const parsedLines: ParsedLine[] = [];
  const errors: AssemblyError[] = [];

  for (let i = 0; i < rawLines.length; i++) {
    const lineNumber = i + 1;  // 1-based line numbers
    const rawLine = rawLines[i];

    // Strip comments — everything after ; or //
    const withoutComment = rawLine.replace(/;.*$/, "").replace(/\/\/.*$/, "");

    // Trim whitespace
    const trimmed = withoutComment.trim();

    // Skip empty lines
    if (trimmed === "") {
      continue;
    }

    // Try to parse this line
    const result = parseLine(trimmed, lineNumber);
    if (result.error) {
      errors.push(result.error);
    }
    if (result.parsed) {
      parsedLines.push(result.parsed);
    }
  }

  return { lines: parsedLines, errors };
}

// ---------------------------------------------------------------------------
// Line classification
// ---------------------------------------------------------------------------

/**
 * Parse a single trimmed, comment-free line of assembly.
 *
 * Classification rules (checked in order):
 *   1. If it ends with ":", it's a label
 *   2. If it starts with ".", it's a directive
 *   3. Otherwise, it's an instruction
 */
function parseLine(
  trimmed: string,
  lineNumber: number,
): { parsed?: ParsedLine; error?: AssemblyError } {
  // --- Label detection ---
  // Labels end with a colon: "loop:", "done:", "_start:"
  // A label may optionally have an instruction on the same line:
  //   "loop: MOV R0, #1" — label "loop" plus instruction MOV
  // But for simplicity, we treat labels as standalone lines.
  if (/^[a-zA-Z_]\w*:$/.test(trimmed)) {
    const name = trimmed.slice(0, -1);  // remove the trailing colon
    return { parsed: { kind: "label", lineNumber, name } };
  }

  // Handle label + instruction on same line (e.g., "loop: MOV R0, #1")
  const labelMatch = trimmed.match(/^([a-zA-Z_]\w*):\s+(.+)$/);
  if (labelMatch) {
    // For now, we only return the label — the instruction part needs to be
    // handled by re-parsing. This is a simplification; a production assembler
    // would handle both in one pass.
    const name = labelMatch[1];
    return { parsed: { kind: "label", lineNumber, name } };
  }

  // --- Directive detection ---
  // Directives start with a dot: ".global _start", ".data", ".text"
  if (trimmed.startsWith(".")) {
    const parts = trimmed.split(/\s+/);
    const name = parts[0];
    const args = parts.slice(1);
    return { parsed: { kind: "directive", lineNumber, name, args } };
  }

  // --- Instruction parsing ---
  return parseInstruction(trimmed, lineNumber);
}

// ---------------------------------------------------------------------------
// Instruction parsing
// ---------------------------------------------------------------------------

/**
 * Parse an instruction line like "MOV R0, #1" or "ADDNE R2, R0, R1".
 *
 * The parsing process:
 *   1. Split the line into mnemonic and operand parts
 *   2. Separate the condition suffix from the mnemonic (if any)
 *   3. Parse each operand (register, immediate, label, or memory)
 *
 * === Condition suffix extraction ===
 *
 * ARM instructions can have a condition suffix: "ADDNE", "BEQ", "MOVGT".
 * We need to carefully separate the base mnemonic from the condition.
 *
 * The algorithm:
 *   1. Check if the full token is a known mnemonic (e.g., "B", "BL", "HLT")
 *   2. If not, check if the last 2 characters are a condition code AND
 *      removing them gives a valid mnemonic
 *   3. Special case for branch: "BEQ" = B + EQ, "BLNE" = BL + NE
 *
 * @example
 *   "MOV" → mnemonic="MOV", condition="AL"
 *   "ADDNE" → mnemonic="ADD", condition="NE"
 *   "BEQ" → mnemonic="B", condition="EQ"
 *   "BLNE" → mnemonic="BL", condition="NE"
 */
function parseInstruction(
  text: string,
  lineNumber: number,
): { parsed?: ParsedLine; error?: AssemblyError } {
  // Split into first token (mnemonic+condition) and the rest (operands)
  const firstSpace = text.search(/\s/);
  const token = firstSpace === -1 ? text : text.substring(0, firstSpace);
  const rest = firstSpace === -1 ? "" : text.substring(firstSpace).trim();

  const upper = token.toUpperCase();

  // Extract mnemonic and condition
  const { mnemonic, condition } = splitMnemonicCondition(upper);

  if (mnemonic === null) {
    return {
      error: { line: lineNumber, message: `Unknown instruction: ${token}` },
    };
  }

  // Parse operands
  const operands = rest === "" ? [] : parseOperands(rest, mnemonic);

  return {
    parsed: {
      kind: "instruction",
      lineNumber,
      mnemonic,
      condition,
      operands,
    },
  };
}

/**
 * Split an uppercase token into mnemonic and condition code.
 *
 * Returns { mnemonic, condition } where condition defaults to "AL" (always)
 * if no condition suffix is found.
 *
 * @example
 *   splitMnemonicCondition("MOV")   → { mnemonic: "MOV", condition: "AL" }
 *   splitMnemonicCondition("ADDNE") → { mnemonic: "ADD", condition: "NE" }
 *   splitMnemonicCondition("BEQ")   → { mnemonic: "B", condition: "EQ" }
 *   splitMnemonicCondition("BLNE")  → { mnemonic: "BL", condition: "NE" }
 *   splitMnemonicCondition("FOOBAR") → { mnemonic: null, condition: "AL" }
 */
function splitMnemonicCondition(
  upper: string,
): { mnemonic: string | null; condition: string } {
  // Check if the whole token is a known mnemonic
  if (ALL_MNEMONICS.has(upper)) {
    return { mnemonic: upper, condition: "AL" };
  }

  // Try removing a 2-character condition suffix
  if (upper.length >= 3) {
    const suffix = upper.slice(-2);
    const base = upper.slice(0, -2);

    if (CONDITION_CODES.has(suffix) && ALL_MNEMONICS.has(base)) {
      return { mnemonic: base, condition: suffix };
    }
  }

  // Unknown instruction
  return { mnemonic: null, condition: "AL" };
}

/**
 * Parse the operand portion of an instruction line.
 *
 * Operands are comma-separated. Each operand is one of:
 *   - Register:   R0, R1, ..., R15, SP, LR, PC
 *   - Immediate:  #1, #42, #0xFF, #0b1010
 *   - Label:      loop, done, _start (a bare identifier)
 *   - Memory:     [R1], [R1, #4] (for LDR/STR)
 *
 * === Parsing strategy ===
 *
 * We split on commas, trim each part, and classify by the first character:
 *   - Starts with R/r and followed by digits → register
 *   - Is a register alias (SP, LR, PC) → register
 *   - Starts with # → immediate
 *   - Starts with [ → memory operand (split into base register and offset)
 *   - Otherwise → label reference
 *
 * @example
 *   "R0, #1"       → [register(0), immediate(1)]
 *   "R2, R0, R1"   → [register(2), register(0), register(1)]
 *   "loop"          → [label("loop")]
 *   "R0, [R1, #4]" → [register(0), register(1), immediate(4)]
 */
function parseOperands(text: string, mnemonic: string): Operand[] {
  const operands: Operand[] = [];

  // Handle memory operands specially — they contain commas inside brackets
  // Split carefully: commas inside [] don't separate operands
  const parts = splitOperands(text);

  for (const part of parts) {
    const trimmed = part.trim();
    if (trimmed === "") continue;

    const operand = parseOneOperand(trimmed, mnemonic);
    if (operand !== null) {
      // Memory operands can expand to multiple operands (base + offset)
      if (Array.isArray(operand)) {
        operands.push(...operand);
      } else {
        operands.push(operand);
      }
    }
  }

  return operands;
}

/**
 * Split an operand string on commas, but not inside brackets.
 *
 * "R0, [R1, #4]" → ["R0", "[R1, #4]"]
 * "R2, R0, R1"   → ["R2", "R0", "R1"]
 */
function splitOperands(text: string): string[] {
  const parts: string[] = [];
  let current = "";
  let bracketDepth = 0;

  for (const ch of text) {
    if (ch === "[") {
      bracketDepth++;
      current += ch;
    } else if (ch === "]") {
      bracketDepth--;
      current += ch;
    } else if (ch === "," && bracketDepth === 0) {
      parts.push(current);
      current = "";
    } else {
      current += ch;
    }
  }

  if (current.trim() !== "") {
    parts.push(current);
  }

  return parts;
}

/**
 * Parse a single operand token.
 *
 * @returns An Operand, an array of Operands (for memory), or null if invalid.
 */
function parseOneOperand(
  token: string,
  _mnemonic: string,
): Operand | Operand[] | null {
  const trimmed = token.trim();

  // --- Memory operand: [Rn] or [Rn, #offset] ---
  // Used by LDR and STR instructions.
  if (trimmed.startsWith("[")) {
    return parseMemoryOperand(trimmed);
  }

  // --- Register: R0-R15 or aliases (SP, LR, PC) ---
  const reg = parseRegister(trimmed);
  if (reg !== null) {
    return { type: "register", value: reg };
  }

  // --- Immediate: #decimal, #0xHex, #0bBinary ---
  if (trimmed.startsWith("#")) {
    const numStr = trimmed.slice(1).trim();
    const value = parseNumber(numStr);
    if (value !== null) {
      return { type: "immediate", value };
    }
    return null;  // invalid immediate
  }

  // --- Label reference: a bare identifier ---
  // Must start with a letter or underscore, followed by word characters
  if (/^[a-zA-Z_]\w*$/.test(trimmed)) {
    return { type: "label", value: trimmed };
  }

  return null;
}

/**
 * Parse a memory operand like [R1] or [R1, #4].
 *
 * Returns an array of operands: [base_register, offset_immediate].
 * If no offset is specified, the offset defaults to 0.
 *
 * @example
 *   "[R1]"      → [register(1), immediate(0)]
 *   "[R1, #4]"  → [register(1), immediate(4)]
 *   "[SP, #-8]" → [register(13), immediate(-8)]
 */
function parseMemoryOperand(token: string): Operand[] | null {
  // Strip the brackets
  const inner = token.replace(/^\[/, "").replace(/\]$/, "").trim();
  const parts = inner.split(",").map(s => s.trim());

  // Parse the base register
  const baseReg = parseRegister(parts[0]);
  if (baseReg === null) return null;

  const operands: Operand[] = [{ type: "register", value: baseReg }];

  // Parse the optional offset
  if (parts.length > 1) {
    const offsetStr = parts[1].trim();
    if (offsetStr.startsWith("#")) {
      const value = parseNumber(offsetStr.slice(1).trim());
      if (value !== null) {
        operands.push({ type: "immediate", value });
      }
    }
  } else {
    // No offset specified — default to 0
    operands.push({ type: "immediate", value: 0 });
  }

  return operands;
}

// ---------------------------------------------------------------------------
// Register parsing
// ---------------------------------------------------------------------------

/**
 * Parse a register name into its number (0-15).
 *
 * Accepts:
 *   - R0 through R15 (case-insensitive)
 *   - SP (= R13), LR (= R14), PC (= R15)
 *
 * Returns null if the token is not a valid register name.
 *
 * @example
 *   parseRegister("R0")  → 0
 *   parseRegister("R15") → 15
 *   parseRegister("SP")  → 13
 *   parseRegister("LR")  → 14
 *   parseRegister("PC")  → 15
 *   parseRegister("42")  → null
 */
export function parseRegister(token: string): number | null {
  const upper = token.toUpperCase().trim();

  // Check aliases first
  const alias = REGISTER_ALIASES.get(upper);
  if (alias !== undefined) return alias;

  // Check R0-R15
  const match = upper.match(/^R(\d{1,2})$/);
  if (match) {
    const num = parseInt(match[1], 10);
    if (num >= 0 && num <= 15) return num;
  }

  return null;
}

// ---------------------------------------------------------------------------
// Number parsing
// ---------------------------------------------------------------------------

/**
 * Parse a numeric literal in decimal, hexadecimal, or binary.
 *
 * Supports:
 *   - Decimal:     42, -7, 255
 *   - Hexadecimal: 0xFF, 0x1A
 *   - Binary:      0b1010, 0b11111111
 *   - Negative:    -1, -0xFF
 *
 * Returns null if the string is not a valid number.
 *
 * @example
 *   parseNumber("42")       → 42
 *   parseNumber("0xFF")     → 255
 *   parseNumber("0b1010")   → 10
 *   parseNumber("-1")       → -1
 *   parseNumber("hello")    → null
 */
export function parseNumber(s: string): number | null {
  const trimmed = s.trim();
  if (trimmed === "") return null;

  // Handle negative numbers
  const negative = trimmed.startsWith("-");
  const abs = negative ? trimmed.slice(1) : trimmed;

  let value: number;

  if (abs.startsWith("0x") || abs.startsWith("0X")) {
    // Hexadecimal
    value = parseInt(abs.slice(2), 16);
  } else if (abs.startsWith("0b") || abs.startsWith("0B")) {
    // Binary
    value = parseInt(abs.slice(2), 2);
  } else {
    // Decimal
    value = parseInt(abs, 10);
  }

  if (isNaN(value)) return null;
  return negative ? -value : value;
}
