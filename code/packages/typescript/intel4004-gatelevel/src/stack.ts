/**
 * Hardware call stack -- 3 levels of 12-bit return addresses.
 *
 * === The 4004's stack ===
 *
 * The Intel 4004 has a 3-level hardware call stack. This is NOT a
 * software stack in RAM -- it's three physical 12-bit registers plus
 * a 2-bit circular pointer, all built from D flip-flops.
 *
 * Why only 3 levels? The 4004 was designed for calculators, which had
 * simple call structures. Three levels of subroutine nesting was enough
 * for the Busicom 141-PF calculator's firmware.
 *
 * === Silent overflow ===
 *
 * When you push a 4th address, the stack wraps silently -- the oldest
 * return address is overwritten. There is no stack overflow exception.
 * This matches the real hardware behavior. The 4004's designers saved
 * transistors by not including overflow detection.
 */

import { register, type Bit, type FlipFlopState } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/**
 * 3-level x 12-bit hardware call stack.
 *
 * Built from 3 x 12 = 36 D flip-flops for storage, plus a 2-bit
 * pointer that wraps modulo 3.
 */
export class HardwareStack {
  /** Flip-flop states for each of the 3 stack levels. */
  _levels: FlipFlopState[][];
  /** Circular pointer (0, 1, or 2). */
  _pointer: number;

  constructor() {
    /** Initialize stack with 3 empty slots and pointer at 0. */
    this._levels = [];
    const zeros: Bit[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    for (let i = 0; i < 3; i++) {
      const [, state1] = register(zeros, 0 as Bit, undefined, 12);
      const [, state2] = register(zeros, 1 as Bit, state1, 12);
      this._levels.push(state2);
    }
    this._pointer = 0;
  }

  /**
   * Push a return address. Wraps silently on overflow.
   *
   * In real hardware: the pointer selects which of the 3 registers
   * to write, then the pointer increments mod 3.
   */
  push(address: number): void {
    const bits = intToBits(address & 0xfff, 12);
    const [, state1] = register(bits, 0 as Bit, this._levels[this._pointer], 12);
    const [, state2] = register(bits, 1 as Bit, state1, 12);
    this._levels[this._pointer] = state2;
    this._pointer = (this._pointer + 1) % 3;
  }

  /**
   * Pop and return the top address.
   *
   * Decrements pointer mod 3, then reads that register.
   */
  pop(): number {
    this._pointer = ((this._pointer - 1) % 3 + 3) % 3;
    const zeros: Bit[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    const [output] = register(zeros, 0 as Bit, this._levels[this._pointer], 12);
    return bitsToInt(output);
  }

  /** Reset all stack levels to 0 and pointer to 0. */
  reset(): void {
    const zeros: Bit[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    for (let i = 0; i < 3; i++) {
      const [, state1] = register(zeros, 0 as Bit, undefined, 12);
      const [, state2] = register(zeros, 1 as Bit, state1, 12);
      this._levels[i] = state2;
    }
    this._pointer = 0;
  }

  /** Current pointer position (not true depth, since we wrap). */
  get depth(): number {
    return this._pointer;
  }

  /** 3 x 12-bit registers (216 gates) + pointer logic (~10 gates). */
  get gateCount(): number {
    return 226;
  }
}
