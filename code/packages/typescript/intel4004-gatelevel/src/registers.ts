/**
 * Register file -- 16 x 4-bit registers built from D flip-flops.
 *
 * === How registers work in hardware ===
 *
 * A register is a group of D flip-flops that share a clock signal. Each
 * flip-flop stores one bit. A 4-bit register has 4 flip-flops. The Intel
 * 4004 has 16 such registers (R0-R15), for a total of 64 flip-flops just
 * for the register file.
 *
 * In this simulation, each register call goes through:
 *     data bits -> D flip-flop x 4 -> output bits
 *
 * The flip-flops are edge-triggered: they capture new data on the rising
 * edge of the clock. Between edges, the stored value is stable.
 *
 * === Register pairs ===
 *
 * The 4004 organizes its 16 registers into 8 pairs:
 *     P0 = R0:R1, P1 = R2:R3, ..., P7 = R14:R15
 *
 * A register pair holds an 8-bit value (high nibble in even register,
 * low nibble in odd register). Pairs are used for:
 *     - FIM: load 8-bit immediate
 *     - SRC: set RAM address
 *     - FIN: indirect ROM read
 *     - JIN: indirect jump
 *
 * === Accumulator ===
 *
 * The accumulator is a separate 4-bit register, not part of the R0-R15
 * file. It has its own dedicated flip-flops and is connected directly to
 * the ALU's output bus.
 */

import { register, type Bit, type FlipFlopState } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/**
 * 16 x 4-bit register file built from D flip-flops.
 *
 * Each of the 16 registers is a group of 4 D flip-flops from the
 * logic_gates sequential module. Reading and writing go through
 * actual flip-flop state transitions.
 */
export class RegisterFile {
  private _states: FlipFlopState[][];

  constructor() {
    /** Initialize 16 registers, each with 4-bit flip-flop state. */
    this._states = [];
    for (let i = 0; i < 16; i++) {
      // Initialize state by clocking zeros through
      const zeros: Bit[] = [0, 0, 0, 0];
      const [, state1] = register(zeros, 0 as Bit, undefined, 4);
      const [, state2] = register(zeros, 1 as Bit, state1, 4);
      this._states.push(state2);
    }
  }

  /**
   * Read a register value. Returns 4-bit integer (0-15).
   *
   * In real hardware, this would route through a 16-to-1 multiplexer
   * built from gates. We simulate the flip-flop read directly.
   */
  read(index: number): number {
    const zeros: Bit[] = [0, 0, 0, 0];
    const [output] = register(zeros, 0 as Bit, this._states[index], 4);
    return bitsToInt(output);
  }

  /**
   * Write a 4-bit value to a register.
   *
   * In real hardware: decoder selects the register, data bus presents
   * the value, clock edge latches it into the flip-flops.
   */
  write(index: number, value: number): void {
    const bits = intToBits(value & 0xf, 4);
    // Clock low (setup)
    const [, state1] = register(bits, 0 as Bit, this._states[index], 4);
    // Clock high (capture on rising edge)
    const [, state2] = register(bits, 1 as Bit, state1, 4);
    this._states[index] = state2;
  }

  /**
   * Read an 8-bit value from a register pair.
   *
   * Pair 0 = R0:R1 (R0=high nibble, R1=low nibble).
   */
  readPair(pairIndex: number): number {
    const high = this.read(pairIndex * 2);
    const low = this.read(pairIndex * 2 + 1);
    return (high << 4) | low;
  }

  /** Write an 8-bit value to a register pair. */
  writePair(pairIndex: number, value: number): void {
    this.write(pairIndex * 2, (value >> 4) & 0xf);
    this.write(pairIndex * 2 + 1, value & 0xf);
  }

  /** Reset all registers to 0 by clocking in zeros. */
  reset(): void {
    for (let i = 0; i < 16; i++) {
      this.write(i, 0);
    }
  }

  /**
   * Gate count for the register file.
   *
   * 16 registers x 4 bits x ~6 gates per D flip-flop = 384 gates.
   * Plus 4-to-16 decoder for write select: ~32 gates.
   * Plus 16-to-1 mux for read select: ~64 gates.
   * Total: ~480 gates.
   */
  get gateCount(): number {
    return 480;
  }
}

/**
 * 4-bit accumulator register built from D flip-flops.
 *
 * The accumulator is the 4004's main working register. Almost every
 * arithmetic and logic operation reads from or writes to it.
 */
export class Accumulator {
  private _state: FlipFlopState[];

  constructor() {
    /** Initialize accumulator to 0. */
    const zeros: Bit[] = [0, 0, 0, 0];
    const [, state1] = register(zeros, 0 as Bit, undefined, 4);
    const [, state2] = register(zeros, 1 as Bit, state1, 4);
    this._state = state2;
  }

  /** Read the accumulator value (0-15). */
  read(): number {
    const zeros: Bit[] = [0, 0, 0, 0];
    const [output] = register(zeros, 0 as Bit, this._state, 4);
    return bitsToInt(output);
  }

  /** Write a 4-bit value to the accumulator. */
  write(value: number): void {
    const bits = intToBits(value & 0xf, 4);
    const [, state1] = register(bits, 0 as Bit, this._state, 4);
    const [, state2] = register(bits, 1 as Bit, state1, 4);
    this._state = state2;
  }

  /** Reset to 0. */
  reset(): void {
    this.write(0);
  }

  /** 4 D flip-flops x ~6 gates = 24 gates. */
  get gateCount(): number {
    return 24;
  }
}

/**
 * 1-bit carry/borrow flag built from a D flip-flop.
 *
 * The carry flag is set by arithmetic operations and read by
 * conditional jumps and multi-digit BCD arithmetic.
 */
export class CarryFlag {
  private _state: FlipFlopState[];

  constructor() {
    /** Initialize carry to 0 (false). */
    const zero: Bit[] = [0];
    const [, state1] = register(zero, 0 as Bit, undefined, 1);
    const [, state2] = register(zero, 1 as Bit, state1, 1);
    this._state = state2;
  }

  /** Read carry flag as a boolean. */
  read(): boolean {
    const zero: Bit[] = [0];
    const [output] = register(zero, 0 as Bit, this._state, 1);
    return output[0] === 1;
  }

  /** Write carry flag. */
  write(value: boolean): void {
    const bit: Bit[] = [value ? 1 : 0];
    const [, state1] = register(bit, 0 as Bit, this._state, 1);
    const [, state2] = register(bit, 1 as Bit, state1, 1);
    this._state = state2;
  }

  /** Reset to 0. */
  reset(): void {
    this.write(false);
  }

  /** 1 D flip-flop x ~6 gates = 6 gates. */
  get gateCount(): number {
    return 6;
  }
}
