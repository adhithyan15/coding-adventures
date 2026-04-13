/**
 * 14-bit program counter for the Intel 8008 gate-level simulator.
 *
 * === What is the program counter? ===
 *
 * The program counter (PC) is a 14-bit register that holds the address of the
 * next instruction to fetch. After fetching each byte, it increments by 1.
 * For multi-byte instructions (MVI=2 bytes, JMP/CAL=3 bytes), it increments
 * once per byte fetched.
 *
 * === 14 bits, not 16 ===
 *
 * The 8008 has a 14-bit address bus (not 16-bit). This gives:
 *   2^14 = 16,384 bytes = 16 KiB of addressable memory.
 *
 * In the physical chip, the PC is built from 14 flip-flops (one per bit).
 * Two of the 8008's 8 package pins are used for address multiplexing, which is
 * why the address bus is only 14 bits despite the chip's 8-bit data path.
 *
 * === Increment via half-adder chain ===
 *
 * Adding 1 to a multi-bit register is done with a chain of half-adders.
 * A half-adder (HA) has two inputs and two outputs:
 *   - sum     = XOR(a, b)
 *   - carry   = AND(a, b)
 *
 * To increment a 14-bit register by 1:
 *   - Feed bit[0] + 1 into HA0: sum0 = XOR(bit0, 1), carry0 = AND(bit0, 1) = bit0
 *   - Feed bit[1] + carry0 into HA1: sum1 = XOR(bit1, carry0), carry1 = AND(bit1, carry0)
 *   - ... repeat for all 14 bits
 *
 * This ripples the carry from LSB to MSB, just like counting in binary.
 * The carry stops propagating as soon as it hits a 0-bit:
 *   0b1011 + 1 = 0b1100  (carry ripples through bits 0,1 then stops at bit 2)
 *
 * Gate count: 14 half-adders = 14 × (1 XOR + 1 AND) = 28 gates.
 *
 * === Load for jumps ===
 *
 * When a JMP or CALL instruction executes, the PC is loaded with a new address
 * rather than incremented. In hardware, this is done by multiplexing the HA
 * output with the target address. We model this as a direct assignment.
 *
 * === Wrap-around ===
 *
 * The 14-bit counter wraps at 0x3FFF → 0x0000 automatically because we mask
 * the carry out of the 14th bit (& 0x3FFF).
 */

import { AND, XOR, type Bit } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/**
 * 14-bit program counter built from a half-adder increment chain.
 *
 * The increment operation uses 14 half-adders chained in sequence, modeling
 * the actual hardware behavior of the 8008's program counter.
 *
 * The PC is always in the range [0, 16383] (0x0000 – 0x3FFF).
 */
export class ProgramCounter {
  /**
   * Current 14-bit PC value (stored as a plain integer for efficiency).
   * The hardware would store this as 14 D flip-flops, but we abstract
   * the flip-flop state to an integer, routing increment through gates.
   */
  private _value: number = 0;

  /**
   * Current program counter value (0–16383).
   */
  get value(): number {
    return this._value;
  }

  /**
   * Increment the PC by 1 using a 14-bit half-adder chain.
   *
   * === Half-adder increment algorithm ===
   *
   * A half-adder adds two 1-bit values without a carry-in:
   *   sum   = XOR(a, b)
   *   carry = AND(a, b)
   *
   * To increment by 1, we chain 14 HAs, where the carry-in to the first HA
   * is always 1 (the "add 1" input), and each subsequent HA receives the
   * carry-out of the previous HA.
   *
   * The process terminates early (conceptually) when carry becomes 0,
   * but we simulate all 14 bits for gate-level accuracy.
   *
   *   HA0: sum[0]  = XOR(bit0, 1)     carry0 = AND(bit0, 1) = bit0
   *   HA1: sum[1]  = XOR(bit1, c0)    carry1 = AND(bit1, c0)
   *   ...
   *   HA13: sum[13] = XOR(bit13, c12)  carry13 = AND(bit13, c12)
   *   (carry13 is discarded — overflow wraps around)
   *
   * @returns The new PC value after incrementing.
   */
  increment(): number {
    const bits = intToBits(this._value, 14);
    const newBits: Bit[] = new Array(14).fill(0) as Bit[];

    // carry-in to the first half-adder is 1 (the constant "+1")
    let carry: Bit = 1;

    for (let i = 0; i < 14; i++) {
      const a = bits[i];
      const b = carry;
      // Half-adder: sum = XOR(a, b), carry = AND(a, b)
      newBits[i] = XOR(a, b);
      carry = AND(a, b);
    }
    // The final carry (bit 14 overflow) is discarded — 14-bit wrap.

    this._value = bitsToInt(newBits);
    return this._value;
  }

  /**
   * Load a new 14-bit address into the PC.
   *
   * Called by JMP and CALL instructions to change program flow.
   * The target address is masked to 14 bits (& 0x3FFF) to prevent overflow.
   *
   * In hardware, this is a parallel load — all 14 flip-flops receive their
   * new value simultaneously on the next clock edge.
   *
   * @param address - New 14-bit address (0–16383). Values > 16383 are masked.
   */
  load(address: number): void {
    this._value = address & 0x3FFF;
  }

  /**
   * Reset the PC to 0 (power-on or CPU reset).
   *
   * The 8008 starts execution at address 0x0000 after reset.
   */
  reset(): void {
    this._value = 0;
  }
}
