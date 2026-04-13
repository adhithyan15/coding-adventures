/**
 * 8-level push-down stack for the Intel 8008 gate-level simulator.
 *
 * === The 8008's on-chip stack ===
 *
 * The Intel 8008 was the first microprocessor to have an on-chip call stack.
 * Previous CPUs (like the 4004) had no stack at all — subroutines required
 * creative software tricks. The 8008 integrated 8 × 14-bit registers into the
 * chip specifically for subroutine support.
 *
 * Key properties:
 * - 8 entries, each 14 bits wide (holds a 14-bit address)
 * - Entry 0 is ALWAYS the current program counter
 * - Entries 1–7 hold saved return addresses (most recent first)
 * - Push (CALL): rotates entries DOWN — entry[i] ← entry[i-1], new target → entry[0]
 * - Pop  (RETURN): rotates entries UP — entry[i-1] ← entry[i]
 * - Maximum call depth: 7 (the 8th level would overwrite the oldest return address)
 *
 * === Push-down vs conventional stacks ===
 *
 * Most modern CPUs use a "stack pointer" model — a register points to the top
 * of a software-managed stack in memory. The 8008 instead uses a fixed-size
 * circular shift register: all 8 entries physically shift when you push or pop.
 *
 * This is why the 8008 has no "stack pointer" register: the hardware handles
 * it internally. The trade-off: only 7 levels of subroutine nesting vs unlimited
 * depth with a software stack.
 *
 * === D flip-flop model ===
 *
 * Each entry is a 14-bit register (14 D flip-flops). Every write routes through
 * `dFlipFlop()` with a rising clock edge simulation (clock=0 then clock=1).
 * The Q output of the slave flip-flop is the stored value.
 *
 * Total gate count: 8 × 14 = 112 D flip-flops.
 * Each flip-flop uses ~10 gates (2 D latches × 2 NOR+NOT gates each + wiring).
 * Approximate total: ~1,120 gate calls per full stack write cycle.
 */

import { dFlipFlop, type FlipFlopState, type Bit } from "@coding-adventures/logic-gates";
import { intToBits, bitsToInt } from "./bits.js";

/** Number of stack entries in the 8008's hardware stack. */
const STACK_DEPTH = 8;

/** Number of bits per stack entry (14-bit address). */
const ENTRY_BITS = 14;

/**
 * Write a 14-bit value to a stack entry via D flip-flop rising-edge simulation.
 *
 * Models the behavior of 14 D flip-flops receiving a rising clock edge:
 * 1. clock=0: each master latch absorbs its data bit
 * 2. clock=1: each slave latch outputs the master's captured value
 *
 * @param address   - 14-bit value to store (0–16383).
 * @param prevState - Previous FlipFlopState for each of the 14 flip-flops.
 * @returns [storedValue, newStates] — the written value and updated state.
 */
function writeEntry(
  address: number,
  prevState: (FlipFlopState | undefined)[],
): [number, FlipFlopState[]] {
  const bits = intToBits(address & 0x3FFF, ENTRY_BITS);
  const outputBits: Bit[] = [];
  const newStates: FlipFlopState[] = [];

  for (let i = 0; i < ENTRY_BITS; i++) {
    // Rising edge: clock=0 (master absorbs), then clock=1 (slave outputs)
    const [, , masterState] = dFlipFlop(bits[i], 0, prevState[i]);
    const [q, , slaveState] = dFlipFlop(bits[i], 1, masterState);
    outputBits.push(q);
    newStates.push(slaveState);
  }

  return [bitsToInt(outputBits), newStates];
}

/**
 * 8-level push-down stack for the Intel 8008.
 *
 * Entry 0 is always the current program counter. All operations maintain
 * this invariant via push (CALL) and pop (RETURN).
 *
 * Each entry is modeled as a 14-bit register via `dFlipFlop()` calls,
 * with flip-flop state maintained across writes.
 */
export class PushDownStack {
  /**
   * Internal storage: 8 × 14-bit entries (as integers for quick access).
   * entries[0] = current PC; entries[1..7] = return addresses.
   */
  private entries: number[] = new Array(STACK_DEPTH).fill(0);

  /**
   * D flip-flop state for each entry's 14 bits.
   * ffStates[entryIndex][bitIndex] = FlipFlopState | undefined
   */
  private ffStates: (FlipFlopState | undefined)[][] = Array.from(
    { length: STACK_DEPTH },
    () => new Array(ENTRY_BITS).fill(undefined),
  );

  /**
   * Read the current program counter (entry 0).
   *
   * Reading is combinational — D flip-flops drive Q continuously.
   */
  get pc(): number {
    return this.entries[0];
  }

  /**
   * Get all 8 stack entries as a snapshot (for debugging/tracing).
   */
  get snapshot(): number[] {
    return [...this.entries];
  }

  /**
   * Write a new value to the program counter (entry 0) via D flip-flop simulation.
   *
   * Models a rising clock edge across 14 flip-flops simultaneously.
   *
   * @param address - New 14-bit PC value (0–16383).
   */
  setPC(address: number): void {
    const [stored, newState] = writeEntry(address & 0x3FFF, this.ffStates[0]);
    this.entries[0] = stored;
    this.ffStates[0] = newState;
  }

  /**
   * Push a new target address onto the stack (CALL operation).
   *
   * === What happens during a CALL ===
   *
   * 1. Stack rotates DOWN: entry[7] ← entry[6], ..., entry[1] ← entry[0].
   * 2. The target address is loaded into entry[0].
   *
   * In hardware: 8 parallel register loads on a single clock edge.
   * Each entry receives the value of the adjacent entry via register gates.
   *
   * @param returnAddress - The current PC (post-instruction) to save.
   * @param target        - The subroutine address to jump to.
   */
  push(returnAddress: number, target: number): void {
    // Rotate DOWN (from bottom to top to avoid clobbering source values)
    for (let i = STACK_DEPTH - 1; i >= 1; i--) {
      const [stored, newState] = writeEntry(this.entries[i - 1], this.ffStates[i]);
      this.entries[i] = stored;
      this.ffStates[i] = newState;
    }
    // Load target into entry 0
    const [stored, newState] = writeEntry(target & 0x3FFF, this.ffStates[0]);
    this.entries[0] = stored;
    this.ffStates[0] = newState;
  }

  /**
   * Pop the top of the stack (RETURN operation).
   *
   * Stack rotates UP: entry[0] ← entry[1], entry[1] ← entry[2], ...,
   * entry[6] ← entry[7]. Entry[7] retains its old value.
   */
  pop(): void {
    // Rotate UP (from top to bottom to avoid clobbering)
    for (let i = 0; i < STACK_DEPTH - 1; i++) {
      const [stored, newState] = writeEntry(this.entries[i + 1], this.ffStates[i]);
      this.entries[i] = stored;
      this.ffStates[i] = newState;
    }
  }

  /**
   * Reset all stack entries and flip-flop state to 0 (CPU reset / power-on).
   */
  reset(): void {
    this.entries.fill(0);
    this.ffStates = Array.from(
      { length: STACK_DEPTH },
      () => new Array(ENTRY_BITS).fill(undefined),
    );
  }
}
