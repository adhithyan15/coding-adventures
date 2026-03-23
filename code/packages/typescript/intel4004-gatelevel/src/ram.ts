/**
 * RAM -- 4 banks x 4 registers x 20 nibbles, built from flip-flops.
 *
 * === The 4004's RAM architecture ===
 *
 * The Intel 4004 used separate RAM chips (Intel 4002), each containing:
 *     - 4 registers
 *     - Each register has 16 main characters + 4 status characters
 *     - Each character is a 4-bit nibble
 *     - Total per chip: 4 x 20 x 4 = 320 bits
 *
 * The full system supports up to 4 RAM banks (4 chips), selected by the
 * DCL instruction. Within a bank, the SRC instruction sets which register
 * and character to access.
 *
 * In real hardware, each nibble is stored in 4 D flip-flops. The full
 * RAM system uses 4 x 4 x 20 x 4 = 1,280 flip-flops. We simulate this
 * using the register() function from the logic_gates package.
 *
 * === Addressing ===
 *
 * RAM is addressed in two steps:
 *     1. DCL sets the bank (0-3, from accumulator bits 0-2)
 *     2. SRC sends an 8-bit address from a register pair:
 *        - High nibble -> register index (0-3)
 *        - Low nibble -> character index (0-15)
 */

import { register, type Bit, type FlipFlopState } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/**
 * 4004 RAM: 4 banks x 4 registers x (16 main + 4 status) nibbles.
 *
 * Every nibble is stored in 4 D flip-flops from the sequential logic
 * package. Reading and writing physically route through flip-flop
 * state transitions.
 */
export class RAM {
  /** main[bank][reg][char] = flip-flop state for one nibble */
  private _main: FlipFlopState[][][][];
  /** status[bank][reg][index] = flip-flop state for one nibble */
  private _status: FlipFlopState[][][][];
  /** Output ports (one per bank, written by WMP) */
  private _output: number[];

  constructor() {
    /** Initialize all RAM to 0. */
    this._main = [];
    this._status = [];

    const zeros: Bit[] = [0, 0, 0, 0];

    for (let bank = 0; bank < 4; bank++) {
      const bankMain: FlipFlopState[][][] = [];
      const bankStatus: FlipFlopState[][][] = [];

      for (let reg = 0; reg < 4; reg++) {
        const regMain: FlipFlopState[][] = [];
        for (let char = 0; char < 16; char++) {
          const [, state1] = register(zeros, 0 as Bit, undefined, 4);
          const [, state2] = register(zeros, 1 as Bit, state1, 4);
          regMain.push(state2);
        }
        bankMain.push(regMain);

        const regStatus: FlipFlopState[][] = [];
        for (let stat = 0; stat < 4; stat++) {
          const [, state1] = register(zeros, 0 as Bit, undefined, 4);
          const [, state2] = register(zeros, 1 as Bit, state1, 4);
          regStatus.push(state2);
        }
        bankStatus.push(regStatus);
      }

      this._main.push(bankMain);
      this._status.push(bankStatus);
    }

    this._output = [0, 0, 0, 0];
  }

  /** Read a main character (4-bit nibble) from RAM. */
  readMain(bank: number, reg: number, char: number): number {
    const state = this._main[bank & 3][reg & 3][char & 0xf];
    const zeros: Bit[] = [0, 0, 0, 0];
    const [output] = register(zeros, 0 as Bit, state, 4);
    return bitsToInt(output);
  }

  /** Write a 4-bit value to a main character. */
  writeMain(bank: number, reg: number, char: number, value: number): void {
    const bits = intToBits(value & 0xf, 4);
    const state = this._main[bank & 3][reg & 3][char & 0xf];
    const [, state1] = register(bits, 0 as Bit, state, 4);
    const [, state2] = register(bits, 1 as Bit, state1, 4);
    this._main[bank & 3][reg & 3][char & 0xf] = state2;
  }

  /** Read a status character (0-3) from RAM. */
  readStatus(bank: number, reg: number, index: number): number {
    const state = this._status[bank & 3][reg & 3][index & 3];
    const zeros: Bit[] = [0, 0, 0, 0];
    const [output] = register(zeros, 0 as Bit, state, 4);
    return bitsToInt(output);
  }

  /** Write a 4-bit value to a status character. */
  writeStatus(bank: number, reg: number, index: number, value: number): void {
    const bits = intToBits(value & 0xf, 4);
    const state = this._status[bank & 3][reg & 3][index & 3];
    const [, state1] = register(bits, 0 as Bit, state, 4);
    const [, state2] = register(bits, 1 as Bit, state1, 4);
    this._status[bank & 3][reg & 3][index & 3] = state2;
  }

  /** Read a RAM output port value. */
  readOutput(bank: number): number {
    return this._output[bank & 3];
  }

  /** Write to a RAM output port (WMP instruction). */
  writeOutput(bank: number, value: number): void {
    this._output[bank & 3] = value & 0xf;
  }

  /** Reset all RAM to 0. */
  reset(): void {
    for (let bank = 0; bank < 4; bank++) {
      for (let reg = 0; reg < 4; reg++) {
        for (let char = 0; char < 16; char++) {
          this.writeMain(bank, reg, char, 0);
        }
        for (let stat = 0; stat < 4; stat++) {
          this.writeStatus(bank, reg, stat, 0);
        }
      }
      this._output[bank] = 0;
    }
  }

  /**
   * 4 banks x 4 regs x 20 nibbles x 4 bits x 6 gates/ff = 7680.
   * Plus addressing/decoding: ~200 gates.
   */
  get gateCount(): number {
    return 7880;
  }
}
