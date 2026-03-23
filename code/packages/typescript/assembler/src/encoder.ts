/**
 * Instruction Encoder — translates structured instruction data into binary.
 *
 * === How ARM instructions are encoded ===
 *
 * Every ARM instruction is exactly 32 bits (4 bytes). The bits are divided
 * into fields whose layout depends on the instruction type. There are three
 * main formats we handle:
 *
 *   1. Data processing (ADD, SUB, MOV, CMP, AND, ORR, etc.)
 *   2. Branch (B, BL)
 *   3. Memory access (LDR, STR)
 *
 * === Data processing format ===
 *
 *   31  28 27 26 25 24  21 20 19  16 15  12 11           0
 *   ┌─────┬─────┬──┬──────┬──┬──────┬──────┬─────────────┐
 *   │Cond │ 00  │I │Opcode│S │  Rn  │  Rd  │  Operand2   │
 *   └─────┴─────┴──┴──────┴──┴──────┴──────┴─────────────┘
 *
 *   Cond (4 bits):     Condition code (when to execute)
 *   00 (2 bits):       Fixed bits identifying this as data processing
 *   I (1 bit):         Immediate flag (1 = operand2 is a constant, 0 = register)
 *   Opcode (4 bits):   Which operation (ADD=0100, MOV=1101, etc.)
 *   S (1 bit):         Set flags? (1 = update N, Z, C, V flags)
 *   Rn (4 bits):       First source register (ignored by MOV/MVN)
 *   Rd (4 bits):       Destination register
 *   Operand2 (12 bits): Second operand — either immediate or register
 *
 * When I=1 (immediate mode), operand2 is split as:
 *   [rotate(4) | imm8(8)]
 *   Actual value = imm8 rotated right by (rotate * 2) positions
 *
 * When I=0 (register mode), operand2's lowest 4 bits are the register number.
 *
 * === Branch format ===
 *
 *   31  28 27 25 24 23                    0
 *   ┌─────┬─────┬──┬───────────────────────┐
 *   │Cond │ 101 │L │      Offset (24)      │
 *   └─────┴─────┴──┴───────────────────────┘
 *
 *   Cond (4 bits):   Condition code
 *   101 (3 bits):    Fixed bits identifying this as a branch
 *   L (1 bit):       Link bit (1 = BL, saves return address in LR)
 *   Offset (24 bits): Signed offset in words (multiplied by 4, relative to PC+8)
 *
 * The offset calculation is tricky because of ARM's pipeline:
 *   target_address = PC + 8 + (offset * 4)
 *   So: offset = (target_address - current_address - 8) / 4
 *
 * === Memory access format ===
 *
 *   31  28 27 26 25 24 23 22 21 20 19  16 15  12 11           0
 *   ┌─────┬─────┬──┬──┬──┬──┬──┬──┬──────┬──────┬─────────────┐
 *   │Cond │ 01  │I │P │U │B │W │L │  Rn  │  Rd  │   Offset    │
 *   └─────┴─────┴──┴──┴──┴──┴──┴──┴──────┴──────┴─────────────┘
 *
 *   L (1 bit): Load/Store (1 = LDR, 0 = STR)
 *   Rn: Base register
 *   Rd: Source/destination register
 *   Offset: 12-bit immediate offset
 *
 * For our simplified assembler, we use pre-indexed mode with immediate offset:
 *   LDR Rd, [Rn, #offset]  — load from memory at Rn + offset
 *   STR Rd, [Rn, #offset]  — store to memory at Rn + offset
 */

import {
  CONDITION_CODES,
  OPCODES,
  FLAG_ONLY_INSTRUCTIONS,
  NO_RN_INSTRUCTIONS,
  HLT_INSTRUCTION,
} from "./types.js";

// ---------------------------------------------------------------------------
// Data processing encoder
// ---------------------------------------------------------------------------

/**
 * Encode a data processing instruction into a 32-bit ARM instruction word.
 *
 * This is the workhorse encoder — it handles ADD, SUB, MOV, CMP, AND, ORR,
 * and all other ALU operations.
 *
 * Parameters:
 *   cond:      Condition code (4 bits, e.g., 0b1110 for AL = always)
 *   opcode:    Operation code (4 bits, e.g., 0b0100 for ADD)
 *   s:         Set flags (true = update NZCV flags after operation)
 *   rn:        First source register (4 bits, 0-15)
 *   rd:        Destination register (4 bits, 0-15)
 *   operand2:  Second operand value (12 bits)
 *   immediate: Is operand2 an immediate value? (sets the I bit)
 *
 * === Encoding example: MOV R0, #1 ===
 *
 *   cond=0b1110 (AL), opcode=0b1101 (MOV), s=false, rn=0, rd=0, operand2=1, immediate=true
 *
 *   Bit layout:
 *     1110 00 1 1101 0 0000 0000 000000000001
 *     ^^^^ ^^ ^ ^^^^ ^ ^^^^ ^^^^ ^^^^^^^^^^^^
 *     cond    I  MOV S  Rn   Rd    operand2
 *
 *   Result: 0xE3A00001
 *
 * === Encoding example: ADD R2, R0, R1 ===
 *
 *   cond=0b1110 (AL), opcode=0b0100 (ADD), s=false, rn=0, rd=2, operand2=1, immediate=false
 *
 *   Bit layout:
 *     1110 00 0 0100 0 0000 0010 000000000001
 *     ^^^^ ^^ ^ ^^^^ ^ ^^^^ ^^^^ ^^^^^^^^^^^^
 *     cond    I  ADD S  Rn   Rd    Rm=R1
 *
 *   Result: 0xE0802001
 *
 * @example
 *   // MOV R0, #1
 *   encodeDataProcessing(0b1110, 0b1101, false, 0, 0, 1, true)
 *   // => 0xE3A00001
 *
 *   // ADD R2, R0, R1
 *   encodeDataProcessing(0b1110, 0b0100, false, 0, 2, 1, false)
 *   // => 0xE0802001
 */
export function encodeDataProcessing(
  cond: number,
  opcode: number,
  s: boolean,
  rn: number,
  rd: number,
  operand2: number,
  immediate: boolean,
): number {
  // Each field is masked to its bit width and shifted to the correct position
  //
  //   Bits [31:28] = condition code  (4 bits)
  //   Bits [27:26] = 00              (data processing identifier)
  //   Bit  [25]    = I               (immediate flag)
  //   Bits [24:21] = opcode          (4 bits)
  //   Bit  [20]    = S               (set flags)
  //   Bits [19:16] = Rn              (4 bits)
  //   Bits [15:12] = Rd              (4 bits)
  //   Bits [11:0]  = operand2        (12 bits)

  // The >>> 0 converts to unsigned 32-bit. Without it, shifting the
  // condition code (e.g., 0xE) left by 28 overflows signed 32-bit range
  // and produces a negative number.
  return (
    ((cond & 0xF) << 28) |
    (0b00 << 26) |
    ((immediate ? 1 : 0) << 25) |
    ((opcode & 0xF) << 21) |
    ((s ? 1 : 0) << 20) |
    ((rn & 0xF) << 16) |
    ((rd & 0xF) << 12) |
    (operand2 & 0xFFF)
  ) >>> 0;
}

// ---------------------------------------------------------------------------
// Branch encoder
// ---------------------------------------------------------------------------

/**
 * Encode a branch instruction into a 32-bit ARM instruction word.
 *
 * === How branch offsets work in ARM ===
 *
 * ARM's branch offset is calculated relative to PC+8 (because of the
 * pipeline: when executing an instruction at address A, the PC has
 * already been incremented to A+8). The offset is in units of words
 * (4 bytes), not bytes, and is sign-extended to 32 bits.
 *
 *   target = PC + 8 + (offset * 4)
 *
 * Rearranging to find the offset for encoding:
 *   offset = (target - current_address - 8) / 4
 *
 * The offset is a 24-bit signed integer, giving a range of +/- 32MB.
 *
 * === Encoding format ===
 *
 *   31  28 27 25 24 23                    0
 *   ┌─────┬─────┬──┬───────────────────────┐
 *   │Cond │ 101 │L │    Offset (24 bits)   │
 *   └─────┴─────┴──┴───────────────────────┘
 *
 * @param cond    Condition code (4 bits)
 * @param offset  Signed word offset (24 bits, already computed)
 * @param link    Whether this is BL (Branch with Link) — saves return address
 *
 * @example
 *   // B to an offset of -2 words (branch back 2 instructions)
 *   encodeBranch(0b1110, -2, false)
 *
 *   // BL (function call) to offset +10
 *   encodeBranch(0b1110, 10, true)
 */
export function encodeBranch(
  cond: number,
  offset: number,
  link: boolean = false,
): number {
  // The offset must be masked to 24 bits. For negative offsets, JavaScript
  // represents them as negative numbers, so we AND with 0xFFFFFF to get
  // the two's complement 24-bit representation.
  //
  // Example: offset = -2
  //   In 32-bit two's complement: 0xFFFFFFFE
  //   Masked to 24 bits: 0xFFFFFE

  return (
    ((cond & 0xF) << 28) |
    (0b101 << 25) |
    ((link ? 1 : 0) << 24) |
    (offset & 0x00FFFFFF)
  ) >>> 0;
}

// ---------------------------------------------------------------------------
// Memory access encoder
// ---------------------------------------------------------------------------

/**
 * Encode a memory access instruction (LDR or STR) into a 32-bit word.
 *
 * === Memory access in ARM ===
 *
 * LDR (Load Register) reads a 32-bit value from memory into a register.
 * STR (Store Register) writes a register's value to memory.
 *
 * The simplest addressing mode is "pre-indexed with immediate offset":
 *   LDR Rd, [Rn, #offset]   — load from address (Rn + offset)
 *   STR Rd, [Rn, #offset]   — store to address (Rn + offset)
 *
 * === Encoding format ===
 *
 *   31  28 27 26 25 24 23 22 21 20 19  16 15  12 11           0
 *   ┌─────┬─────┬──┬──┬──┬──┬──┬──┬──────┬──────┬─────────────┐
 *   │Cond │ 01  │I │P │U │B │W │L │  Rn  │  Rd  │   Offset    │
 *   └─────┴─────┴──┴──┴──┴──┴──┴──┴──────┴──────┴─────────────┘
 *
 *   I=0:  Offset is an immediate (12 bits)
 *   P=1:  Pre-indexed (add offset before access)
 *   U:    Up (1) or Down (0) — add or subtract offset
 *   B=0:  Word access (32-bit)
 *   W=0:  No write-back (don't update Rn)
 *   L:    Load (1) or Store (0)
 *
 * @param cond    Condition code (4 bits)
 * @param load    true for LDR, false for STR
 * @param rn      Base register (4 bits)
 * @param rd      Source/destination register (4 bits)
 * @param offset  Signed 12-bit immediate offset
 *
 * @example
 *   // LDR R0, [R1, #4] — load from address R1+4
 *   encodeMemory(0b1110, true, 1, 0, 4)
 *
 *   // STR R2, [R3, #0] — store R2 to address R3
 *   encodeMemory(0b1110, false, 3, 2, 0)
 */
export function encodeMemory(
  cond: number,
  load: boolean,
  rn: number,
  rd: number,
  offset: number,
): number {
  // Determine sign of offset — the U bit indicates whether to add or subtract
  const up = offset >= 0 ? 1 : 0;
  const absOffset = Math.abs(offset) & 0xFFF;

  // Fixed bits for our simplified encoding:
  //   I=0 (immediate offset), P=1 (pre-indexed), B=0 (word), W=0 (no write-back)

  return (
    ((cond & 0xF) << 28) |
    (0b01 << 26) |      // memory access identifier
    (0 << 25) |          // I=0: immediate offset
    (1 << 24) |          // P=1: pre-indexed
    (up << 23) |         // U: up/down
    (0 << 22) |          // B=0: word access
    (0 << 21) |          // W=0: no write-back
    ((load ? 1 : 0) << 20) |  // L: load/store
    ((rn & 0xF) << 16) |
    ((rd & 0xF) << 12) |
    absOffset
  ) >>> 0;
}

// ---------------------------------------------------------------------------
// Convenience encoders
// ---------------------------------------------------------------------------
// These functions mirror the Python ARM simulator's encode_* helpers.
// They provide a simple interface for encoding common instructions
// without needing to know the bit-level details.

/**
 * Encode: MOV Rd, #imm → 32-bit instruction.
 *
 * MOV (Move) loads an immediate value into a register. It's the most
 * basic way to get a value into a register.
 *
 * Example:
 *   encodeMovImm(0, 1)  → MOV R0, #1  → 0xE3A00001
 *   encodeMovImm(1, 2)  → MOV R1, #2  → 0xE3A01002
 *
 * Note: The immediate must fit in 8 bits (0-255) with no rotation for
 * this simple encoder. ARM's full immediate encoding supports larger
 * values via rotation, but we keep it simple for educational clarity.
 *
 * @param rd  Destination register (0-15)
 * @param imm Immediate value (0-255)
 */
export function encodeMovImm(rd: number, imm: number): number {
  const cond = CONDITION_CODES.get("AL")!;
  const opcode = OPCODES.get("MOV")!;
  return encodeDataProcessing(cond, opcode, false, 0, rd, imm & 0xFF, true);
}

/**
 * Encode: ADD Rd, Rn, Rm → 32-bit instruction.
 *
 * ADD (Addition) computes Rn + Rm and stores the result in Rd.
 *
 * Example:
 *   encodeAdd(2, 0, 1)  → ADD R2, R0, R1  → 0xE0802001
 *
 * @param rd  Destination register
 * @param rn  First source register
 * @param rm  Second source register
 */
export function encodeAdd(rd: number, rn: number, rm: number): number {
  const cond = CONDITION_CODES.get("AL")!;
  const opcode = OPCODES.get("ADD")!;
  return encodeDataProcessing(cond, opcode, false, rn, rd, rm, false);
}

/**
 * Encode: SUB Rd, Rn, Rm → 32-bit instruction.
 *
 * SUB (Subtraction) computes Rn - Rm and stores the result in Rd.
 *
 * Example:
 *   encodeSub(2, 0, 1)  → SUB R2, R0, R1  → 0xE0402001
 *
 * @param rd  Destination register
 * @param rn  First source register
 * @param rm  Second source register
 */
export function encodeSub(rd: number, rn: number, rm: number): number {
  const cond = CONDITION_CODES.get("AL")!;
  const opcode = OPCODES.get("SUB")!;
  return encodeDataProcessing(cond, opcode, false, rn, rd, rm, false);
}

/**
 * Encode: HLT → 32-bit instruction (custom halt sentinel).
 *
 * We use 0xFFFFFFFF as a custom halt sentinel. In real ARM, condition
 * code 0b1111 has special meaning (unconditional instructions in ARMv5+).
 * We repurpose it as a clean way to stop the simulator.
 *
 * Example:
 *   encodeHlt()  → 0xFFFFFFFF
 */
export function encodeHlt(): number {
  return HLT_INSTRUCTION;
}

// ---------------------------------------------------------------------------
// Immediate encoding helper
// ---------------------------------------------------------------------------

/**
 * Try to encode an immediate value into ARM's 8-bit-with-rotation format.
 *
 * === ARM immediate encoding ===
 *
 * ARM can only encode 8-bit immediates in data processing instructions,
 * but those 8 bits can be rotated right by an even number of positions
 * (0, 2, 4, ..., 30). This clever trick allows encoding values like:
 *
 *   0xFF       = 0xFF rotated by 0   → rotate=0,  imm8=0xFF
 *   0xFF0      = 0xFF rotated by 28  → rotate=14, imm8=0xFF
 *   0xC0000034 = 0x0D rotated by 2   → rotate=1,  imm8=0x0D
 *
 * But NOT all 32-bit values can be encoded this way. For example,
 * 0x01010101 cannot be expressed as an 8-bit value with any rotation.
 *
 * @param value  The 32-bit value to encode
 * @returns      The 12-bit operand2 field (rotate | imm8), or null if
 *               the value can't be encoded as an immediate
 *
 * @example
 *   encodeImmediate(1)     // => 1      (rotate=0, imm8=1)
 *   encodeImmediate(0xFF)  // => 0xFF   (rotate=0, imm8=0xFF)
 *   encodeImmediate(0x3FC) // => 0xFFF  (rotate=15, imm8=0xFF)
 *   encodeImmediate(0x101) // => null   (cannot be encoded)
 */
export function encodeImmediate(value: number): number | null {
  // Ensure we're working with unsigned 32-bit
  const unsigned = value >>> 0;

  // Try each possible rotation (0, 2, 4, ..., 30)
  for (let rotate = 0; rotate < 16; rotate++) {
    // Rotate LEFT by (rotate * 2) to undo what the CPU will do (rotate right)
    const shift = rotate * 2;
    // Rotate right by shift positions to check if we get an 8-bit value
    const rotated = shift === 0
      ? unsigned
      : ((unsigned << shift) | (unsigned >>> (32 - shift))) >>> 0;

    if (rotated <= 0xFF) {
      // Success! The value can be encoded as imm8 rotated right by (rotate*2)
      return (rotate << 8) | rotated;
    }
  }

  // No rotation produces an 8-bit value — this immediate can't be encoded
  return null;
}

// ---------------------------------------------------------------------------
// Machine code assembly helper
// ---------------------------------------------------------------------------

/**
 * Convert an array of 32-bit instruction words to bytes (little-endian).
 *
 * ARM uses little-endian byte order in its default configuration. Each
 * instruction is 4 bytes, so a program of N instructions produces 4N bytes.
 *
 * Little-endian means the least significant byte comes first:
 *   0xE3A00001 → [0x01, 0x00, 0xA0, 0xE3]
 *
 * This is a convenience function for creating test programs from raw
 * instruction values, mirroring the Python ARM simulator's `assemble()`.
 *
 * @example
 *   const program = instructionsToBytes([
 *     encodeMovImm(0, 1),     // MOV R0, #1
 *     encodeMovImm(1, 2),     // MOV R1, #2
 *     encodeAdd(2, 0, 1),     // ADD R2, R0, R1
 *     encodeHlt(),            // HLT
 *   ]);
 *   // program is a Uint8Array of 16 bytes
 */
export function instructionsToBytes(instructions: readonly number[]): Uint8Array {
  const buffer = new Uint8Array(instructions.length * 4);
  const view = new DataView(buffer.buffer);

  for (let i = 0; i < instructions.length; i++) {
    // Write each instruction as a 32-bit little-endian value
    view.setUint32(i * 4, instructions[i] >>> 0, true /* littleEndian */);
  }

  return buffer;
}
