/**
 * Types for the Assembler — Layer 5 of the computing stack.
 *
 * === Why separate types? ===
 *
 * The assembler's public API revolves around a few key data structures that
 * flow between the parser, encoder, and assembler stages. By defining them
 * in one place, we get:
 *
 *   1. A single source of truth — no circular imports between modules
 *   2. Clear documentation of the "contract" between stages
 *   3. Easy reference for anyone reading the codebase
 *
 * === Data flow through the assembler ===
 *
 *   Source text (string)
 *       │
 *       ▼
 *   ┌──────────┐
 *   │  Parser   │ ──→ ParsedLine[] (structured representation of each line)
 *   └──────────┘
 *       │
 *       ▼
 *   ┌──────────┐
 *   │ Pass 1    │ ──→ SymbolTable (label → address mapping)
 *   └──────────┘
 *       │
 *       ▼
 *   ┌──────────┐
 *   │ Pass 2    │ ──→ Machine code (Uint8Array) + source map
 *   └──────────┘
 *       │
 *       ▼
 *   AssemblyResult (machine code + symbol table + source map + errors)
 */

// ---------------------------------------------------------------------------
// Assembly errors
// ---------------------------------------------------------------------------
// When the assembler encounters something it can't handle — an unknown
// instruction, an invalid register name, a missing label — it records an
// error rather than crashing. This lets us report ALL errors in a single
// pass rather than stopping at the first one.
//
// This is a design choice borrowed from modern compilers: collect errors,
// report them all at once, and let the programmer fix multiple issues
// without re-running the assembler for each one.

/**
 * An error encountered during assembly.
 *
 * Each error records:
 *   - line: the 1-based line number in the source text
 *   - message: a human-readable description of what went wrong
 *
 * @example
 *   { line: 3, message: "Unknown instruction: FOOBAR" }
 *   { line: 7, message: "Invalid register: R99" }
 *   { line: 12, message: "Undefined label: loop_end" }
 */
export interface AssemblyError {
  readonly line: number;
  readonly message: string;
}

// ---------------------------------------------------------------------------
// Assembly result
// ---------------------------------------------------------------------------
// The assembler's output is a bundle of everything you need to run the
// program AND debug it. Machine code alone isn't enough — you need to know
// where labels ended up (symbol table) and which binary address corresponds
// to which source line (source map).

/**
 * The result of assembling a source file.
 *
 * Contains four pieces of information:
 *
 *   machineCode:  The assembled binary as a Uint8Array. Each ARM instruction
 *                 is 4 bytes, stored in little-endian order (the default
 *                 ARM byte order).
 *
 *   symbolTable:  A map from label names to their memory addresses. For
 *                 example, if "loop:" appears at the third instruction
 *                 (address 8), the symbol table has { "loop": 8 }.
 *
 *   sourceMap:    A map from memory addresses to source line numbers. This
 *                 is the inverse of "which instruction is on line N?" —
 *                 it tells you "address A came from line L." Debuggers use
 *                 this to highlight the current source line during execution.
 *
 *   errors:       Any errors encountered during assembly. If this array is
 *                 non-empty, the machineCode may be incomplete or incorrect.
 *
 * @example
 *   const result = assembler.assemble("MOV R0, #1\nHLT\n");
 *   // result.machineCode = Uint8Array of 8 bytes (2 instructions x 4 bytes)
 *   // result.symbolTable = {} (no labels)
 *   // result.sourceMap = { 0: 1, 4: 2 } (address 0 → line 1, address 4 → line 2)
 *   // result.errors = []
 */
export interface AssemblyResult {
  readonly machineCode: Uint8Array;
  readonly symbolTable: ReadonlyMap<string, number>;
  readonly sourceMap: ReadonlyMap<number, number>;
  readonly errors: readonly AssemblyError[];
}

// ---------------------------------------------------------------------------
// Parsed line types
// ---------------------------------------------------------------------------
// After the parser splits the source text into lines and tokenizes each one,
// it produces a ParsedLine for each non-empty, non-comment line. A ParsedLine
// can be one of three things:
//
//   1. A label definition ("loop:", "done:", "_start:")
//   2. An instruction ("MOV R0, #1", "ADD R2, R0, R1")
//   3. A directive (".data", ".text", ".global _start")
//
// Labels are special because they don't generate machine code — they just
// mark a position. Instructions generate exactly one 32-bit word of machine
// code. Directives control the assembler's behavior (we support a minimal
// set for now).

/**
 * The type of operand in an instruction.
 *
 * ARM instructions can have three kinds of operands:
 *
 *   register:   A register name like R0, R1, ..., R15, SP, LR, PC.
 *               The value is the register number (0-15).
 *
 *   immediate:  A literal number preceded by #, like #1, #42, #0xFF.
 *               The value is the number itself.
 *
 *   label:      A symbolic name that refers to a memory address.
 *               The value is the label name as a string (resolved in pass 2).
 *
 * @example
 *   MOV R0, #1       → operands: [register(0), immediate(1)]
 *   ADD R2, R0, R1   → operands: [register(2), register(0), register(1)]
 *   B loop           → operands: [label("loop")]
 */
export type OperandType = "register" | "immediate" | "label";

/**
 * A single operand in a parsed instruction.
 *
 * The `type` field tells you how to interpret `value`:
 *   - "register":  value is a number (0-15)
 *   - "immediate": value is a number
 *   - "label":     value is a string (the label name)
 */
export interface Operand {
  readonly type: OperandType;
  readonly value: number | string;
}

/**
 * A parsed line of assembly source.
 *
 * Each line is classified as one of:
 *   - "label":       A label definition (e.g., "loop:")
 *   - "instruction": An ARM instruction (e.g., "MOV R0, #1")
 *   - "directive":   An assembler directive (e.g., ".global _start")
 *
 * The `lineNumber` is 1-based (matching how text editors show line numbers).
 */
export interface ParsedLine {
  readonly kind: "label" | "instruction" | "directive";
  readonly lineNumber: number;

  /** For labels: the label name. For directives: the directive name. */
  readonly name?: string;

  /** For instructions: the mnemonic (e.g., "MOV", "ADD", "B"). */
  readonly mnemonic?: string;

  /** For instructions: the condition suffix (e.g., "EQ", "NE", "AL"). */
  readonly condition?: string;

  /** For instructions: the parsed operands. */
  readonly operands?: readonly Operand[];

  /** For directives: any arguments after the directive name. */
  readonly args?: readonly string[];
}

// ---------------------------------------------------------------------------
// Condition codes
// ---------------------------------------------------------------------------
// ARM's signature feature: every instruction can be conditionally executed.
// The condition is encoded in bits [31:28] of the instruction word. The CPU
// checks the condition flags (set by a previous CMP or S-suffixed instruction)
// and only executes the instruction if the condition is met.
//
// This reduces the need for branch instructions, which is good for pipeline
// performance (branches can cause pipeline flushes).
//
//   Condition  Binary  Meaning             Flag test
//   ─────────  ──────  ──────────────────  ─────────────────────
//   EQ         0000    Equal               Z == 1
//   NE         0001    Not equal           Z == 0
//   CS/HS      0010    Carry set / >=      C == 1
//   CC/LO      0011    Carry clear / <     C == 0
//   MI         0100    Minus (negative)    N == 1
//   PL         0101    Plus (positive)     N == 0
//   VS         0110    Overflow set        V == 1
//   VC         0111    Overflow clear      V == 0
//   HI         1000    Unsigned higher     C == 1 && Z == 0
//   LS         1001    Unsigned lower/same C == 0 || Z == 1
//   GE         1010    Signed >=           N == V
//   LT         1011    Signed <            N != V
//   GT         1100    Signed >            Z == 0 && N == V
//   LE         1101    Signed <=           Z == 1 || N != V
//   AL         1110    Always (default)    (always true)
//
// When no condition suffix is written, ARM assumes AL (always execute).

export const CONDITION_CODES: ReadonlyMap<string, number> = new Map([
  ["EQ", 0b0000],
  ["NE", 0b0001],
  ["CS", 0b0010],
  ["HS", 0b0010],
  ["CC", 0b0011],
  ["LO", 0b0011],
  ["MI", 0b0100],
  ["PL", 0b0101],
  ["VS", 0b0110],
  ["VC", 0b0111],
  ["HI", 0b1000],
  ["LS", 0b1001],
  ["GE", 0b1010],
  ["LT", 0b1011],
  ["GT", 0b1100],
  ["LE", 0b1101],
  ["AL", 0b1110],
]);

// ---------------------------------------------------------------------------
// Opcode constants
// ---------------------------------------------------------------------------
// ARM data processing opcodes occupy bits [24:21] of the instruction word.
// Each opcode specifies which arithmetic or logical operation the ALU performs.
//
// We support the subset needed for basic programs: moves, arithmetic, logic,
// comparison, and branches.

/**
 * ARM data processing opcodes (bits [24:21]).
 *
 *   Opcode  Binary  Mnemonic  Operation
 *   ──────  ──────  ────────  ──────────────────────────
 *   0000    0000    AND       Rd = Rn AND operand2
 *   0001    0001    EOR       Rd = Rn XOR operand2
 *   0010    0010    SUB       Rd = Rn - operand2
 *   0011    0011    RSB       Rd = operand2 - Rn
 *   0100    0100    ADD       Rd = Rn + operand2
 *   1000    1000    TST       Rn AND operand2 (flags only)
 *   1001    1001    TEQ       Rn XOR operand2 (flags only)
 *   1010    1010    CMP       Rn - operand2 (flags only)
 *   1011    1011    CMN       Rn + operand2 (flags only)
 *   1100    1100    ORR       Rd = Rn OR operand2
 *   1101    1101    MOV       Rd = operand2 (ignores Rn)
 *   1110    1110    BIC       Rd = Rn AND NOT operand2
 *   1111    1111    MVN       Rd = NOT operand2 (ignores Rn)
 */
export const OPCODES: ReadonlyMap<string, number> = new Map([
  ["AND", 0b0000],
  ["EOR", 0b0001],
  ["SUB", 0b0010],
  ["RSB", 0b0011],
  ["ADD", 0b0100],
  ["ADC", 0b0101],
  ["SBC", 0b0110],
  ["RSC", 0b0111],
  ["TST", 0b1000],
  ["TEQ", 0b1001],
  ["CMP", 0b1010],
  ["CMN", 0b1011],
  ["ORR", 0b1100],
  ["MOV", 0b1101],
  ["BIC", 0b1110],
  ["MVN", 0b1111],
]);

// ---------------------------------------------------------------------------
// Register aliases
// ---------------------------------------------------------------------------
// ARM has 16 registers (R0-R15), but three of them have special roles and
// alternate names:
//
//   R13 = SP (Stack Pointer)  — points to the top of the stack
//   R14 = LR (Link Register)  — holds the return address after a BL call
//   R15 = PC (Program Counter) — the address of the current instruction + 8
//
// We allow both names in assembly source: "MOV SP, #0" is the same as
// "MOV R13, #0".

export const REGISTER_ALIASES: ReadonlyMap<string, number> = new Map([
  ["SP", 13],
  ["LR", 14],
  ["PC", 15],
]);

// ---------------------------------------------------------------------------
// Instruction classification
// ---------------------------------------------------------------------------
// Different instructions use different encoding formats. We classify
// mnemonics by their format so the encoder knows which bit layout to use.

/** Instructions that set flags but don't write to Rd (no destination register). */
export const FLAG_ONLY_INSTRUCTIONS = new Set(["CMP", "CMN", "TST", "TEQ"]);

/** Instructions that ignore Rn (only use operand2 and Rd). */
export const NO_RN_INSTRUCTIONS = new Set(["MOV", "MVN"]);

/** Branch instructions (use a completely different encoding format). */
export const BRANCH_INSTRUCTIONS = new Set(["B", "BL"]);

/** Memory instructions (LDR/STR use the memory encoding format). */
export const MEMORY_INSTRUCTIONS = new Set(["LDR", "STR"]);

/** The halt instruction — our custom sentinel to stop the simulator. */
export const HLT_INSTRUCTION = 0xFFFFFFFF;
