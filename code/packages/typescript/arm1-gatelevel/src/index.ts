/**
 * =========================================================================
 * @coding-adventures/arm1-gatelevel
 * =========================================================================
 *
 * ARM1 gate-level simulator built from logic gates.
 *
 * Every arithmetic operation routes through actual logic gate functions --
 * AND, OR, XOR, NOT -- chained into adders, then into a 32-bit ALU. The
 * barrel shifter is built from multiplexer trees.
 *
 * This is NOT the same as the behavioral simulator. Both produce identical
 * results for any program. The difference is the execution path:
 *
 *   Behavioral:  opcode -> match statement -> host arithmetic -> result
 *   Gate-level:  opcode -> decoder gates -> barrel shifter muxes ->
 *                ALU gates -> adder gates -> logic gates -> result
 *
 * # Architecture
 *
 * The gate-level simulator composes packages from layers below:
 *   - logic-gates: AND, OR, XOR, NOT, mux2 (2-input multiplexer)
 *   - arithmetic: rippleCarryAdder (32 full adders chained)
 *   - arm1-simulator: types, condition codes, instruction encoding helpers
 */

export const VERSION = "0.1.0";

// =========================================================================
// Imports
// =========================================================================

import {
  AND, OR, XOR, NOT, XNOR, mux2,
  type Bit,
} from "@coding-adventures/logic-gates";

import { rippleCarryAdder } from "@coding-adventures/arithmetic";

import {
  type Flags, type Trace, type MemoryAccess, type DecodedInstruction,
  MODE_USR, MODE_FIQ, MODE_IRQ, MODE_SVC,
  COND_EQ, COND_NE, COND_CS, COND_CC, COND_MI, COND_PL,
  COND_VS, COND_VC, COND_HI, COND_LS, COND_GE, COND_LT,
  COND_GT, COND_LE, COND_AL, COND_NV,
  OP_MOV, OP_MVN, OP_AND, OP_EOR, OP_ORR, OP_BIC,
  OP_ADD, OP_ADC, OP_SUB, OP_SBC, OP_RSB, OP_RSC,
  OP_TST, OP_TEQ, OP_CMP, OP_CMN,
  FLAG_I, FLAG_F, FLAG_N, FLAG_Z, FLAG_C, FLAG_V,
  PC_MASK, MODE_MASK, HALT_SWI,
  INST_DATA_PROCESSING, INST_LOAD_STORE, INST_BLOCK_TRANSFER,
  INST_BRANCH, INST_SWI, INST_COPROCESSOR, INST_UNDEFINED,
  decode, disassemble, condString, isTestOp,
  barrelShift as behavioralBarrelShift,
} from "@coding-adventures/arm1-simulator";

// Re-export types needed by consumers
export type { Flags, Trace, MemoryAccess, DecodedInstruction };

// =========================================================================
// Bit Conversion Helpers
// =========================================================================
//
// Converts between integer values and bit arrays (LSB-first). The ARM1
// uses 32-bit data paths, so most conversions use width=32.
//
// LSB-first ordering matches how ripple-carry adders process data:
// bit 0 feeds the first full adder, bit 1 feeds the second, etc.
//
//   intToBits(5, 32) -> [1, 0, 1, 0, 0, 0, ..., 0]  (32 elements)

/** Converts a uint32 to a Bit array of given width (LSB first). */
export function intToBits(value: number, width: number): Bit[] {
  value = value >>> 0;
  const bits: Bit[] = new Array(width);
  for (let i = 0; i < width; i++) {
    bits[i] = ((value >>> i) & 1) as Bit;
  }
  return bits;
}

/** Converts a Bit array (LSB first) to a uint32. */
export function bitsToInt(bits: Bit[]): number {
  let result = 0;
  for (let i = 0; i < bits.length && i < 32; i++) {
    result = (result | (bits[i] << i)) >>> 0;
  }
  return result;
}

// =========================================================================
// Gate-Level ALU Result
// =========================================================================

export interface GateALUResult {
  result: Bit[];  // 32-bit result (LSB first)
  N: Bit;         // Negative flag (bit 31 of result)
  Z: Bit;         // Zero flag (1 if result is all zeros)
  C: Bit;         // Carry flag
  V: Bit;         // Overflow flag
}

// =========================================================================
// Gate-Level ALU
// =========================================================================
//
// Every operation routes through actual gate function calls:
//   - Arithmetic: rippleCarryAdder (32 full adders -> 160+ gate calls)
//   - Logical: AND/OR/XOR/NOT applied to each of 32 bits (32-64 gate calls)

/**
 * Performs one of the 16 ALU operations using gate-level logic.
 *
 * Parameters:
 *   opcode        - 4-bit ALU operation (0=AND ... 15=MVN)
 *   a             - first operand (Rn value), 32 bits LSB-first
 *   b             - second operand (after barrel shifter), 32 bits LSB-first
 *   carryIn       - current carry flag (0 or 1)
 *   shifterCarry  - carry from barrel shifter (0 or 1)
 *   oldV          - current overflow flag (0 or 1)
 */
export function gateALUExecute(
  opcode: number,
  a: Bit[],
  b: Bit[],
  carryIn: Bit,
  shifterCarry: Bit,
  oldV: Bit,
): GateALUResult {
  let result: Bit[];
  let carry: Bit;
  let overflow: Bit;

  switch (opcode) {
    // -- Logical operations --
    // Each bit processed independently through gate functions.
    // C flag comes from barrel shifter, V flag preserved.

    case OP_AND: case OP_TST:
      result = bitwiseGate(a, b, AND);
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_EOR: case OP_TEQ:
      result = bitwiseGate(a, b, XOR);
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_ORR:
      result = bitwiseGate(a, b, OR);
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_MOV:
      result = [...b];
      carry = shifterCarry;
      overflow = oldV;
      break;

    case OP_BIC: {
      // BIC = AND(a, NOT(b))
      const notB = bitwiseNot(b);
      result = bitwiseGate(a, notB, AND);
      carry = shifterCarry;
      overflow = oldV;
      break;
    }

    case OP_MVN:
      result = bitwiseNot(b);
      carry = shifterCarry;
      overflow = oldV;
      break;

    // -- Arithmetic operations --
    // All route through the ripple-carry adder (32 full adders chained).

    case OP_ADD: case OP_CMN: {
      // A + B
      const [sum, cout] = rippleCarryAdder(a, b, 0 as Bit);
      result = sum;
      carry = cout;
      overflow = computeOverflow(a, b, sum);
      break;
    }

    case OP_ADC: {
      // A + B + C
      const [sum, cout] = rippleCarryAdder(a, b, carryIn);
      result = sum;
      carry = cout;
      overflow = computeOverflow(a, b, sum);
      break;
    }

    case OP_SUB: case OP_CMP: {
      // A - B = A + NOT(B) + 1
      const notB = bitwiseNot(b);
      const [sum, cout] = rippleCarryAdder(a, notB, 1 as Bit);
      result = sum;
      carry = cout;
      overflow = computeOverflow(a, notB, sum);
      break;
    }

    case OP_SBC: {
      // A - B - !C = A + NOT(B) + C
      const notB = bitwiseNot(b);
      const [sum, cout] = rippleCarryAdder(a, notB, carryIn);
      result = sum;
      carry = cout;
      overflow = computeOverflow(a, notB, sum);
      break;
    }

    case OP_RSB: {
      // B - A = B + NOT(A) + 1
      const notA = bitwiseNot(a);
      const [sum, cout] = rippleCarryAdder(b, notA, 1 as Bit);
      result = sum;
      carry = cout;
      overflow = computeOverflow(b, notA, sum);
      break;
    }

    case OP_RSC: {
      // B - A - !C = B + NOT(A) + C
      const notA = bitwiseNot(a);
      const [sum, cout] = rippleCarryAdder(b, notA, carryIn);
      result = sum;
      carry = cout;
      overflow = computeOverflow(b, notA, sum);
      break;
    }

    default:
      result = new Array(32).fill(0) as Bit[];
      carry = 0 as Bit;
      overflow = 0 as Bit;
  }

  // Compute N and Z flags from result bits using gates
  const n = result[31];

  // Zero flag: NOR of all 32 result bits
  // Z = 1 only when all bits are 0
  const z = computeZero(result);

  return { result, N: n, Z: z, C: carry, V: overflow };
}

/**
 * Applies a 2-input gate function to each bit pair.
 * This is how the real ARM1 does AND, OR, XOR -- 32 gate instances in parallel.
 */
function bitwiseGate(a: Bit[], b: Bit[], gate: (x: Bit, y: Bit) => Bit): Bit[] {
  const result: Bit[] = new Array(a.length);
  for (let i = 0; i < a.length; i++) {
    result[i] = gate(a[i], b[i]);
  }
  return result;
}

/** Applies NOT to each bit. */
function bitwiseNot(bits: Bit[]): Bit[] {
  const result: Bit[] = new Array(bits.length);
  for (let i = 0; i < bits.length; i++) {
    result[i] = NOT(bits[i]);
  }
  return result;
}

/**
 * Checks if all 32 bits are zero using a tree of OR/NOR gates.
 * In hardware, this is a tree reduction: OR pairs, then OR the results, etc.
 */
function computeZero(bits: Bit[]): Bit {
  let combined: Bit = bits[0];
  for (let i = 1; i < bits.length; i++) {
    combined = OR(combined, bits[i]);
  }
  return NOT(combined);
}

/**
 * Detects signed overflow using XOR gates.
 * Overflow occurs when both inputs have the same sign but the result differs.
 * V = (a[31] XOR result[31]) AND (b[31] XOR result[31])
 */
function computeOverflow(a: Bit[], b: Bit[], result: Bit[]): Bit {
  const xor1 = XOR(a[31], result[31]);
  const xor2 = XOR(b[31], result[31]);
  return AND(xor1, xor2);
}

// =========================================================================
// Gate-Level Barrel Shifter
// =========================================================================
//
// On the real ARM1, the barrel shifter was a 32x32 crossbar network of
// pass transistors. We model it with a 5-level tree of mux2 gates.
//
// Each level handles one bit of the shift amount:
//   Level 0: shift by 0 or 1   (controlled by amount bit 0)
//   Level 1: shift by 0 or 2   (controlled by amount bit 1)
//   Level 2: shift by 0 or 4   (controlled by amount bit 2)
//   Level 3: shift by 0 or 8   (controlled by amount bit 3)
//   Level 4: shift by 0 or 16  (controlled by amount bit 4)
//
// Total: 5 * 32 = 160 mux2 gates per shift.

/**
 * Performs a shift operation on a 32-bit value using a tree of multiplexer gates.
 * Returns [shifted value (32 bits), carry-out (1 bit)].
 */
export function gateBarrelShift(
  value: Bit[],
  shiftType: number,
  amount: number,
  carryIn: Bit,
  byRegister: boolean,
): [Bit[], Bit] {
  if (byRegister && amount === 0) {
    return [[...value], carryIn];
  }

  switch (shiftType) {
    case 0: return gateLSL(value, amount, carryIn, byRegister);
    case 1: return gateLSR(value, amount, carryIn, byRegister);
    case 2: return gateASR(value, amount, carryIn, byRegister);
    case 3: return gateROR(value, amount, carryIn, byRegister);
    default: return [[...value], carryIn];
  }
}

/**
 * Logical Shift Left using a 5-level multiplexer tree.
 *
 * For LSL, each output bit i gets the input from bit (i - shiftAmount),
 * or 0 if i < shiftAmount.
 */
function gateLSL(value: Bit[], amount: number, carryIn: Bit, _byRegister: boolean): [Bit[], Bit] {
  if (amount === 0) {
    return [[...value], carryIn];
  }
  if (amount >= 32) {
    const result: Bit[] = new Array(32).fill(0) as Bit[];
    if (amount === 32) {
      return [result, value[0]];
    }
    return [result, 0 as Bit];
  }

  // Build through 5 levels of muxes
  let current: Bit[] = [...value];

  for (let level = 0; level < 5; level++) {
    const shift = 1 << level;
    const sel = ((amount >>> level) & 1) as Bit;
    const next: Bit[] = new Array(32);
    for (let i = 0; i < 32; i++) {
      const shifted: Bit = (i >= shift) ? current[i - shift] : (0 as Bit);
      next[i] = mux2(current[i], shifted, sel);
    }
    current = next;
  }

  // Carry = last bit shifted out = bit (32 - amount) of original
  const carry: Bit = (amount > 0 && amount <= 32) ? value[32 - amount] : carryIn;
  return [current, carry];
}

/** Logical Shift Right using mux tree. */
function gateLSR(value: Bit[], amount: number, carryIn: Bit, byRegister: boolean): [Bit[], Bit] {
  if (amount === 0 && !byRegister) {
    // Immediate LSR #0 encodes LSR #32
    return [new Array(32).fill(0) as Bit[], value[31]];
  }
  if (amount === 0) {
    return [[...value], carryIn];
  }
  if (amount >= 32) {
    const result: Bit[] = new Array(32).fill(0) as Bit[];
    if (amount === 32) {
      return [result, value[31]];
    }
    return [result, 0 as Bit];
  }

  let current: Bit[] = [...value];

  for (let level = 0; level < 5; level++) {
    const shift = 1 << level;
    const sel = ((amount >>> level) & 1) as Bit;
    const next: Bit[] = new Array(32);
    for (let i = 0; i < 32; i++) {
      const shifted: Bit = (i + shift < 32) ? current[i + shift] : (0 as Bit);
      next[i] = mux2(current[i], shifted, sel);
    }
    current = next;
  }

  return [current, value[amount - 1]];
}

/** Arithmetic Shift Right (sign-extending) using mux tree. */
function gateASR(value: Bit[], amount: number, carryIn: Bit, byRegister: boolean): [Bit[], Bit] {
  const signBit = value[31];

  if (amount === 0 && !byRegister) {
    // Immediate ASR #0 encodes ASR #32
    const result: Bit[] = new Array(32).fill(signBit) as Bit[];
    return [result, signBit];
  }
  if (amount === 0) {
    return [[...value], carryIn];
  }
  if (amount >= 32) {
    const result: Bit[] = new Array(32).fill(signBit) as Bit[];
    return [result, signBit];
  }

  let current: Bit[] = [...value];

  for (let level = 0; level < 5; level++) {
    const shift = 1 << level;
    const sel = ((amount >>> level) & 1) as Bit;
    const next: Bit[] = new Array(32);
    for (let i = 0; i < 32; i++) {
      const shifted: Bit = (i + shift < 32) ? current[i + shift] : signBit;
      next[i] = mux2(current[i], shifted, sel);
    }
    current = next;
  }

  return [current, value[amount - 1]];
}

/** Rotate Right using mux tree. */
function gateROR(value: Bit[], amount: number, carryIn: Bit, byRegister: boolean): [Bit[], Bit] {
  if (amount === 0 && !byRegister) {
    // RRX: 33-bit rotate through carry
    const result: Bit[] = new Array(32);
    for (let i = 0; i < 31; i++) {
      result[i] = value[i + 1];
    }
    result[31] = carryIn;  // Old carry becomes MSB
    const carry = value[0];  // Old LSB becomes new carry
    return [result, carry];
  }
  if (amount === 0) {
    return [[...value], carryIn];
  }

  // Normalize to 0-31
  amount = amount & 31;
  if (amount === 0) {
    return [[...value], value[31]];
  }

  let current: Bit[] = [...value];

  for (let level = 0; level < 5; level++) {
    const shift = 1 << level;
    const sel = ((amount >>> level) & 1) as Bit;
    const next: Bit[] = new Array(32);
    for (let i = 0; i < 32; i++) {
      // Rotate: bits wrap around
      const shifted = current[(i + shift) % 32];
      next[i] = mux2(current[i], shifted, sel);
    }
    current = next;
  }

  // Carry = MSB of result
  return [current, current[31]];
}

/** Decodes a rotated immediate using gate-level rotation. */
function gateDecodeImmediate(imm8: number, rotate: number): [Bit[], Bit] {
  const bits = intToBits(imm8, 32);
  const rotateAmount = rotate * 2;
  if (rotateAmount === 0) {
    return [bits, 0 as Bit];
  }
  return gateROR(bits, rotateAmount, 0 as Bit, false);
}

// =========================================================================
// ARM1 Gate-Level CPU
// =========================================================================

export class ARM1GateLevel {
  /**
   * Register file: stored as bit arrays (27 x 32 flip-flop states).
   * Each register is a 32-element Bit array (LSB first).
   */
  private regs: Bit[][] = [];

  /** Memory (not gate-level - would need millions of flip-flops). */
  private memory: Uint8Array;

  /** Has the CPU been halted? */
  private _halted: boolean = false;

  /** Gate operation count tracking. */
  private _gateOps: number = 0;

  constructor(memorySize: number = 1024 * 1024) {
    if (memorySize <= 0) memorySize = 1024 * 1024;
    this.memory = new Uint8Array(memorySize);
    // Initialize 27 registers as 32-bit arrays
    this.regs = [];
    for (let i = 0; i < 27; i++) {
      this.regs.push(new Array(32).fill(0) as Bit[]);
    }
    this.reset();
  }

  /** Reset restores the CPU to power-on state. */
  reset(): void {
    for (let i = 0; i < 27; i++) {
      this.regs[i] = new Array(32).fill(0) as Bit[];
    }
    // Set R15: SVC mode, IRQ/FIQ disabled
    const r15val = (FLAG_I | FLAG_F | MODE_SVC) >>> 0;
    this.regs[15] = intToBits(r15val, 32);
    this._halted = false;
    this._gateOps = 0;
  }

  // =====================================================================
  // Register access (gate-level)
  // =====================================================================

  private readReg(index: number): number {
    const phys = this.physicalReg(index);
    return bitsToInt(this.regs[phys]);
  }

  private writeReg(index: number, value: number): void {
    const phys = this.physicalReg(index);
    this.regs[phys] = intToBits(value >>> 0, 32);
  }

  private physicalReg(index: number): number {
    const mode = bitsToInt(this.regs[15]) & MODE_MASK;
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

  private readRegBits(index: number): Bit[] {
    const phys = this.physicalReg(index);
    return [...this.regs[phys]];
  }

  /** Current program counter (26-bit address). */
  get pc(): number {
    return bitsToInt(this.regs[15]) & PC_MASK;
  }

  set pc(addr: number) {
    let r15 = bitsToInt(this.regs[15]);
    r15 = ((r15 & ~PC_MASK) | (addr & PC_MASK)) >>> 0;
    this.regs[15] = intToBits(r15, 32);
  }

  /** Current condition flags. */
  get flags(): Flags {
    const r15 = this.regs[15];
    return {
      N: r15[31] === 1,
      Z: r15[30] === 1,
      C: r15[29] === 1,
      V: r15[28] === 1,
    };
  }

  private setFlags(n: Bit, z: Bit, c: Bit, v: Bit): void {
    this.regs[15][31] = n;
    this.regs[15][30] = z;
    this.regs[15][29] = c;
    this.regs[15][28] = v;
  }

  /** Current processor mode. */
  get mode(): number {
    return bitsToInt(this.regs[15]) & MODE_MASK;
  }

  /** Whether the CPU has been halted. */
  get halted(): boolean {
    return this._halted;
  }

  /** Total number of gate operations performed. */
  get gateOps(): number {
    return this._gateOps;
  }

  // =====================================================================
  // Memory (same as behavioral - not gate-level)
  // =====================================================================

  readWord(addr: number): number {
    addr = addr & PC_MASK;
    const a = addr & ~3;
    if (a + 3 >= this.memory.length) return 0;
    return (
      this.memory[a] |
      (this.memory[a + 1] << 8) |
      (this.memory[a + 2] << 16) |
      (this.memory[a + 3] << 24)
    ) >>> 0;
  }

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

  readByte(addr: number): number {
    addr = addr & PC_MASK;
    if (addr >= this.memory.length) return 0;
    return this.memory[addr];
  }

  writeByte(addr: number, value: number): void {
    addr = addr & PC_MASK;
    if (addr >= this.memory.length) return;
    this.memory[addr] = value & 0xFF;
  }

  loadProgram(code: Uint8Array | number[], startAddr: number = 0): void {
    for (let i = 0; i < code.length; i++) {
      const addr = startAddr + i;
      if (addr < this.memory.length) {
        this.memory[addr] = code[i];
      }
    }
  }

  // =====================================================================
  // Condition evaluation (gate-level)
  // =====================================================================

  private evaluateCondition(cond: number, flags: Flags): boolean {
    const n: Bit = flags.N ? 1 : 0;
    const z: Bit = flags.Z ? 1 : 0;
    const c: Bit = flags.C ? 1 : 0;
    const v: Bit = flags.V ? 1 : 0;

    this._gateOps += 4;

    switch (cond) {
      case COND_EQ: return z === 1;
      case COND_NE: return NOT(z) === 1;
      case COND_CS: return c === 1;
      case COND_CC: return NOT(c) === 1;
      case COND_MI: return n === 1;
      case COND_PL: return NOT(n) === 1;
      case COND_VS: return v === 1;
      case COND_VC: return NOT(v) === 1;
      case COND_HI: return AND(c, NOT(z)) === 1;
      case COND_LS: return OR(NOT(c), z) === 1;
      case COND_GE: return XNOR(n, v) === 1;
      case COND_LT: return XOR(n, v) === 1;
      case COND_GT: return AND(NOT(z), XNOR(n, v)) === 1;
      case COND_LE: return OR(z, XOR(n, v)) === 1;
      case COND_AL: return true;
      case COND_NV: return false;
      default: return false;
    }
  }

  // =====================================================================
  // Execution
  // =====================================================================

  /** Executes one instruction and returns a trace. */
  step(): Trace {
    const pcVal = this.pc;
    const regsBefore: number[] = [];
    for (let i = 0; i < 16; i++) regsBefore.push(this.readReg(i));
    const flagsBefore = this.flags;

    const instruction = this.readWord(pcVal);
    const decoded = decode(instruction);
    const condMet = this.evaluateCondition(decoded.cond, flagsBefore);

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

    const regsAfter: number[] = [];
    for (let i = 0; i < 16; i++) regsAfter.push(this.readReg(i));
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
  // Data Processing (gate-level)
  // =====================================================================

  private executeDataProcessing(d: DecodedInstruction, trace: Trace): void {
    // Read Rn as bits
    let aBits: Bit[];
    if (d.opcode !== OP_MOV && d.opcode !== OP_MVN) {
      aBits = this.readRegBitsForExec(d.rn);
    } else {
      aBits = new Array(32).fill(0) as Bit[];
    }

    // Get Operand2 through gate-level barrel shifter
    let bBits: Bit[];
    let shifterCarry: Bit;
    const flags = this.flags;
    const flagC: Bit = flags.C ? 1 : 0;
    const flagV: Bit = flags.V ? 1 : 0;

    if (d.immediate) {
      [bBits, shifterCarry] = gateDecodeImmediate(d.imm8, d.rotate);
      if (d.rotate === 0) {
        shifterCarry = flagC;
      }
    } else {
      const rmBits = this.readRegBitsForExec(d.rm);
      let shiftAmount: number;
      if (d.shiftByReg) {
        shiftAmount = this.readReg(d.rs) & 0xFF;
      } else {
        shiftAmount = d.shiftImm;
      }
      [bBits, shifterCarry] = gateBarrelShift(rmBits, d.shiftType, shiftAmount, flagC, d.shiftByReg);
    }

    // Execute ALU operation through gate-level ALU
    const result = gateALUExecute(d.opcode, aBits, bBits, flagC, shifterCarry, flagV);
    this._gateOps += 200;

    const resultVal = bitsToInt(result.result);

    // Write result
    if (!isTestOp(d.opcode)) {
      if (d.rd === 15) {
        if (d.s) {
          this.regs[15] = intToBits(resultVal, 32);
        } else {
          this.pc = resultVal & PC_MASK;
        }
      } else {
        this.writeReg(d.rd, resultVal);
      }
    }

    // Update flags
    if (d.s && d.rd !== 15) {
      this.setFlags(result.N, result.Z, result.C, result.V);
    }
    if (isTestOp(d.opcode)) {
      this.setFlags(result.N, result.Z, result.C, result.V);
    }
  }

  private readRegBitsForExec(index: number): Bit[] {
    if (index === 15) {
      const val = (bitsToInt(this.regs[15]) + 4) >>> 0;
      return intToBits(val, 32);
    }
    return this.readRegBits(index);
  }

  private readRegForExec(index: number): number {
    if (index === 15) {
      return (bitsToInt(this.regs[15]) + 4) >>> 0;
    }
    return this.readReg(index);
  }

  // =====================================================================
  // Load/Store, Block Transfer, Branch, SWI
  // =====================================================================

  private executeLoadStore(d: DecodedInstruction, trace: Trace): void {
    let offset: number;
    if (d.immediate) {
      let rmVal = this.readRegForExec(d.rm);
      if (d.shiftImm !== 0) {
        const rmBits = intToBits(rmVal, 32);
        const flagC: Bit = this.flags.C ? 1 : 0;
        const [shifted] = gateBarrelShift(rmBits, d.shiftType, d.shiftImm, flagC, false);
        rmVal = bitsToInt(shifted);
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
        const rotation = (transferAddr & 3) * 8;
        if (rotation !== 0) {
          value = ((value >>> rotation) | (value << (32 - rotation))) >>> 0;
        }
      }
      trace.memoryReads.push({ address: transferAddr, value });
      if (d.rd === 15) {
        this.regs[15] = intToBits(value, 32);
      } else {
        this.writeReg(d.rd, value);
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

    if (d.writeBack || !d.preIndex) {
      if (d.rn !== 15) {
        this.writeReg(d.rn, addr);
      }
    }
  }

  private executeBlockTransfer(d: DecodedInstruction, trace: Trace): void {
    const base = this.readReg(d.rn);
    let count = 0;
    for (let i = 0; i < 16; i++) {
      if ((d.registerList >>> i) & 1) count++;
    }
    if (count === 0) return;

    let startAddr: number;
    if (!d.preIndex && d.up) {
      startAddr = base;
    } else if (d.preIndex && d.up) {
      startAddr = (base + 4) >>> 0;
    } else if (!d.preIndex && !d.up) {
      startAddr = (base - (count * 4) + 4) >>> 0;
    } else {
      startAddr = (base - (count * 4)) >>> 0;
    }

    let addr = startAddr;
    for (let i = 0; i < 16; i++) {
      if (!((d.registerList >>> i) & 1)) continue;

      if (d.load) {
        const value = this.readWord(addr);
        trace.memoryReads.push({ address: addr, value });
        if (i === 15) {
          this.regs[15] = intToBits(value, 32);
        } else {
          this.writeReg(i, value);
        }
      } else {
        let value: number;
        if (i === 15) {
          value = (bitsToInt(this.regs[15]) + 4) >>> 0;
        } else {
          value = this.readReg(i);
        }
        this.writeWord(addr, value);
        trace.memoryWrites.push({ address: addr, value });
      }
      addr = (addr + 4) >>> 0;
    }

    if (d.writeBack) {
      let newBase: number;
      if (d.up) {
        newBase = (base + (count * 4)) >>> 0;
      } else {
        newBase = (base - (count * 4)) >>> 0;
      }
      this.writeReg(d.rn, newBase);
    }
  }

  private executeBranch(d: DecodedInstruction, trace: Trace): void {
    const branchBase = (this.pc + 4) >>> 0;
    if (d.link) {
      const returnAddr = bitsToInt(this.regs[15]);
      this.writeReg(14, returnAddr);
    }
    const target = (branchBase + d.branchOffset) >>> 0;
    this.pc = target & PC_MASK;
  }

  private executeSWI(d: DecodedInstruction, _trace: Trace): void {
    if (d.swiComment === HALT_SWI) {
      this._halted = true;
      return;
    }
    const r15val = bitsToInt(this.regs[15]);
    this.regs[25] = [...this.regs[15]];
    this.regs[26] = [...this.regs[15]];

    let newR15 = ((r15val & (~MODE_MASK >>> 0)) | MODE_SVC) >>> 0;
    newR15 = (newR15 | FLAG_I) >>> 0;
    this.regs[15] = intToBits(newR15, 32);
    this.pc = 0x08;
  }

  private trapUndefined(_instrAddr: number): void {
    this.regs[26] = [...this.regs[15]];
    const r15val = bitsToInt(this.regs[15]);
    let newR15 = ((r15val & (~MODE_MASK >>> 0)) | MODE_SVC) >>> 0;
    newR15 = (newR15 | FLAG_I) >>> 0;
    this.regs[15] = intToBits(newR15, 32);
    this.pc = 0x04;
  }
}
