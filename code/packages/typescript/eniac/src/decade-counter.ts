/**
 * Decade Ring Counter — ENIAC's way of storing one decimal digit.
 *
 * === How ENIAC Stored Numbers ===
 *
 * ENIAC didn't use binary. Each decimal digit (0-9) was stored in a
 * **ring of 10 vacuum tubes** (flip-flops). Exactly one tube in the ring
 * is "on" at any time — its position tells you the digit value.
 *
 * ```
 *     [0]---[1]---[2]---[3]---[4]---[5]---[6]---[7]---[8]---[9]
 *      |                                                       |
 *      +-------------------------------------------------------+
 *                         (ring wraps around)
 * ```
 *
 * === How Counting Works ===
 *
 * To count: a pulse advances the "on" position one step clockwise.
 * When it wraps from 9→0, it produces a **carry pulse** to the next
 * decade counter (the tens digit).
 *
 * This is the decimal equivalent of a binary counter — but uses
 * 10 tubes per digit instead of 1 flip-flop per bit.
 *
 * === Why Decimal? ===
 *
 * ENIAC's designers (Eckert and Mauchly) chose decimal arithmetic because:
 * 1. It was familiar — humans think in decimal
 * 2. Punch card I/O was already decimal
 * 3. Converting binary↔decimal would add complexity
 *
 * The tradeoff: 10 tubes per digit vs. ~3.32 tubes per digit in binary
 * (log2(10) ≈ 3.32 bits needed to represent 0-9). ENIAC's 17,468 tubes
 * were partly a consequence of this decimal choice.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** State of one vacuum tube in a decade ring counter. */
export interface TubeState {
  /** Position in the ring (0-9). */
  position: number;
  /** Whether this tube is currently conducting (the "on" tube). */
  conducting: boolean;
}

/** A decade ring counter: 10 tubes representing one decimal digit (0-9). */
export interface DecadeCounter {
  /** The 10 tubes in the ring. Exactly one has conducting=true. */
  tubes: TubeState[];
  /** Which digit is currently represented (0-9). */
  currentDigit: number;
}

/** Result of sending pulses to a decade counter. */
export interface PulseResult {
  /** The updated counter state after all pulses. */
  counter: DecadeCounter;
  /** True if the counter wrapped from 9→0 (carry to next decade). */
  carry: boolean;
  /** Sequence of digit positions visited during pulsing (for animation). */
  stepsTraced: number[];
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create a decade ring counter initialized to a given digit.
 *
 * Sets up 10 tubes in a ring with exactly one conducting (the tube
 * at the specified digit position).
 *
 * @param digit - Initial digit value (0-9, default 0).
 * @returns A new DecadeCounter.
 */
export function createDecadeCounter(digit: number = 0): DecadeCounter {
  if (digit < 0 || digit > 9 || !Number.isInteger(digit)) {
    throw new Error(`digit must be an integer 0-9, got ${digit}`);
  }

  const tubes: TubeState[] = Array.from({ length: 10 }, (_, i) => ({
    position: i,
    conducting: i === digit,
  }));

  return { tubes, currentDigit: digit };
}

// ---------------------------------------------------------------------------
// Pulse Operation
// ---------------------------------------------------------------------------

/**
 * Send N pulses to a decade counter.
 *
 * Each pulse advances the "on" tube one position clockwise in the ring.
 * If the counter wraps from 9→0, a carry is generated.
 *
 * The stepsTraced array records each intermediate position, which is
 * useful for animating the ring counter stepping through positions.
 *
 * @param counter - The current counter state.
 * @param pulses - Number of pulses to send (default 1).
 * @returns PulseResult with updated counter, carry flag, and trace.
 *
 * @example
 * ```ts
 * let counter = createDecadeCounter(7);
 * let result = pulseDecadeCounter(counter, 5);
 * // 7→8→9→0→1→2
 * // result.counter.currentDigit === 2
 * // result.carry === true (wrapped 9→0)
 * // result.stepsTraced === [8, 9, 0, 1, 2]
 * ```
 */
export function pulseDecadeCounter(
  counter: DecadeCounter,
  pulses: number = 1,
): PulseResult {
  if (pulses < 0 || !Number.isInteger(pulses)) {
    throw new Error(`pulses must be a non-negative integer, got ${pulses}`);
  }

  let digit = counter.currentDigit;
  let carry = false;
  const stepsTraced: number[] = [];

  for (let i = 0; i < pulses; i++) {
    digit = (digit + 1) % 10;
    stepsTraced.push(digit);

    // Carry is generated when wrapping from 9→0
    if (digit === 0) {
      carry = true;
    }
  }

  return {
    counter: createDecadeCounter(digit),
    carry,
    stepsTraced,
  };
}
