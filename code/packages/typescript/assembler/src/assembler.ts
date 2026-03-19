/**
 * Assembler — the two-pass engine that turns assembly into machine code.
 *
 * === What is a two-pass assembler? ===
 *
 * A two-pass assembler reads the source code twice:
 *
 *   Pass 1 (Symbol Collection):
 *     Scan through every line, keeping track of the current memory address.
 *     When we encounter a label, record its address in the symbol table.
 *     When we encounter an instruction, advance the address by 4 bytes
 *     (each ARM instruction is exactly 32 bits = 4 bytes).
 *
 *   Pass 2 (Code Generation):
 *     Scan through every line again. This time, encode each instruction
 *     into binary. When an instruction references a label (like "B loop"),
 *     look up the label's address in the symbol table and compute the
 *     correct offset.
 *
 * === Why two passes? ===
 *
 * Forward references. Consider this program:
 *
 *     B done        ← references "done", which hasn't been defined yet!
 *     MOV R0, #1
 *   done:
 *     HLT
 *
 * On the first pass, when we see "B done", we don't know where "done" is.
 * But after scanning the whole file, we know done is at address 8.
 * On the second pass, we can encode the branch offset correctly.
 *
 * A one-pass assembler would need to "backpatch" — leave holes in the
 * output and fill them in later. Two passes are simpler to understand
 * and implement.
 *
 * === Architecture ===
 *
 *   ┌─────────────┐        ┌──────────────┐
 *   │ Source text  │ ──────→│   Parser      │──→ ParsedLine[]
 *   └─────────────┘        └──────────────┘
 *                                 │
 *                    ┌────────────┴────────────┐
 *                    ▼                         ▼
 *              ┌──────────┐              ┌──────────┐
 *              │  Pass 1   │              │  Pass 2   │
 *              │  Labels   │──→SymbolTable│  Encode   │──→ Machine code
 *              └──────────┘              └──────────┘
 *                                              │
 *                                              ▼
 *                                       AssemblyResult
 */

import type {
  AssemblyError,
  AssemblyResult,
  Operand,
  ParsedLine,
} from "./types.js";
import {
  CONDITION_CODES,
  OPCODES,
  FLAG_ONLY_INSTRUCTIONS,
  NO_RN_INSTRUCTIONS,
  BRANCH_INSTRUCTIONS,
  MEMORY_INSTRUCTIONS,
  HLT_INSTRUCTION,
} from "./types.js";
import { parse } from "./parser.js";
import {
  encodeDataProcessing,
  encodeBranch,
  encodeMemory,
  encodeImmediate,
} from "./encoder.js";

// ---------------------------------------------------------------------------
// The Assembler class
// ---------------------------------------------------------------------------

/**
 * A two-pass ARM assembler.
 *
 * Usage:
 *   const assembler = new Assembler();
 *   const result = assembler.assemble("MOV R0, #42\nHLT\n");
 *
 *   if (result.errors.length > 0) {
 *     console.error("Assembly errors:", result.errors);
 *   } else {
 *     console.log("Machine code:", result.machineCode);
 *     console.log("Symbol table:", result.symbolTable);
 *   }
 *
 * The assembler is stateless — each call to assemble() starts fresh.
 * You can reuse the same Assembler instance for multiple programs.
 */
export class Assembler {
  /**
   * Assemble source text into machine code.
   *
   * This is the main entry point. It:
   *   1. Parses the source into structured lines
   *   2. Runs pass 1 to collect labels
   *   3. Runs pass 2 to encode instructions
   *   4. Returns everything bundled as an AssemblyResult
   *
   * @param source  The assembly source text
   * @returns       The assembly result (machine code, symbols, source map, errors)
   */
  assemble(source: string): AssemblyResult {
    // --- Step 1: Parse ---
    // The parser turns raw text into structured ParsedLine objects.
    // It also catches syntax-level errors (unknown tokens, etc.).
    const { lines, errors: parseErrors } = parse(source);

    // --- Step 2: Pass 1 — Collect labels ---
    // Walk through the parsed lines, tracking the current address.
    // Labels don't advance the address; instructions and data do.
    const { symbolTable, errors: pass1Errors } = this.pass1(lines);

    // --- Step 3: Pass 2 — Encode instructions ---
    // Walk through again, this time encoding each instruction to binary.
    // Label references are resolved using the symbol table from pass 1.
    const { machineCode, sourceMap, errors: pass2Errors } = this.pass2(
      lines,
      symbolTable,
    );

    // Combine all errors
    const allErrors = [...parseErrors, ...pass1Errors, ...pass2Errors];

    return {
      machineCode,
      symbolTable,
      sourceMap,
      errors: allErrors,
    };
  }

  // -------------------------------------------------------------------------
  // Pass 1: Symbol Collection
  // -------------------------------------------------------------------------

  /**
   * First pass — scan for labels and record their addresses.
   *
   * === How addresses work ===
   *
   * ARM instructions are always 4 bytes (32 bits). So the address of the
   * Nth instruction is N * 4. Labels mark addresses between instructions:
   *
   *   Address  Source
   *   ──────── ──────────────────
   *   0x0000   MOV R0, #1         ← instruction at address 0
   *   0x0004   MOV R1, #2         ← instruction at address 4
   *            loop:               ← label "loop" = address 8
   *   0x0008   ADD R2, R0, R1     ← instruction at address 8
   *   0x000C   HLT                ← instruction at address 12
   *
   * Notice that "loop:" points to address 8 — the address of the NEXT
   * instruction after the label, not the label itself (labels don't
   * occupy space in memory).
   *
   * @param lines  The parsed lines from the parser
   * @returns      The symbol table (label → address) and any errors
   */
  private pass1(
    lines: readonly ParsedLine[],
  ): { symbolTable: Map<string, number>; errors: AssemblyError[] } {
    const symbolTable = new Map<string, number>();
    const errors: AssemblyError[] = [];
    let address = 0;

    for (const line of lines) {
      if (line.kind === "label") {
        // Record the label's address
        const name = line.name!;
        if (symbolTable.has(name)) {
          errors.push({
            line: line.lineNumber,
            message: `Duplicate label: ${name} (previously defined)`,
          });
        } else {
          symbolTable.set(name, address);
        }
        // Labels don't advance the address — they just mark a position
      } else if (line.kind === "instruction") {
        // Each instruction is 4 bytes (32 bits)
        address += 4;
      } else if (line.kind === "directive") {
        // Directives we handle: .data, .text, .global (none change address for now)
        // A future extension might handle .word, .asciz, etc.
      }
    }

    return { symbolTable, errors };
  }

  // -------------------------------------------------------------------------
  // Pass 2: Code Generation
  // -------------------------------------------------------------------------

  /**
   * Second pass — encode each instruction into binary machine code.
   *
   * For each instruction line, we:
   *   1. Determine the encoding format (data processing, branch, memory, halt)
   *   2. Resolve any label references using the symbol table
   *   3. Encode the instruction to a 32-bit word
   *   4. Write the bytes to the output buffer (little-endian)
   *   5. Record the source mapping (address → line number)
   *
   * @param lines        The parsed lines
   * @param symbolTable  The symbol table from pass 1
   * @returns            Machine code bytes, source map, and any errors
   */
  private pass2(
    lines: readonly ParsedLine[],
    symbolTable: ReadonlyMap<string, number>,
  ): {
    machineCode: Uint8Array;
    sourceMap: Map<number, number>;
    errors: AssemblyError[];
  } {
    const instructions: number[] = [];
    const sourceMap = new Map<number, number>();
    const errors: AssemblyError[] = [];
    let address = 0;

    for (const line of lines) {
      if (line.kind !== "instruction") continue;

      // Record source mapping
      sourceMap.set(address, line.lineNumber);

      // Encode the instruction
      const result = this.encodeInstruction(line, address, symbolTable);

      if (result.error) {
        errors.push(result.error);
        // Emit a NOP (MOV R0, R0) as a placeholder so addresses stay correct
        instructions.push(0xE1A00000);
      } else {
        instructions.push(result.word!);
      }

      address += 4;
    }

    // Convert instruction words to bytes (little-endian)
    const machineCode = new Uint8Array(instructions.length * 4);
    const view = new DataView(machineCode.buffer);
    for (let i = 0; i < instructions.length; i++) {
      view.setUint32(i * 4, instructions[i] >>> 0, true /* littleEndian */);
    }

    return { machineCode, sourceMap, errors };
  }

  // -------------------------------------------------------------------------
  // Instruction encoding dispatch
  // -------------------------------------------------------------------------

  /**
   * Encode a single parsed instruction into a 32-bit word.
   *
   * This is the central dispatch function. It examines the mnemonic and
   * routes to the appropriate encoder:
   *
   *   HLT         → halt sentinel (0xFFFFFFFF)
   *   NOP         → MOV R0, R0 (0xE1A00000)
   *   B, BL       → branch encoding
   *   LDR, STR    → memory encoding
   *   Everything  → data processing encoding
   *
   * @param line         The parsed instruction
   * @param address      The current memory address (for branch offset calculation)
   * @param symbolTable  The symbol table (for resolving label references)
   */
  private encodeInstruction(
    line: ParsedLine,
    address: number,
    symbolTable: ReadonlyMap<string, number>,
  ): { word?: number; error?: AssemblyError } {
    const mnemonic = line.mnemonic!;
    const condition = line.condition ?? "AL";
    const cond = CONDITION_CODES.get(condition);

    if (cond === undefined) {
      return {
        error: {
          line: line.lineNumber,
          message: `Unknown condition code: ${condition}`,
        },
      };
    }

    // --- HLT (Halt) ---
    if (mnemonic === "HLT") {
      return { word: HLT_INSTRUCTION };
    }

    // --- NOP (No Operation) ---
    // Encoded as MOV R0, R0 — a harmless instruction that does nothing.
    // This is the standard ARM NOP encoding.
    if (mnemonic === "NOP") {
      return { word: 0xE1A00000 };
    }

    // --- Branch instructions (B, BL) ---
    if (BRANCH_INSTRUCTIONS.has(mnemonic)) {
      return this.encodeBranchInstruction(line, address, symbolTable, cond);
    }

    // --- Memory instructions (LDR, STR) ---
    if (MEMORY_INSTRUCTIONS.has(mnemonic)) {
      return this.encodeMemoryInstruction(line, cond);
    }

    // --- Data processing instructions ---
    return this.encodeDataProcessingInstruction(line, cond);
  }

  // -------------------------------------------------------------------------
  // Branch instruction encoding
  // -------------------------------------------------------------------------

  /**
   * Encode a branch instruction (B or BL).
   *
   * === Branch offset calculation ===
   *
   * ARM branch offsets are relative to (PC + 8) because of the pipeline.
   * When the CPU is executing the instruction at address A, the PC has
   * already been incremented to A + 8 (two instructions ahead).
   *
   * The offset in the instruction is in words (multiples of 4 bytes):
   *
   *   target_address = current_address + 8 + (offset * 4)
   *   offset = (target_address - current_address - 8) / 4
   *
   * Example: Branch from address 0 to address 12
   *   offset = (12 - 0 - 8) / 4 = 1
   *
   * Example: Branch from address 16 to address 0 (backwards)
   *   offset = (0 - 16 - 8) / 4 = -6
   */
  private encodeBranchInstruction(
    line: ParsedLine,
    address: number,
    symbolTable: ReadonlyMap<string, number>,
    cond: number,
  ): { word?: number; error?: AssemblyError } {
    const mnemonic = line.mnemonic!;
    const operands = line.operands ?? [];
    const link = mnemonic === "BL";

    if (operands.length !== 1) {
      return {
        error: {
          line: line.lineNumber,
          message: `${mnemonic} requires exactly 1 operand (label), got ${operands.length}`,
        },
      };
    }

    const operand = operands[0];
    let targetAddress: number;

    if (operand.type === "label") {
      // Resolve the label from the symbol table
      const labelName = operand.value as string;
      const resolved = symbolTable.get(labelName);
      if (resolved === undefined) {
        return {
          error: {
            line: line.lineNumber,
            message: `Undefined label: ${labelName}`,
          },
        };
      }
      targetAddress = resolved;
    } else if (operand.type === "immediate") {
      // Direct address (less common, but supported)
      targetAddress = operand.value as number;
    } else {
      return {
        error: {
          line: line.lineNumber,
          message: `${mnemonic} operand must be a label or immediate, got register`,
        },
      };
    }

    // Calculate the word offset: (target - current - 8) / 4
    const byteOffset = targetAddress - address - 8;
    const wordOffset = byteOffset >> 2;  // divide by 4 (shift right by 2)

    return { word: encodeBranch(cond, wordOffset, link) };
  }

  // -------------------------------------------------------------------------
  // Memory instruction encoding
  // -------------------------------------------------------------------------

  /**
   * Encode a memory instruction (LDR or STR).
   *
   * Supported forms:
   *   LDR Rd, [Rn]           → load from address in Rn
   *   LDR Rd, [Rn, #offset]  → load from Rn + offset
   *   STR Rd, [Rn]           → store to address in Rn
   *   STR Rd, [Rn, #offset]  → store to Rn + offset
   *
   * The parser expands memory operands into separate register and
   * immediate operands, so "LDR R0, [R1, #4]" arrives as:
   *   operands: [register(0), register(1), immediate(4)]
   */
  private encodeMemoryInstruction(
    line: ParsedLine,
    cond: number,
  ): { word?: number; error?: AssemblyError } {
    const mnemonic = line.mnemonic!;
    const operands = line.operands ?? [];
    const load = mnemonic === "LDR";

    // Expect: Rd, Rn, [offset]
    if (operands.length < 2) {
      return {
        error: {
          line: line.lineNumber,
          message: `${mnemonic} requires at least 2 operands (Rd, [Rn]), got ${operands.length}`,
        },
      };
    }

    const rd = this.expectRegister(operands[0], line.lineNumber, "Rd");
    if (rd.error) return rd;

    const rn = this.expectRegister(operands[1], line.lineNumber, "Rn");
    if (rn.error) return rn;

    // Offset is optional, defaults to 0
    let offset = 0;
    if (operands.length > 2 && operands[2].type === "immediate") {
      offset = operands[2].value as number;
    }

    return { word: encodeMemory(cond, load, rn.value!, rd.value!, offset) };
  }

  // -------------------------------------------------------------------------
  // Data processing instruction encoding
  // -------------------------------------------------------------------------

  /**
   * Encode a data processing instruction (ADD, SUB, MOV, CMP, etc.).
   *
   * === Instruction forms ===
   *
   * Data processing instructions come in several forms depending on
   * the mnemonic:
   *
   * Three-register form (ADD, SUB, AND, ORR, EOR, etc.):
   *   ADD Rd, Rn, Rm       → Rd = Rn op Rm
   *   ADD Rd, Rn, #imm     → Rd = Rn op imm
   *
   * Two-operand form (MOV, MVN):
   *   MOV Rd, Rm            → Rd = Rm
   *   MOV Rd, #imm          → Rd = imm
   *
   * Compare form (CMP, CMN, TST, TEQ) — sets flags, no destination:
   *   CMP Rn, Rm            → flags = Rn - Rm
   *   CMP Rn, #imm          → flags = Rn - imm
   *
   * The S suffix makes any instruction set flags:
   *   ADDS R0, R1, R2       → R0 = R1 + R2, update flags
   */
  private encodeDataProcessingInstruction(
    line: ParsedLine,
    cond: number,
  ): { word?: number; error?: AssemblyError } {
    const rawMnemonic = line.mnemonic!;
    const operands = line.operands ?? [];

    // Check for S suffix (set flags)
    let mnemonic = rawMnemonic;
    let setFlags = false;
    if (rawMnemonic.endsWith("S") && !OPCODES.has(rawMnemonic)) {
      mnemonic = rawMnemonic.slice(0, -1);
      setFlags = true;
    }

    const opcode = OPCODES.get(mnemonic);
    if (opcode === undefined) {
      return {
        error: {
          line: line.lineNumber,
          message: `Unknown data processing instruction: ${rawMnemonic}`,
        },
      };
    }

    // Flag-only instructions (CMP, CMN, TST, TEQ) always set flags
    if (FLAG_ONLY_INSTRUCTIONS.has(mnemonic)) {
      setFlags = true;
    }

    // Dispatch based on instruction form
    if (FLAG_ONLY_INSTRUCTIONS.has(mnemonic)) {
      return this.encodeFlagOnly(line, cond, opcode, operands);
    } else if (NO_RN_INSTRUCTIONS.has(mnemonic)) {
      return this.encodeNoRn(line, cond, opcode, setFlags, operands);
    } else {
      return this.encodeThreeOperand(line, cond, opcode, setFlags, operands);
    }
  }

  /**
   * Encode a flag-only instruction (CMP, CMN, TST, TEQ).
   *
   * Form: CMP Rn, operand2
   *   - No destination register (Rd is set to 0)
   *   - S bit is always 1 (these instructions exist to set flags)
   */
  private encodeFlagOnly(
    line: ParsedLine,
    cond: number,
    opcode: number,
    operands: readonly Operand[],
  ): { word?: number; error?: AssemblyError } {
    if (operands.length !== 2) {
      return {
        error: {
          line: line.lineNumber,
          message: `${line.mnemonic} requires 2 operands (Rn, operand2), got ${operands.length}`,
        },
      };
    }

    const rn = this.expectRegister(operands[0], line.lineNumber, "Rn");
    if (rn.error) return rn;

    const op2 = this.encodeOperand2(operands[1], line.lineNumber);
    if (op2.error) return op2;

    const word = encodeDataProcessing(
      cond,
      opcode,
      true,  // S=1 always for flag-only instructions
      rn.value!,
      0,     // Rd=0 (no destination)
      op2.value!,
      op2.immediate!,
    );

    return { word };
  }

  /**
   * Encode a no-Rn instruction (MOV, MVN).
   *
   * Form: MOV Rd, operand2
   *   - No first source register (Rn is set to 0)
   */
  private encodeNoRn(
    line: ParsedLine,
    cond: number,
    opcode: number,
    setFlags: boolean,
    operands: readonly Operand[],
  ): { word?: number; error?: AssemblyError } {
    if (operands.length !== 2) {
      return {
        error: {
          line: line.lineNumber,
          message: `${line.mnemonic} requires 2 operands (Rd, operand2), got ${operands.length}`,
        },
      };
    }

    const rd = this.expectRegister(operands[0], line.lineNumber, "Rd");
    if (rd.error) return rd;

    const op2 = this.encodeOperand2(operands[1], line.lineNumber);
    if (op2.error) return op2;

    const word = encodeDataProcessing(
      cond,
      opcode,
      setFlags,
      0,       // Rn=0 (ignored by MOV/MVN)
      rd.value!,
      op2.value!,
      op2.immediate!,
    );

    return { word };
  }

  /**
   * Encode a three-operand instruction (ADD, SUB, AND, ORR, etc.).
   *
   * Form: ADD Rd, Rn, operand2
   */
  private encodeThreeOperand(
    line: ParsedLine,
    cond: number,
    opcode: number,
    setFlags: boolean,
    operands: readonly Operand[],
  ): { word?: number; error?: AssemblyError } {
    if (operands.length !== 3) {
      return {
        error: {
          line: line.lineNumber,
          message: `${line.mnemonic} requires 3 operands (Rd, Rn, operand2), got ${operands.length}`,
        },
      };
    }

    const rd = this.expectRegister(operands[0], line.lineNumber, "Rd");
    if (rd.error) return rd;

    const rn = this.expectRegister(operands[1], line.lineNumber, "Rn");
    if (rn.error) return rn;

    const op2 = this.encodeOperand2(operands[2], line.lineNumber);
    if (op2.error) return op2;

    const word = encodeDataProcessing(
      cond,
      opcode,
      setFlags,
      rn.value!,
      rd.value!,
      op2.value!,
      op2.immediate!,
    );

    return { word };
  }

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

  /**
   * Extract a register number from an operand, or return an error.
   */
  private expectRegister(
    operand: Operand,
    lineNumber: number,
    name: string,
  ): { value?: number; error?: AssemblyError } {
    if (operand.type !== "register") {
      return {
        error: {
          line: lineNumber,
          message: `Expected register for ${name}, got ${operand.type}: ${operand.value}`,
        },
      };
    }
    return { value: operand.value as number };
  }

  /**
   * Encode the second operand (register or immediate) into the 12-bit
   * operand2 field.
   *
   * If the operand is a register, the 12-bit field is just the register
   * number in the lowest 4 bits.
   *
   * If the operand is an immediate, we try to encode it using ARM's
   * 8-bit-with-rotation scheme.
   */
  private encodeOperand2(
    operand: Operand,
    lineNumber: number,
  ): { value?: number; immediate?: boolean; error?: AssemblyError } {
    if (operand.type === "register") {
      return { value: operand.value as number, immediate: false };
    }

    if (operand.type === "immediate") {
      const imm = operand.value as number;
      const encoded = encodeImmediate(imm);
      if (encoded === null) {
        return {
          error: {
            line: lineNumber,
            message: `Immediate value ${imm} cannot be encoded in ARM format (must be representable as 8-bit value with even rotation)`,
          },
        };
      }
      return { value: encoded, immediate: true };
    }

    return {
      error: {
        line: lineNumber,
        message: `Expected register or immediate, got ${operand.type}: ${operand.value}`,
      },
    };
  }
}

// ---------------------------------------------------------------------------
// Convenience function
// ---------------------------------------------------------------------------

/**
 * Assemble source text into machine code (convenience wrapper).
 *
 * This creates a temporary Assembler instance and calls assemble().
 * Use the Assembler class directly if you need to assemble multiple
 * programs (though the assembler is stateless, so there's no performance
 * difference).
 *
 * @example
 *   const result = assemble("MOV R0, #42\nADD R1, R0, #1\nHLT\n");
 *   console.log(result.machineCode);  // Uint8Array of 12 bytes
 */
export function assemble(source: string): AssemblyResult {
  return new Assembler().assemble(source);
}
