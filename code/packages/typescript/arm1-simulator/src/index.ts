/**
 * =========================================================================
 * @coding-adventures/arm1-simulator
 * =========================================================================
 *
 * ARM1 (ARMv1) behavioral instruction set simulator — ported from Go.
 *
 * The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
 * in Cambridge, UK. First silicon powered on April 26, 1985 — and worked
 * correctly on the very first attempt. This file implements a complete
 * behavioral simulator for the ARM1's instruction set.
 *
 * # Architecture Summary
 *
 *   - 32-bit RISC processor, 25,000 transistors
 *   - 16 visible registers (R0-R15), 25 physical (banked for FIQ/IRQ/SVC)
 *   - R15 = combined Program Counter + Status Register
 *   - 3-stage pipeline: Fetch -> Decode -> Execute
 *   - Every instruction is conditional (4-bit condition code)
 *   - Inline barrel shifter on Operand2 (shift for free)
 *   - No multiply instruction (added in ARM2)
 *   - No cache, no MMU
 *   - 26-bit address space (64 MiB)
 *
 * # CRITICAL TypeScript Notes
 *
 * JavaScript bitwise operators operate on SIGNED 32-bit integers. This means:
 *   - `(1 << 31)` produces `-2147483648`, not `2147483648`
 *   - `0x80000000 | 0` produces `-2147483648`
 *   - To get UNSIGNED 32-bit results, use `>>> 0` (unsigned right shift by 0)
 *   - `(1 << 32)` wraps to `1` (shift amount is mod 32)
 *
 * Every bitwise result that should be unsigned must use `>>> 0`.
 */

export const VERSION = "0.1.0";

// =========================================================================
// Processor Modes
// =========================================================================
//
// The ARM1 supports 4 processor modes. Each mode has its own banked copies
// of certain registers, allowing fast context switching.
//
//   Mode  M1:M0  Banked Registers
//   ----  -----  ----------------
//   USR   0b00   (none - base set)
//   FIQ   0b01   R8_fiq..R12_fiq, R13_fiq, R14_fiq
//   IRQ   0b10   R13_irq, R14_irq
//   SVC   0b11   R13_svc, R14_svc

export const MODE_USR = 0;
export const MODE_FIQ = 1;
export const MODE_IRQ = 2;
export const MODE_SVC = 3;

/** Returns a human-readable name for a processor mode. */
export function modeString(mode: number): string {
  switch (mode) {
    case MODE_USR: return "USR";
    case MODE_FIQ: return "FIQ";
    case MODE_IRQ: return "IRQ";
    case MODE_SVC: return "SVC";
    default: return "???";
  }
}

// =========================================================================
// Condition Codes
// =========================================================================
//
// Every ARM instruction has a 4-bit condition code in bits 31:28.
// The instruction only executes if the condition is met.

export const COND_EQ = 0x0;  // Equal - Z set
export const COND_NE = 0x1;  // Not equal - Z clear
export const COND_CS = 0x2;  // Carry set / unsigned higher or same
export const COND_CC = 0x3;  // Carry clear / unsigned lower
export const COND_MI = 0x4;  // Minus / negative - N set
export const COND_PL = 0x5;  // Plus / positive or zero - N clear
export const COND_VS = 0x6;  // Overflow set
export const COND_VC = 0x7;  // Overflow clear
export const COND_HI = 0x8;  // Unsigned higher - C set AND Z clear
export const COND_LS = 0x9;  // Unsigned lower or same - C clear OR Z set
export const COND_GE = 0xA;  // Signed greater or equal - N == V
export const COND_LT = 0xB;  // Signed less than - N != V
export const COND_GT = 0xC;  // Signed greater than - Z clear AND N == V
export const COND_LE = 0xD;  // Signed less or equal - Z set OR N != V
export const COND_AL = 0xE;  // Always (unconditional)
export const COND_NV = 0xF;  // Never (reserved - do not use)

/** Returns the assembly-language suffix for a condition code. */
export function condString(cond: number): string {
  const names: Record<number, string> = {
    [COND_EQ]: "EQ", [COND_NE]: "NE", [COND_CS]: "CS", [COND_CC]: "CC",
    [COND_MI]: "MI", [COND_PL]: "PL", [COND_VS]: "VS", [COND_VC]: "VC",
    [COND_HI]: "HI", [COND_LS]: "LS", [COND_GE]: "GE", [COND_LT]: "LT",
    [COND_GT]: "GT", [COND_LE]: "LE", [COND_AL]: "", [COND_NV]: "NV",
  };
  return names[cond] ?? "??";
}

// =========================================================================
// ALU Opcodes
// =========================================================================
//
// The ARM1's ALU supports 16 operations, selected by bits 24:21.

export const OP_AND = 0x0;  // Rd = Rn AND Op2
export const OP_EOR = 0x1;  // Rd = Rn XOR Op2
export const OP_SUB = 0x2;  // Rd = Rn - Op2
export const OP_RSB = 0x3;  // Rd = Op2 - Rn
export const OP_ADD = 0x4;  // Rd = Rn + Op2
export const OP_ADC = 0x5;  // Rd = Rn + Op2 + Carry
export const OP_SBC = 0x6;  // Rd = Rn - Op2 - NOT(Carry)
export const OP_RSC = 0x7;  // Rd = Op2 - Rn - NOT(Carry)
export const OP_TST = 0x8;  // Rn AND Op2, flags only
export const OP_TEQ = 0x9;  // Rn XOR Op2, flags only
export const OP_CMP = 0xA;  // Rn - Op2, flags only
export const OP_CMN = 0xB;  // Rn + Op2, flags only
export const OP_ORR = 0xC;  // Rd = Rn OR Op2
export const OP_MOV = 0xD;  // Rd = Op2
export const OP_BIC = 0xE;  // Rd = Rn AND NOT(Op2)
export const OP_MVN = 0xF;  // Rd = NOT(Op2)

const OP_NAMES = [
  "AND", "EOR", "SUB", "RSB", "ADD", "ADC", "SBC", "RSC",
  "TST", "TEQ", "CMP", "CMN", "ORR", "MOV", "BIC", "MVN",
];

/** Returns the mnemonic for an ALU opcode. */
export function opString(opcode: number): string {
  if (opcode >= 0 && opcode < 16) return OP_NAMES[opcode];
  return "???";
}

/** Returns true if the ALU opcode is a test-only operation (TST, TEQ, CMP, CMN). */
export function isTestOp(opcode: number): boolean {
  return opcode >= OP_TST && opcode <= OP_CMN;
}

/** Returns true if the ALU opcode is a logical operation. */
export function isLogicalOp(opcode: number): boolean {
  switch (opcode) {
    case OP_AND: case OP_EOR: case OP_TST: case OP_TEQ:
    case OP_ORR: case OP_MOV: case OP_BIC: case OP_MVN:
      return true;
    default:
      return false;
  }
}

// =========================================================================
// Shift Types
// =========================================================================
//
// The barrel shifter supports 4 shift types, encoded in bits 6:5.

export const SHIFT_LSL = 0;  // Logical Shift Left
export const SHIFT_LSR = 1;  // Logical Shift Right
export const SHIFT_ASR = 2;  // Arithmetic Shift Right (sign-extending)
export const SHIFT_ROR = 3;  // Rotate Right (ROR #0 encodes RRX)

/** Returns the mnemonic for a shift type. */
export function shiftString(shiftType: number): string {
  switch (shiftType) {
    case SHIFT_LSL: return "LSL";
    case SHIFT_LSR: return "LSR";
    case SHIFT_ASR: return "ASR";
    case SHIFT_ROR: return "ROR";
    default: return "???";
  }
}

// =========================================================================
// R15 Bit Positions
// =========================================================================
//
// R15 is the combined PC + Status Register:
//   Bit 31: N (Negative)     Bit 27: I (IRQ disable)
//   Bit 30: Z (Zero)         Bit 26: F (FIQ disable)
//   Bit 29: C (Carry)        Bits 25:2: Program Counter (24 bits)
//   Bit 28: V (Overflow)     Bits 1:0: Processor Mode

export const FLAG_N    = (1 << 31) >>> 0;  // 0x80000000
export const FLAG_Z    = (1 << 30) >>> 0;  // 0x40000000
export const FLAG_C    = (1 << 29) >>> 0;  // 0x20000000
export const FLAG_V    = (1 << 28) >>> 0;  // 0x10000000
export const FLAG_I    = (1 << 27) >>> 0;  // 0x08000000
export const FLAG_F    = (1 << 26) >>> 0;  // 0x04000000
export const PC_MASK   = 0x03FFFFFC;       // Bits 25:2 - the 24-bit PC field
export const MODE_MASK = 0x3;              // Bits 1:0 - processor mode
export const HALT_SWI  = 0x123456;

// =========================================================================
// Interfaces
// =========================================================================

/** Represents the ARM1's four condition flags. */
export interface Flags {
  N: boolean;  // Negative - set when result's bit 31 is 1
  Z: boolean;  // Zero - set when result is 0
  C: boolean;  // Carry - set on unsigned overflow or shifter carry-out
  V: boolean;  // Overflow - set on signed overflow
}

/** Records the state change caused by executing one instruction. */
export interface Trace {
  address: number;       // PC where this instruction was fetched
  raw: number;           // The 32-bit instruction word
  mnemonic: string;      // Disassembled form
  condition: string;     // Condition code suffix
  conditionMet: boolean; // Did the condition check pass?
  regsBefore: number[];  // 16 registers before execution
  regsAfter: number[];   // 16 registers after execution
  flagsBefore: Flags;
  flagsAfter: Flags;
  memoryReads: MemoryAccess[];
  memoryWrites: MemoryAccess[];
}

/** Records a single memory read or write. */
export interface MemoryAccess {
  address: number;
  value: number;
}

/** Holds the output of an ALU operation. */
export interface ALUResult {
  result: number;      // The 32-bit result (unsigned)
  N: boolean;          // Negative flag
  Z: boolean;          // Zero flag
  C: boolean;          // Carry flag
  V: boolean;          // Overflow flag
  writeResult: boolean; // Should the result be written to Rd?
}

// =========================================================================
// Instruction Types
// =========================================================================

export const INST_DATA_PROCESSING = 0;
export const INST_LOAD_STORE = 1;
export const INST_BLOCK_TRANSFER = 2;
export const INST_BRANCH = 3;
export const INST_SWI = 4;
export const INST_COPROCESSOR = 5;
export const INST_UNDEFINED = 6;

/** Holds all fields extracted from a 32-bit ARM instruction. */
export interface DecodedInstruction {
  raw: number;
  type: number;
  cond: number;

  // Data Processing fields
  opcode: number;
  s: boolean;
  rn: number;
  rd: number;
  immediate: boolean;

  // Operand2 - immediate form
  imm8: number;
  rotate: number;

  // Operand2 - register form
  rm: number;
  shiftType: number;
  shiftByReg: boolean;
  shiftImm: number;
  rs: number;

  // Load/Store fields
  load: boolean;
  byte: boolean;
  preIndex: boolean;
  up: boolean;
  writeBack: boolean;
  offset12: number;

  // Block Transfer fields
  registerList: number;
  forceUser: boolean;

  // Branch fields
  link: boolean;
  branchOffset: number;

  // SWI fields
  swiComment: number;
}

// =========================================================================
// Condition Evaluator
// =========================================================================
//
// Every ARM instruction has a 4-bit condition code in bits 31:28. The
// instruction only executes if the condition is satisfied by the current
// flags (N, Z, C, V).
//
// Condition Truth Table:
//
//   Code  Suffix  Meaning                  Test
//   ----  ------  -------                  ----
//   0000  EQ      Equal                    Z == 1
//   0001  NE      Not Equal                Z == 0
//   0010  CS/HS   Carry Set / Unsigned >=  C == 1
//   0011  CC/LO   Carry Clear / Unsigned < C == 0
//   0100  MI      Minus (Negative)         N == 1
//   0101  PL      Plus (Non-negative)      N == 0
//   0110  VS      Overflow Set             V == 1
//   0111  VC      Overflow Clear           V == 0
//   1000  HI      Unsigned Higher          C == 1 AND Z == 0
//   1001  LS      Unsigned Lower or Same   C == 0 OR  Z == 1
//   1010  GE      Signed >=                N == V
//   1011  LT      Signed <                 N != V
//   1100  GT      Signed >                 Z == 0 AND N == V
//   1101  LE      Signed <=                Z == 1 OR  N != V
//   1110  AL      Always                   true
//   1111  NV      Never (reserved)         false

export function evaluateCondition(cond: number, flags: Flags): boolean {
  switch (cond) {
    case COND_EQ: return flags.Z;
    case COND_NE: return !flags.Z;
    case COND_CS: return flags.C;
    case COND_CC: return !flags.C;
    case COND_MI: return flags.N;
    case COND_PL: return !flags.N;
    case COND_VS: return flags.V;
    case COND_VC: return !flags.V;
    case COND_HI: return flags.C && !flags.Z;
    case COND_LS: return !flags.C || flags.Z;
    case COND_GE: return flags.N === flags.V;
    case COND_LT: return flags.N !== flags.V;
    case COND_GT: return !flags.Z && (flags.N === flags.V);
    case COND_LE: return flags.Z || (flags.N !== flags.V);
    case COND_AL: return true;
    case COND_NV: return false;
    default: return false;
  }
}

// =========================================================================
// Barrel Shifter
// =========================================================================
//
// The barrel shifter is the ARM1's most distinctive hardware feature. On the
// real chip, it was a 32x32 crossbar network of pass transistors.
//
// Every data processing instruction's second operand passes through the
// barrel shifter before reaching the ALU. This means instructions like:
//
//   ADD R0, R1, R2, LSL #3    -> R0 = R1 + (R2 << 3)
//
// execute in a single cycle - the shift is free.

/**
 * Applies a shift operation to a 32-bit value.
 *
 * Returns [result, carryOut] where result is the shifted value and
 * carryOut is the carry flag from the shifter.
 */
export function barrelShift(
  value: number,
  shiftType: number,
  amount: number,
  carryIn: boolean,
  byRegister: boolean,
): [number, boolean] {
  // Ensure value is unsigned 32-bit
  value = value >>> 0;

  // When shifting by a register value, if amount is 0, value passes through
  // unchanged and carry is unaffected.
  if (byRegister && amount === 0) {
    return [value, carryIn];
  }

  switch (shiftType) {
    case SHIFT_LSL: return shiftLSL(value, amount, carryIn, byRegister);
    case SHIFT_LSR: return shiftLSR(value, amount, carryIn, byRegister);
    case SHIFT_ASR: return shiftASR(value, amount, carryIn, byRegister);
    case SHIFT_ROR: return shiftROR(value, amount, carryIn, byRegister);
    default: return [value, carryIn];
  }
}

/**
 * Logical Shift Left.
 *
 *   Before (LSL #3):  [b31 b30 ... b3 b2 b1 b0]
 *   After:            [b28 b27 ... b0  0  0  0 ]
 *   Carry out:        b29 (the last bit shifted out)
 *
 * Special case: LSL #0 means "no shift" - value unchanged, carry unchanged.
 */
function shiftLSL(value: number, amount: number, carryIn: boolean, _byRegister: boolean): [number, boolean] {
  if (amount === 0) {
    return [value, carryIn];
  }
  if (amount >= 32) {
    if (amount === 32) {
      return [0, (value & 1) !== 0];
    }
    return [0, false];
  }
  const carry = ((value >>> (32 - amount)) & 1) !== 0;
  return [(value << amount) >>> 0, carry];
}

/**
 * Logical Shift Right.
 *
 * Special case: immediate LSR #0 encodes LSR #32 (result = 0, carry = bit 31).
 */
function shiftLSR(value: number, amount: number, carryIn: boolean, byRegister: boolean): [number, boolean] {
  if (amount === 0 && !byRegister) {
    // Immediate LSR #0 encodes LSR #32
    return [0, (value >>> 31) !== 0];
  }
  if (amount === 0) {
    return [value, carryIn];
  }
  if (amount >= 32) {
    if (amount === 32) {
      return [0, (value >>> 31) !== 0];
    }
    return [0, false];
  }
  const carry = ((value >>> (amount - 1)) & 1) !== 0;
  return [value >>> amount, carry];
}

/**
 * Arithmetic Shift Right (sign-extending).
 *
 * The sign bit (bit 31) is replicated into vacated positions.
 *
 * Special case: immediate ASR #0 encodes ASR #32:
 *   If bit 31 = 0: result = 0x00000000, carry = 0
 *   If bit 31 = 1: result = 0xFFFFFFFF, carry = 1
 */
function shiftASR(value: number, amount: number, carryIn: boolean, byRegister: boolean): [number, boolean] {
  const signBit = (value >>> 31) !== 0;

  if (amount === 0 && !byRegister) {
    if (signBit) {
      return [0xFFFFFFFF, true];
    }
    return [0, false];
  }
  if (amount === 0) {
    return [value, carryIn];
  }
  if (amount >= 32) {
    if (signBit) {
      return [0xFFFFFFFF, true];
    }
    return [0, false];
  }

  // Arithmetic right shift: use signed interpretation then convert back.
  // In JavaScript, `>>` is arithmetic shift right (sign-extending).
  // We use `value | 0` to ensure it's treated as signed 32-bit, then `>>> 0`
  // to convert the result back to unsigned.
  const signed = value | 0;  // reinterpret as signed 32-bit
  const result = (signed >> amount) >>> 0;
  const carry = ((value >>> (amount - 1)) & 1) !== 0;
  return [result, carry];
}

/**
 * Rotate Right.
 *
 * Special case: immediate ROR #0 encodes RRX (Rotate Right Extended):
 *   33-bit rotation through carry flag. Old carry -> bit 31, old bit 0 -> new carry.
 */
function shiftROR(value: number, amount: number, carryIn: boolean, byRegister: boolean): [number, boolean] {
  if (amount === 0 && !byRegister) {
    // RRX - Rotate Right Extended (33-bit rotation through carry)
    const carry = (value & 1) !== 0;
    let result = value >>> 1;
    if (carryIn) {
      result = (result | 0x80000000) >>> 0;
    }
    return [result, carry];
  }
  if (amount === 0) {
    return [value, carryIn];
  }

  // Normalize rotation amount to 0-31
  amount = amount & 31;
  if (amount === 0) {
    // ROR by 32 (or multiple of 32): value unchanged, carry = bit 31
    return [value, (value >>> 31) !== 0];
  }

  const result = ((value >>> amount) | (value << (32 - amount))) >>> 0;
  const carry = ((result >>> 31) & 1) !== 0;
  return [result, carry];
}

/**
 * Decodes a rotated immediate value from the Operand2 field.
 *
 * The encoding packs a wide range of constants into 12 bits:
 *   - Bits 7:0:   8-bit immediate value
 *   - Bits 11:8:  4-bit rotation amount (actual rotation = 2 * this value)
 *
 * Returns [value, carryOut].
 */
export function decodeImmediate(imm8: number, rotate: number): [number, boolean] {
  const rotateAmount = rotate * 2;
  if (rotateAmount === 0) {
    return [imm8, false];
  }
  const value = ((imm8 >>> rotateAmount) | (imm8 << (32 - rotateAmount))) >>> 0;
  const carryOut = (value >>> 31) !== 0;
  return [value, carryOut];
}

// =========================================================================
// ALU (32-bit Arithmetic Logic Unit)
// =========================================================================
//
// The ARM1's ALU performs 16 operations. It takes two 32-bit inputs (Rn and
// the barrel-shifted Operand2) and produces a 32-bit result plus four
// condition flags (N, Z, C, V).
//
// Flag computation differs for logical vs arithmetic ops:
//
// Arithmetic (ADD, SUB, ADC, SBC, RSB, RSC, CMP, CMN):
//   N = result bit 31, Z = result == 0
//   C = carry out from 32-bit adder
//   V = signed overflow
//
// Logical (AND, EOR, TST, TEQ, ORR, MOV, BIC, MVN):
//   N = result bit 31, Z = result == 0
//   C = carry out from barrel shifter
//   V = unchanged

/**
 * Performs one of the 16 ALU operations.
 *
 * Parameters:
 *   opcode       - 4-bit ALU operation
 *   a            - first operand (value of Rn)
 *   b            - second operand (barrel-shifted Operand2)
 *   carryIn      - current carry flag
 *   shifterCarry - carry output from barrel shifter
 *   oldV         - current overflow flag
 */
export function aluExecute(
  opcode: number,
  a: number,
  b: number,
  carryIn: boolean,
  shifterCarry: boolean,
  oldV: boolean,
): ALUResult {
  // Ensure unsigned 32-bit
  a = a >>> 0;
  b = b >>> 0;

  let result = 0;
  let carry = false;
  let overflow = false;
  const writeResult = !isTestOp(opcode);

  switch (opcode) {
    // -- Logical operations --
    case OP_AND: case OP_TST:
      result = (a & b) >>> 0;
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_EOR: case OP_TEQ:
      result = (a ^ b) >>> 0;
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_ORR:
      result = (a | b) >>> 0;
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_MOV:
      result = b;
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_BIC:
      result = (a & (~b >>> 0)) >>> 0;
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_MVN:
      result = (~b) >>> 0;
      carry = shifterCarry;
      overflow = oldV;
      break;

    // -- Arithmetic operations --
    case OP_ADD: case OP_CMN:
      [result, carry, overflow] = add32(a, b, false);
      break;

    case OP_ADC:
      [result, carry, overflow] = add32(a, b, carryIn);
      break;

    case OP_SUB: case OP_CMP:
      // A - B = A + NOT(B) + 1
      [result, carry, overflow] = add32(a, (~b) >>> 0, true);
      break;

    case OP_SBC:
      // A - B - NOT(C) = A + NOT(B) + C
      [result, carry, overflow] = add32(a, (~b) >>> 0, carryIn);
      break;

    case OP_RSB:
      // B - A = B + NOT(A) + 1
      [result, carry, overflow] = add32(b, (~a) >>> 0, true);
      break;

    case OP_RSC:
      // B - A - NOT(C) = B + NOT(A) + C
      [result, carry, overflow] = add32(b, (~a) >>> 0, carryIn);
      break;
  }

  return {
    result: result >>> 0,
    N: (result >>> 31) !== 0,
    Z: (result >>> 0) === 0,
    C: carry,
    V: overflow,
    writeResult,
  };
}

/**
 * Performs 32-bit addition with carry-in, producing carry-out and overflow.
 *
 * We use a technique that avoids 64-bit arithmetic (which JavaScript doesn't
 * natively support for bitwise ops): split each operand into high 16 bits
 * and low 16 bits, add with carry propagation.
 *
 * Overflow detection: both operands have the same sign, but the result has
 * a different sign.
 */
function add32(a: number, b: number, carryIn: boolean): [number, boolean, boolean] {
  a = a >>> 0;
  b = b >>> 0;
  const cin = carryIn ? 1 : 0;

  // Split into 16-bit halves for carry detection
  const loA = a & 0xFFFF;
  const hiA = a >>> 16;
  const loB = b & 0xFFFF;
  const hiB = b >>> 16;

  const loSum = loA + loB + cin;
  const loCarry = loSum > 0xFFFF ? 1 : 0;
  const hiSum = hiA + hiB + loCarry;
  const carry = hiSum > 0xFFFF;

  const result = (((hiSum & 0xFFFF) << 16) | (loSum & 0xFFFF)) >>> 0;

  // Overflow: (a ^ result) & (b ^ result) has bit 31 set
  const overflow = ((((a ^ result) & (b ^ result)) >>> 31) & 1) !== 0;

  return [result, carry, overflow];
}

// =========================================================================
// Instruction Decoder
// =========================================================================
//
// ARM1 instructions are classified by bits 27:25:
//
//   Bits 27:26  Bit 25  Class
//   ----------  ------  -----
//   00          -       Data Processing / PSR Transfer
//   01          -       Single Data Transfer (LDR/STR)
//   10          0       Block Data Transfer (LDM/STM)
//   10          1       Branch (B/BL)
//   11          -       Coprocessor / SWI

/** Creates a default (zeroed) decoded instruction. */
function emptyDecoded(): DecodedInstruction {
  return {
    raw: 0, type: 0, cond: 0,
    opcode: 0, s: false, rn: 0, rd: 0, immediate: false,
    imm8: 0, rotate: 0,
    rm: 0, shiftType: 0, shiftByReg: false, shiftImm: 0, rs: 0,
    load: false, byte: false, preIndex: false, up: false, writeBack: false, offset12: 0,
    registerList: 0, forceUser: false,
    link: false, branchOffset: 0,
    swiComment: 0,
  };
}

/** Decodes a 32-bit ARM instruction into its constituent fields. */
export function decode(instruction: number): DecodedInstruction {
  instruction = instruction >>> 0;
  const d = emptyDecoded();
  d.raw = instruction;
  d.cond = (instruction >>> 28) & 0xF;

  const bits2726 = (instruction >>> 26) & 0x3;
  const bit25 = (instruction >>> 25) & 0x1;

  if (bits2726 === 0) {
    d.type = INST_DATA_PROCESSING;
    decodeDataProcessing(d, instruction);
  } else if (bits2726 === 1) {
    d.type = INST_LOAD_STORE;
    decodeLoadStore(d, instruction);
  } else if (bits2726 === 2 && bit25 === 0) {
    d.type = INST_BLOCK_TRANSFER;
    decodeBlockTransfer(d, instruction);
  } else if (bits2726 === 2 && bit25 === 1) {
    d.type = INST_BRANCH;
    decodeBranch(d, instruction);
  } else if (bits2726 === 3) {
    if (((instruction >>> 24) & 0xF) === 0xF) {
      d.type = INST_SWI;
      d.swiComment = instruction & 0x00FFFFFF;
    } else {
      d.type = INST_COPROCESSOR;
    }
  } else {
    d.type = INST_UNDEFINED;
  }

  return d;
}

function decodeDataProcessing(d: DecodedInstruction, inst: number): void {
  d.immediate = ((inst >>> 25) & 1) === 1;
  d.opcode = (inst >>> 21) & 0xF;
  d.s = ((inst >>> 20) & 1) === 1;
  d.rn = (inst >>> 16) & 0xF;
  d.rd = (inst >>> 12) & 0xF;

  if (d.immediate) {
    d.imm8 = inst & 0xFF;
    d.rotate = (inst >>> 8) & 0xF;
  } else {
    d.rm = inst & 0xF;
    d.shiftType = (inst >>> 5) & 0x3;
    d.shiftByReg = ((inst >>> 4) & 1) === 1;
    if (d.shiftByReg) {
      d.rs = (inst >>> 8) & 0xF;
    } else {
      d.shiftImm = (inst >>> 7) & 0x1F;
    }
  }
}

function decodeLoadStore(d: DecodedInstruction, inst: number): void {
  d.immediate = ((inst >>> 25) & 1) === 1;  // For LDR/STR, I=1 means REGISTER offset
  d.preIndex = ((inst >>> 24) & 1) === 1;
  d.up = ((inst >>> 23) & 1) === 1;
  d.byte = ((inst >>> 22) & 1) === 1;
  d.writeBack = ((inst >>> 21) & 1) === 1;
  d.load = ((inst >>> 20) & 1) === 1;
  d.rn = (inst >>> 16) & 0xF;
  d.rd = (inst >>> 12) & 0xF;

  if (d.immediate) {
    // Register offset
    d.rm = inst & 0xF;
    d.shiftType = (inst >>> 5) & 0x3;
    d.shiftImm = (inst >>> 7) & 0x1F;
  } else {
    // Immediate offset
    d.offset12 = inst & 0xFFF;
  }
}

function decodeBlockTransfer(d: DecodedInstruction, inst: number): void {
  d.preIndex = ((inst >>> 24) & 1) === 1;
  d.up = ((inst >>> 23) & 1) === 1;
  d.forceUser = ((inst >>> 22) & 1) === 1;
  d.writeBack = ((inst >>> 21) & 1) === 1;
  d.load = ((inst >>> 20) & 1) === 1;
  d.rn = (inst >>> 16) & 0xF;
  d.registerList = inst & 0xFFFF;
}

function decodeBranch(d: DecodedInstruction, inst: number): void {
  d.link = ((inst >>> 24) & 1) === 1;

  // The 24-bit offset is sign-extended to 32 bits, then shifted left by 2
  let offset = inst & 0x00FFFFFF;
  // Sign-extend from 24 bits to 32 bits
  if ((offset >>> 23) !== 0) {
    offset = (offset | 0xFF000000) >>> 0;
  }
  // Convert to signed 32-bit, shift left by 2
  d.branchOffset = (offset | 0) << 2;  // `| 0` makes it signed, `<< 2` shifts
}

// =========================================================================
// Disassembly
// =========================================================================

/** Returns a human-readable assembly string for a decoded instruction. */
export function disassemble(d: DecodedInstruction): string {
  const cond = condString(d.cond);

  switch (d.type) {
    case INST_DATA_PROCESSING:
      return disasmDataProcessing(d, cond);
    case INST_LOAD_STORE:
      return disasmLoadStore(d, cond);
    case INST_BLOCK_TRANSFER:
      return disasmBlockTransfer(d, cond);
    case INST_BRANCH:
      return disasmBranch(d, cond);
    case INST_SWI:
      if (d.swiComment === HALT_SWI) return `HLT${cond}`;
      return `SWI${cond} #0x${d.swiComment.toString(16).toUpperCase()}`;
    case INST_COPROCESSOR:
      return `CDP${cond} (undefined)`;
    default:
      return `UND${cond} #0x${(d.raw >>> 0).toString(16).toUpperCase().padStart(8, "0")}`;
  }
}

function disasmDataProcessing(d: DecodedInstruction, cond: string): string {
  const op = opString(d.opcode);
  const suf = d.s && !isTestOp(d.opcode) ? "S" : "";
  const op2 = disasmOperand2(d);

  if (d.opcode === OP_MOV || d.opcode === OP_MVN) {
    return `${op}${cond}${suf} R${d.rd}, ${op2}`;
  }
  if (isTestOp(d.opcode)) {
    return `${op}${cond} R${d.rn}, ${op2}`;
  }
  return `${op}${cond}${suf} R${d.rd}, R${d.rn}, ${op2}`;
}

function disasmOperand2(d: DecodedInstruction): string {
  if (d.immediate) {
    const [val] = decodeImmediate(d.imm8, d.rotate);
    return `#${val}`;
  }
  if (!d.shiftByReg && d.shiftImm === 0 && d.shiftType === SHIFT_LSL) {
    return `R${d.rm}`;
  }
  if (d.shiftByReg) {
    return `R${d.rm}, ${shiftString(d.shiftType)} R${d.rs}`;
  }
  let amount = d.shiftImm;
  if (amount === 0) {
    switch (d.shiftType) {
      case SHIFT_LSR: case SHIFT_ASR:
        amount = 32;
        break;
      case SHIFT_ROR:
        return `R${d.rm}, RRX`;
    }
  }
  return `R${d.rm}, ${shiftString(d.shiftType)} #${amount}`;
}

function disasmLoadStore(d: DecodedInstruction, cond: string): string {
  const op = d.load ? "LDR" : "STR";
  const bSuf = d.byte ? "B" : "";

  let offset: string;
  if (d.immediate) {
    offset = `R${d.rm}`;
    if (d.shiftImm !== 0) {
      offset += `, ${shiftString(d.shiftType)} #${d.shiftImm}`;
    }
  } else {
    offset = `#${d.offset12}`;
  }

  const sign = d.up ? "" : "-";

  if (d.preIndex) {
    const wb = d.writeBack ? "!" : "";
    return `${op}${cond}${bSuf} R${d.rd}, [R${d.rn}, ${sign}${offset}]${wb}`;
  }
  return `${op}${cond}${bSuf} R${d.rd}, [R${d.rn}], ${sign}${offset}`;
}

function disasmBlockTransfer(d: DecodedInstruction, cond: string): string {
  const op = d.load ? "LDM" : "STM";

  let mode: string;
  if (!d.preIndex && d.up) mode = "IA";
  else if (d.preIndex && d.up) mode = "IB";
  else if (!d.preIndex && !d.up) mode = "DA";
  else mode = "DB";

  const wb = d.writeBack ? "!" : "";
  const regs = disasmRegList(d.registerList);
  return `${op}${cond}${mode} R${d.rn}${wb}, {${regs}}`;
}

function disasmBranch(d: DecodedInstruction, cond: string): string {
  const op = d.link ? "BL" : "B";
  return `${op}${cond} #${d.branchOffset}`;
}

function disasmRegList(list: number): string {
  const parts: string[] = [];
  for (let i = 0; i < 16; i++) {
    if ((list >>> i) & 1) {
      if (i === 15) parts.push("PC");
      else if (i === 14) parts.push("LR");
      else if (i === 13) parts.push("SP");
      else parts.push(`R${i}`);
    }
  }
  return parts.join(", ");
}

// =========================================================================
// ARM1 Simulator Class
// =========================================================================

export class ARM1 {
  /**
   * Register file: 27 physical 32-bit registers (stored as unsigned).
   *
   * Layout:
   *   [0..15]  = R0-R15 (User/System mode base registers)
   *   [16..22] = R8_fiq through R14_fiq
   *   [23..24] = R13_irq, R14_irq
   *   [25..26] = R13_svc, R14_svc
   */
  private regs: Uint32Array;

  /** Memory - byte-addressable, little-endian */
  private memory: Uint8Array;

  /** Has the CPU been halted? */
  private _halted: boolean;

  constructor(memorySize: number = 1024 * 1024) {
    if (memorySize <= 0) memorySize = 1024 * 1024;
    this.regs = new Uint32Array(27);
    this.memory = new Uint8Array(memorySize);
    this._halted = false;
    this.reset();
  }

  /**
   * Reset restores the CPU to its power-on state:
   *   - Supervisor mode (SVC)
   *   - IRQs and FIQs disabled
   *   - PC = 0
   *   - All flags cleared
   */
  reset(): void {
    this.regs.fill(0);
    // Set R15: SVC mode (bits 1:0 = 11), IRQ/FIQ disabled (bits 27,26 = 11)
    this.regs[15] = (FLAG_I | FLAG_F | MODE_SVC) >>> 0;
    this._halted = false;
  }

  // =====================================================================
  // Register access
  // =====================================================================

  readRegister(index: number): number {
    return this.regs[this.physicalReg(index)];
  }

  writeRegister(index: number, value: number): void {
    this.regs[this.physicalReg(index)] = value >>> 0;
  }

  private physicalReg(index: number): number {
    const mode = this.mode;
    if (mode === MODE_FIQ && index >= 8 && index <= 14) {
      return 16 + (index - 8);
    }
    if (mode === MODE_IRQ && index >= 13 && index <= 14) {
      return 23 + (index - 13);
    }
    if (mode === MODE_SVC && index >= 13 && index <= 14) {
      return 25 + (index - 13);
    }
    return index;
  }

  /** Current program counter (26-bit address). */
  get pc(): number {
    return this.regs[15] & PC_MASK;
  }

  /** Sets the program counter portion of R15 without changing flags/mode. */
  set pc(addr: number) {
    this.regs[15] = ((this.regs[15] & ~PC_MASK) | (addr & PC_MASK)) >>> 0;
  }

  /** Current condition flags. */
  get flags(): Flags {
    const r15 = this.regs[15];
    return {
      N: (r15 & FLAG_N) !== 0,
      Z: (r15 & FLAG_Z) !== 0,
      C: (r15 & FLAG_C) !== 0,
      V: (r15 & FLAG_V) !== 0,
    };
  }

  /** Updates the condition flags in R15. */
  set flags(f: Flags) {
    let r15 = this.regs[15] & (~(FLAG_N | FLAG_Z | FLAG_C | FLAG_V) >>> 0);
    if (f.N) r15 |= FLAG_N;
    if (f.Z) r15 |= FLAG_Z;
    if (f.C) r15 |= FLAG_C;
    if (f.V) r15 |= FLAG_V;
    this.regs[15] = r15 >>> 0;
  }

  /** Current processor mode (0=USR, 1=FIQ, 2=IRQ, 3=SVC). */
  get mode(): number {
    return this.regs[15] & MODE_MASK;
  }

  /** Whether the CPU has been halted. */
  get halted(): boolean {
    return this._halted;
  }

  // =====================================================================
  // Memory access
  // =====================================================================

  /** Reads a 32-bit word from memory (little-endian, word-aligned). */
  readWord(addr: number): number {
    addr = addr & PC_MASK;
    const a = addr & ~3;  // Word-align
    if (a + 3 >= this.memory.length) return 0;
    return (
      this.memory[a] |
      (this.memory[a + 1] << 8) |
      (this.memory[a + 2] << 16) |
      (this.memory[a + 3] << 24)
    ) >>> 0;
  }

  /** Writes a 32-bit word to memory (little-endian). */
  writeWord(addr: number, value: number): void {
    addr = addr & PC_MASK;
    const a = addr & ~3;
    if (a + 3 >= this.memory.length) return;
    value = value >>> 0;
    this.memory[a] = value & 0xFF;
    this.memory[a + 1] = (value >>> 8) & 0xFF;
    this.memory[a + 2] = (value >>> 16) & 0xFF;
    this.memory[a + 3] = (value >>> 24) & 0xFF;
  }

  /** Reads a single byte from memory. */
  readByte(addr: number): number {
    addr = addr & PC_MASK;
    if (addr >= this.memory.length) return 0;
    return this.memory[addr];
  }

  /** Writes a single byte to memory. */
  writeByte(addr: number, value: number): void {
    addr = addr & PC_MASK;
    if (addr >= this.memory.length) return;
    this.memory[addr] = value & 0xFF;
  }

  /** Loads machine code into memory at the given start address. */
  loadProgram(code: Uint8Array | number[], startAddr: number = 0): void {
    for (let i = 0; i < code.length; i++) {
      const addr = startAddr + i;
      if (addr < this.memory.length) {
        this.memory[addr] = code[i];
      }
    }
  }

  // =====================================================================
  // Execution
  // =====================================================================

  /** Executes one instruction and returns a trace. */
  step(): Trace {
    const pcVal = this.pc;
    const regsBefore: number[] = [];
    for (let i = 0; i < 16; i++) regsBefore.push(this.readRegister(i));
    const flagsBefore = this.flags;

    // Fetch
    const instruction = this.readWord(pcVal);

    // Decode
    const decoded = decode(instruction);

    // Evaluate condition
    const condMet = evaluateCondition(decoded.cond, flagsBefore);

    const trace: Trace = {
      address: pcVal,
      raw: instruction,
      mnemonic: disassemble(decoded),
      condition: condString(decoded.cond),
      conditionMet: condMet,
      regsBefore,
      regsAfter: [],
      flagsBefore,
      flagsAfter: { N: false, Z: false, C: false, V: false },
      memoryReads: [],
      memoryWrites: [],
    };

    // Advance PC
    this.pc = pcVal + 4;

    if (condMet) {
      switch (decoded.type) {
        case INST_DATA_PROCESSING:
          this.executeDataProcessing(decoded, trace);
          break;
        case INST_LOAD_STORE:
          this.executeLoadStore(decoded, trace);
          break;
        case INST_BLOCK_TRANSFER:
          this.executeBlockTransfer(decoded, trace);
          break;
        case INST_BRANCH:
          this.executeBranch(decoded, trace);
          break;
        case INST_SWI:
          this.executeSWI(decoded, trace);
          break;
        case INST_COPROCESSOR:
        case INST_UNDEFINED:
          this.trapUndefined(pcVal);
          break;
      }
    }

    // Capture state after execution
    const regsAfter: number[] = [];
    for (let i = 0; i < 16; i++) regsAfter.push(this.readRegister(i));
    trace.regsAfter = regsAfter;
    trace.flagsAfter = this.flags;

    return trace;
  }

  /** Executes instructions until halted or maxSteps reached. */
  run(maxSteps: number): Trace[] {
    const traces: Trace[] = [];
    for (let i = 0; i < maxSteps && !this._halted; i++) {
      traces.push(this.step());
    }
    return traces;
  }

  // =====================================================================
  // Data Processing execution
  // =====================================================================

  private executeDataProcessing(d: DecodedInstruction, trace: Trace): void {
    // Get first operand (Rn)
    let a = 0;
    if (d.opcode !== OP_MOV && d.opcode !== OP_MVN) {
      a = this.readRegForExec(d.rn);
    }

    // Get second operand (Operand2) through barrel shifter
    let b: number;
    let shifterCarry: boolean;
    const currentFlags = this.flags;

    if (d.immediate) {
      [b, shifterCarry] = decodeImmediate(d.imm8, d.rotate);
      if (d.rotate === 0) {
        shifterCarry = currentFlags.C;
      }
    } else {
      const rmVal = this.readRegForExec(d.rm);
      let shiftAmount: number;
      if (d.shiftByReg) {
        shiftAmount = this.readRegForExec(d.rs) & 0xFF;
      } else {
        shiftAmount = d.shiftImm;
      }
      [b, shifterCarry] = barrelShift(rmVal, d.shiftType, shiftAmount, currentFlags.C, d.shiftByReg);
    }

    // Execute ALU operation
    const result = aluExecute(d.opcode, a, b, currentFlags.C, shifterCarry, currentFlags.V);

    // Write result to Rd
    if (result.writeResult) {
      if (d.rd === 15) {
        if (d.s) {
          // MOVS PC, LR - restore PC and flags
          this.regs[15] = result.result >>> 0;
        } else {
          this.pc = result.result & PC_MASK;
        }
      } else {
        this.writeRegister(d.rd, result.result);
      }
    }

    // Update flags
    if (d.s && d.rd !== 15) {
      this.flags = { N: result.N, Z: result.Z, C: result.C, V: result.V };
    }
    if (isTestOp(d.opcode)) {
      this.flags = { N: result.N, Z: result.Z, C: result.C, V: result.V };
    }
  }

  /**
   * Reads a register value as it would appear during instruction execution.
   * For R15, returns PC + 8 (accounting for 3-stage pipeline).
   */
  private readRegForExec(index: number): number {
    if (index === 15) {
      // R15 reads as PC + 8 during execution.
      // We already advanced PC by 4 in step(), so add 4 more.
      return (this.regs[15] + 4) >>> 0;
    }
    return this.readRegister(index);
  }

  // =====================================================================
  // Load/Store execution
  // =====================================================================

  private executeLoadStore(d: DecodedInstruction, trace: Trace): void {
    let offset: number;
    if (d.immediate) {
      // Register offset (with optional shift)
      let rmVal = this.readRegForExec(d.rm);
      if (d.shiftImm !== 0) {
        [rmVal] = barrelShift(rmVal, d.shiftType, d.shiftImm, this.flags.C, false);
      }
      offset = rmVal;
    } else {
      offset = d.offset12;
    }

    const base = this.readRegForExec(d.rn);

    let addr: number;
    if (d.up) {
      addr = (base + offset) >>> 0;
    } else {
      addr = (base - offset) >>> 0;
    }

    let transferAddr = addr;
    if (!d.preIndex) {
      transferAddr = base;
    }

    if (d.load) {
      let value: number;
      if (d.byte) {
        value = this.readByte(transferAddr);
      } else {
        value = this.readWord(transferAddr);
        // ARM1 quirk: unaligned word loads rotate the data
        const rotation = (transferAddr & 3) * 8;
        if (rotation !== 0) {
          value = ((value >>> rotation) | (value << (32 - rotation))) >>> 0;
        }
      }
      trace.memoryReads.push({ address: transferAddr, value });

      if (d.rd === 15) {
        this.regs[15] = value >>> 0;
      } else {
        this.writeRegister(d.rd, value);
      }
    } else {
      const value = this.readRegForExec(d.rd);
      if (d.byte) {
        this.writeByte(transferAddr, value & 0xFF);
      } else {
        this.writeWord(transferAddr, value);
      }
      trace.memoryWrites.push({ address: transferAddr, value });
    }

    // Write-back
    if (d.writeBack || !d.preIndex) {
      if (d.rn !== 15) {
        this.writeRegister(d.rn, addr);
      }
    }
  }

  // =====================================================================
  // Block Transfer execution (LDM/STM)
  // =====================================================================

  private executeBlockTransfer(d: DecodedInstruction, trace: Trace): void {
    const base = this.readRegister(d.rn);
    const regList = d.registerList;

    // Count registers
    let count = 0;
    for (let i = 0; i < 16; i++) {
      if ((regList >>> i) & 1) count++;
    }
    if (count === 0) return;

    // Calculate start address based on addressing mode
    let startAddr: number;
    if (!d.preIndex && d.up) {         // IA
      startAddr = base;
    } else if (d.preIndex && d.up) {   // IB
      startAddr = (base + 4) >>> 0;
    } else if (!d.preIndex && !d.up) { // DA
      startAddr = (base - (count * 4) + 4) >>> 0;
    } else {                            // DB
      startAddr = (base - (count * 4)) >>> 0;
    }

    let addr = startAddr;
    for (let i = 0; i < 16; i++) {
      if (!((regList >>> i) & 1)) continue;

      if (d.load) {
        const value = this.readWord(addr);
        trace.memoryReads.push({ address: addr, value });
        if (i === 15) {
          this.regs[15] = value >>> 0;
        } else {
          this.writeRegister(i, value);
        }
      } else {
        let value: number;
        if (i === 15) {
          value = (this.regs[15] + 4) >>> 0;  // PC + 8
        } else {
          value = this.readRegister(i);
        }
        this.writeWord(addr, value);
        trace.memoryWrites.push({ address: addr, value });
      }
      addr = (addr + 4) >>> 0;
    }

    // Write-back
    if (d.writeBack) {
      let newBase: number;
      if (d.up) {
        newBase = (base + (count * 4)) >>> 0;
      } else {
        newBase = (base - (count * 4)) >>> 0;
      }
      this.writeRegister(d.rn, newBase);
    }
  }

  // =====================================================================
  // Branch execution
  // =====================================================================

  private executeBranch(d: DecodedInstruction, trace: Trace): void {
    // Current PC (already advanced by 4 in step)
    const branchBase = (this.pc + 4) >>> 0;

    if (d.link) {
      // BL: save return address in R14 (LR)
      const returnAddr = this.regs[15];
      this.writeRegister(14, returnAddr);
    }

    // Compute target address
    const target = (branchBase + d.branchOffset) >>> 0;
    this.pc = target & PC_MASK;
  }

  // =====================================================================
  // SWI execution
  // =====================================================================

  private executeSWI(d: DecodedInstruction, _trace: Trace): void {
    if (d.swiComment === HALT_SWI) {
      this._halted = true;
      return;
    }

    // Real SWI: enter Supervisor mode
    this.regs[25] = this.regs[15];
    this.regs[26] = this.regs[15];

    let r15 = this.regs[15];
    r15 = ((r15 & (~MODE_MASK >>> 0)) | MODE_SVC) >>> 0;
    r15 = (r15 | FLAG_I) >>> 0;
    this.regs[15] = r15;

    this.pc = 0x08;
  }

  // =====================================================================
  // Exception handling
  // =====================================================================

  private trapUndefined(_instrAddr: number): void {
    this.regs[26] = this.regs[15];

    let r15 = this.regs[15];
    r15 = ((r15 & (~MODE_MASK >>> 0)) | MODE_SVC) >>> 0;
    r15 = (r15 | FLAG_I) >>> 0;
    this.regs[15] = r15;

    this.pc = 0x04;
  }
}

// =========================================================================
// Encoding Helpers
// =========================================================================
//
// These functions create instruction words for writing test programs
// without an assembler.

/** Creates a data processing instruction word. */
export function encodeDataProcessing(
  cond: number, opcode: number, s: number, rn: number, rd: number, operand2: number,
): number {
  return (
    (cond << 28) | operand2 |
    (opcode << 21) | (s << 20) |
    (rn << 16) | (rd << 12)
  ) >>> 0;
}

/** Creates a MOV immediate instruction. Example: encodeMovImm(COND_AL, 0, 42) -> MOV R0, #42 */
export function encodeMovImm(cond: number, rd: number, imm8: number): number {
  return encodeDataProcessing(cond, OP_MOV, 0, 0, rd, (1 << 25) | imm8);
}

/** Creates a data processing instruction with a register operand. */
export function encodeALUReg(
  cond: number, opcode: number, s: number, rd: number, rn: number, rm: number,
): number {
  return encodeDataProcessing(cond, opcode, s, rn, rd, rm);
}

/** Creates a Branch or Branch-with-Link instruction. */
export function encodeBranch(cond: number, link: boolean, offset: number): number {
  let inst = ((cond << 28) | 0x0A000000) >>> 0;
  if (link) {
    inst = (inst | 0x01000000) >>> 0;
  }
  // Offset is in bytes, relative to PC+8. Encode (offset/4) in 24 bits.
  const encoded = ((offset >> 2) & 0x00FFFFFF) >>> 0;
  inst = (inst | encoded) >>> 0;
  return inst;
}

/** Creates our pseudo-halt instruction (SWI 0x123456). */
export function encodeHalt(): number {
  return ((COND_AL << 28) | 0x0F000000 | HALT_SWI) >>> 0;
}

/** Creates a Load Register instruction with immediate offset. */
export function encodeLDR(
  cond: number, rd: number, rn: number, offset: number, preIndex: boolean,
): number {
  let inst = ((cond << 28) | 0x04100000) >>> 0;  // LDR, I=0
  inst = (inst | (rd << 12) | (rn << 16)) >>> 0;
  if (preIndex) inst = (inst | (1 << 24)) >>> 0;
  if (offset >= 0) {
    inst = (inst | (1 << 23) | (offset & 0xFFF)) >>> 0;
  } else {
    inst = (inst | ((-offset) & 0xFFF)) >>> 0;
  }
  return inst;
}

/** Creates a Store Register instruction with immediate offset. */
export function encodeSTR(
  cond: number, rd: number, rn: number, offset: number, preIndex: boolean,
): number {
  let inst = ((cond << 28) | 0x04000000) >>> 0;  // STR, I=0
  inst = (inst | (rd << 12) | (rn << 16)) >>> 0;
  if (preIndex) inst = (inst | (1 << 24)) >>> 0;
  if (offset >= 0) {
    inst = (inst | (1 << 23) | (offset & 0xFFF)) >>> 0;
  } else {
    inst = (inst | ((-offset) & 0xFFF)) >>> 0;
  }
  return inst;
}

/** Creates a Load Multiple instruction. */
export function encodeLDM(
  cond: number, rn: number, regList: number, writeBack: boolean, mode: string,
): number {
  let inst = ((cond << 28) | 0x08100000) >>> 0;  // LDM
  inst = (inst | (rn << 16) | (regList & 0xFFFF)) >>> 0;
  if (writeBack) inst = (inst | (1 << 21)) >>> 0;
  switch (mode) {
    case "IA": inst = (inst | (1 << 23)) >>> 0; break;              // P=0, U=1
    case "IB": inst = (inst | (1 << 24) | (1 << 23)) >>> 0; break;  // P=1, U=1
    case "DA": break;                                                 // P=0, U=0
    case "DB": inst = (inst | (1 << 24)) >>> 0; break;              // P=1, U=0
  }
  return inst;
}

/** Creates a Store Multiple instruction. */
export function encodeSTM(
  cond: number, rn: number, regList: number, writeBack: boolean, mode: string,
): number {
  let inst = encodeLDM(cond, rn, regList, writeBack, mode);
  inst = (inst & ~(1 << 20)) >>> 0;  // Clear L bit
  return inst;
}
