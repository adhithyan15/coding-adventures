/**
 * Register file for the Intel 8008 gate-level simulator.
 *
 * === Physical design ===
 *
 * The Intel 8008 had 7 general-purpose 8-bit registers: A, B, C, D, E, H, L.
 * Compare with the 4004's accumulator-only design — the 8008 represents a
 * fundamental shift toward register-rich architecture.
 *
 * In hardware, each register is built from 8 D flip-flops. Each flip-flop
 * stores one bit of state. The `dFlipFlop()` function from the logic-gates
 * package models this (master-slave edge-triggered design).
 *
 * Register count: 7 × 8 bits = 56 D flip-flops
 * vs 4004: 16 × 4 bits = 64 flip-flops (more registers but narrower)
 *
 * === Register encoding ===
 *
 * The 8008 uses a 3-bit field to select registers in instructions:
 *   000 = B   001 = C   010 = D   011 = E
 *   100 = H   101 = L   110 = M (memory pseudo-register, not stored here)
 *   111 = A (accumulator)
 *
 * Register index 6 (M) is NOT stored in this register file — M is a
 * memory access shorthand. The register file validates and rejects index 6.
 *
 * === D flip-flop write model ===
 *
 * Each write routes through `dFlipFlop()` with a rising clock edge (0 → 1).
 * We model this as two calls: clock=0 (master absorbs data), then clock=1
 * (slave outputs captured value). The result is the gate-level simulated
 * stored value.
 *
 * State is maintained per flip-flop as `FlipFlopState` objects.
 */

import { dFlipFlop, type FlipFlopState, type Bit } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";
import type { GateFlags } from "./alu.js";

/** Create 8 fresh flip-flop states for an 8-bit register. */
function freshFFStates(): (FlipFlopState | undefined)[] {
  return new Array(8).fill(undefined);
}

/**
 * Write a value to an 8-bit register via D flip-flop gate simulation.
 *
 * Models a rising clock edge (clock 0 → 1):
 * 1. Present data with clock=0: master latch absorbs each bit.
 * 2. Raise clock to 1: slave latch outputs the captured bit.
 *
 * The Q output of each slave flip-flop is the stored bit.
 *
 * @param bits       - 8 input bits (LSB first) to write.
 * @param prevStates - Previous FlipFlopState for each of the 8 flip-flops.
 * @returns [outputBits, newStates] — the stored bit values and updated state.
 */
function writeRegister(
  bits: Bit[],
  prevStates: (FlipFlopState | undefined)[],
): [Bit[], FlipFlopState[]] {
  const outputBits: Bit[] = [];
  const newStates: FlipFlopState[] = [];

  for (let i = 0; i < 8; i++) {
    // Rising edge simulation:
    // Step 1: clock=0 → master absorbs data[i]
    const [, , masterState] = dFlipFlop(bits[i], 0, prevStates[i]);
    // Step 2: clock=1 → slave outputs master's captured value
    const [q, , slaveState] = dFlipFlop(bits[i], 1, masterState);
    outputBits.push(q);
    newStates.push(slaveState);
  }

  return [outputBits, newStates];
}

/**
 * 7-register file for the Intel 8008 gate-level simulator.
 *
 * Models: A (index 7), B (0), C (1), D (2), E (3), H (4), L (5).
 * Index 6 is M (pseudo-register) — this file throws if you try to use it.
 *
 * Each register consists of 8 D flip-flops (modeled via `dFlipFlop()`).
 * The flip-flop state is maintained across calls to correctly simulate
 * the master-slave edge-triggered behavior.
 *
 * === Read behavior ===
 *
 * The read path in hardware is combinational — D flip-flops continuously
 * drive their stored Q output to the register file output bus. We model
 * this as a direct read from the stored integer value (no gates needed
 * for reading, just wiring).
 *
 * === Write behavior ===
 *
 * Each write simulates a rising clock edge through `dFlipFlop()`:
 *   clock=0 (master transparent) → clock=1 (slave outputs captured value)
 */
export class RegisterFile {
  /**
   * Internal register storage: 8 entries (index 0–7).
   * Index 6 is unused (M pseudo-register).
   * Each value is 0–255 (8 bits).
   */
  private regs: number[] = new Array(8).fill(0);

  /**
   * Flip-flop state for each register's 8 bits.
   * Indexed as ffStates[regIndex][bitIndex].
   */
  private ffStates: (FlipFlopState | undefined)[][] = Array.from(
    { length: 8 },
    () => freshFFStates(),
  );

  /**
   * Read an 8-bit register value by index.
   *
   * Combinational path: flip-flops output Q continuously.
   *
   * @param index - Register index 0–7 (0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 7=A).
   * @returns 8-bit integer (0–255).
   * @throws Error if index is 6 (M pseudo-register).
   */
  read(index: number): number {
    if (index === 6) {
      throw new Error("Register index 6 is M (memory pseudo-register). Resolve to a memory address first.");
    }
    if (index < 0 || index > 7) {
      throw new RangeError(`Register index must be 0–7, got ${index}`);
    }
    return this.regs[index];
  }

  /**
   * Write an 8-bit value to a register via D flip-flop gate simulation.
   *
   * Models a rising clock edge: master absorbs data, slave outputs it.
   * The output of each slave flip-flop is stored as the register value.
   *
   * @param index - Register index 0–7 (index 6 throws).
   * @param value - 8-bit integer (0–255).
   */
  write(index: number, value: number): void {
    if (index === 6) {
      throw new Error("Register index 6 is M (memory pseudo-register). Write to memory directly.");
    }
    if (index < 0 || index > 7) {
      throw new RangeError(`Register index must be 0–7, got ${index}`);
    }
    const bits = intToBits(value & 0xFF, 8);
    const [outputBits, newStates] = writeRegister(bits, this.ffStates[index]);
    this.ffStates[index] = newStates;
    this.regs[index] = bitsToInt(outputBits);
  }

  /** Accumulator (register A = index 7). */
  get a(): number { return this.regs[7]; }
  set a(value: number) { this.write(7, value); }

  /** Register B (index 0). */
  get b(): number { return this.regs[0]; }
  set b(value: number) { this.write(0, value); }

  /** Register C (index 1). */
  get c(): number { return this.regs[1]; }
  set c(value: number) { this.write(1, value); }

  /** Register D (index 2). */
  get d(): number { return this.regs[2]; }
  set d(value: number) { this.write(2, value); }

  /** Register E (index 3). */
  get e(): number { return this.regs[3]; }
  set e(value: number) { this.write(3, value); }

  /** Register H (index 4) — high byte of memory address pair. */
  get h(): number { return this.regs[4]; }
  set h(value: number) { this.write(4, value); }

  /** Register L (index 5) — low byte of memory address pair. */
  get l(): number { return this.regs[5]; }
  set l(value: number) { this.write(5, value); }

  /**
   * 14-bit H:L address formed by combining H and L.
   *
   * address = (H & 0x3F) << 8 | L
   *         = H[5:0] concatenated with L[7:0]
   *         = 14-bit address
   */
  get hlAddress(): number {
    return ((this.regs[4] & 0x3F) << 8) | this.regs[5];
  }

  /** Reset all registers and flip-flop state to 0. */
  reset(): void {
    this.regs.fill(0);
    this.ffStates = Array.from({ length: 8 }, () => freshFFStates());
  }

  /** Read all register values as raw integers (for inspection/trace). */
  snapshot(): Record<string, number> {
    return {
      a: this.regs[7],
      b: this.regs[0],
      c: this.regs[1],
      d: this.regs[2],
      e: this.regs[3],
      h: this.regs[4],
      l: this.regs[5],
    };
  }
}

/**
 * Flag register — stores the 4 Intel 8008 condition flags.
 *
 * In hardware: 4 D flip-flops, each storing one bit.
 * CY, Z, S, P are updated by ALU operations and read by conditional branches.
 *
 * Each flag write goes through a D flip-flop rising-edge simulation.
 */
export class FlagRegister {
  private carry: Bit = 0;
  private zero: Bit = 0;
  private sign: Bit = 0;
  private parity: Bit = 0;

  // Flip-flop state for each flag
  private cyFF: FlipFlopState | undefined = undefined;
  private zFF: FlipFlopState | undefined = undefined;
  private sFF: FlipFlopState | undefined = undefined;
  private pFF: FlipFlopState | undefined = undefined;

  /** Write a single bit through a D flip-flop rising edge simulation. */
  private writeBit(bit: Bit, state: FlipFlopState | undefined): [Bit, FlipFlopState] {
    const [, , masterState] = dFlipFlop(bit, 0, state);
    const [q, , slaveState] = dFlipFlop(bit, 1, masterState);
    return [q, slaveState];
  }

  /** Read current carry flag. */
  get cy(): Bit { return this.carry; }
  /** Read current zero flag. */
  get z(): Bit { return this.zero; }
  /** Read current sign flag. */
  get s(): Bit { return this.sign; }
  /** Read current parity flag (1 = even parity). */
  get p(): Bit { return this.parity; }

  /**
   * Update all four flags from a GateFlags record.
   * Each flag routes through a D flip-flop rising-edge simulation.
   */
  update(flags: GateFlags): void {
    [this.carry, this.cyFF] = this.writeBit(flags.carry, this.cyFF);
    [this.zero,  this.zFF]  = this.writeBit(flags.zero,  this.zFF);
    [this.sign,  this.sFF]  = this.writeBit(flags.sign,  this.sFF);
    [this.parity, this.pFF] = this.writeBit(flags.parity, this.pFF);
  }

  /**
   * Update carry flag only (for rotate instructions).
   * Z, S, P are not affected by rotates.
   */
  updateCarryOnly(cy: Bit): void {
    [this.carry, this.cyFF] = this.writeBit(cy, this.cyFF);
  }

  /**
   * Update Z, S, P flags only (for INR/DCR, which preserve CY).
   */
  updateWithoutCarry(flags: GateFlags): void {
    [this.zero,  this.zFF]  = this.writeBit(flags.zero,  this.zFF);
    [this.sign,  this.sFF]  = this.writeBit(flags.sign,  this.sFF);
    [this.parity, this.pFF] = this.writeBit(flags.parity, this.pFF);
  }

  /** Reset all flags and flip-flop state to 0. */
  reset(): void {
    this.carry  = 0;
    this.zero   = 0;
    this.sign   = 0;
    this.parity = 0;
    this.cyFF   = undefined;
    this.zFF    = undefined;
    this.sFF    = undefined;
    this.pFF    = undefined;
  }

  /** Get flags as a plain object for tracing. */
  snapshot(): { carry: boolean; zero: boolean; sign: boolean; parity: boolean } {
    return {
      carry:  this.carry  === 1,
      zero:   this.zero   === 1,
      sign:   this.sign   === 1,
      parity: this.parity === 1,
    };
  }
}
