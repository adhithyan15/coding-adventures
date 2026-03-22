/**
 * Program counter -- 12-bit register with increment and load.
 *
 * === The 4004's program counter ===
 *
 * The program counter (PC) holds the address of the next instruction to
 * fetch from ROM. It's 12 bits wide, addressing 4096 bytes of ROM.
 *
 * In real hardware, the PC is:
 * - A 12-bit register (12 D flip-flops)
 * - An incrementer (chain of half-adders for PC+1 or PC+2)
 * - A load input (for jump instructions)
 *
 * The incrementer uses half-adders chained together. To add 1:
 *     bit0 -> halfAdder(bit0, 1) -> sum0, carry
 *     bit1 -> halfAdder(bit1, carry) -> sum1, carry
 *     ...and so on for all 12 bits.
 *
 * This is simpler than a full adder chain because we're always adding
 * a constant (1 or 2), so one input is fixed.
 */

import { halfAdder } from "@coding-adventures/arithmetic";
import { register, type Bit, type FlipFlopState } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/**
 * 12-bit program counter built from flip-flops and half-adders.
 *
 * Supports:
 *   - increment(): PC += 1 (for 1-byte instructions)
 *   - increment2(): PC += 2 (for 2-byte instructions)
 *   - load(addr): PC = addr (for jumps)
 *   - read(): current PC value
 */
export class ProgramCounter {
  private _state: FlipFlopState[];

  constructor() {
    /** Initialize PC to 0. */
    const zeros: Bit[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    const [, state1] = register(zeros, 0 as Bit, undefined, 12);
    const [, state2] = register(zeros, 1 as Bit, state1, 12);
    this._state = state2;
  }

  /** Read current PC value (0-4095). */
  read(): number {
    const zeros: Bit[] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    const [output] = register(zeros, 0 as Bit, this._state, 12);
    return bitsToInt(output);
  }

  /** Load a new address into the PC (for jumps). */
  load(address: number): void {
    const bits = intToBits(address & 0xfff, 12);
    const [, state1] = register(bits, 0 as Bit, this._state, 12);
    const [, state2] = register(bits, 1 as Bit, state1, 12);
    this._state = state2;
  }

  /**
   * Increment PC by 1 using a chain of half-adders.
   *
   * This is how a real incrementer works:
   *     carry_in = 1 (we're adding 1)
   *     For each bit position:
   *         [new_bit, carry] = halfAdder(old_bit, carry)
   */
  increment(): void {
    const currentBits = intToBits(this.read(), 12);
    let carry: Bit = 1; // Adding 1
    const newBits: Bit[] = [];
    for (const bit of currentBits) {
      const [sumBit, newCarry] = halfAdder(bit, carry);
      newBits.push(sumBit);
      carry = newCarry;
    }
    this.load(bitsToInt(newBits));
  }

  /**
   * Increment PC by 2 (for 2-byte instructions).
   *
   * Two cascaded increments through the half-adder chain.
   */
  increment2(): void {
    this.increment();
    this.increment();
  }

  /** Reset PC to 0. */
  reset(): void {
    this.load(0);
  }

  /** 12-bit register (72 gates) + 12 half-adders (24 gates) = 96. */
  get gateCount(): number {
    return 96;
  }
}
