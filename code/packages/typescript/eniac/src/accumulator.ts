/**
 * ENIAC Accumulator — multi-digit decimal addition using ring counters.
 *
 * === ENIAC's Accumulators ===
 *
 * Each ENIAC accumulator held a 10-digit decimal number (plus sign).
 * It was built from 10 decade ring counters chained together, with
 * carry propagation from ones → tens → hundreds → ... → billions.
 *
 * ENIAC had 20 accumulators, each containing ~550 vacuum tubes.
 * Total for accumulators alone: ~11,000 tubes.
 *
 * === How Addition Works ===
 *
 * To add a number to the accumulator, the process works digit by digit:
 *
 * 1. Extract the ones digit of the addend
 * 2. Send that many pulses to the ones decade ring counter
 * 3. If the ones counter wraps (carry), send one pulse to the tens counter
 * 4. Extract the tens digit of the addend
 * 5. Send that many pulses to the tens counter (plus any carry from step 3)
 * 6. Continue for each digit...
 *
 * This is fundamentally different from binary addition (which uses XOR/AND
 * gates in parallel). ENIAC's approach is sequential pulse counting —
 * slower but conceptually simpler and directly in decimal.
 *
 * === Worked Example: 42 + 75 = 117 ===
 *
 * Accumulator starts at: 0042
 *
 * Ones decade (digit=2, add 5):
 *   2→3→4→5→6→7  (5 pulses, no wrap, digit=7)
 *
 * Tens decade (digit=4, add 7):
 *   4→5→6→7→8→9→0→1  (7 pulses, wraps 9→0, carry! digit=1)
 *
 * Hundreds decade (digit=0, add 0 + carry=1):
 *   0→1  (1 pulse from carry, digit=1)
 *
 * Thousands decade (digit=0, add 0 + no carry):
 *   No change (digit=0)
 *
 * Result: 0117 ✓
 */

import {
  type DecadeCounter,
  type PulseResult,
  createDecadeCounter,
  pulseDecadeCounter,
} from "./decade-counter.js";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** A multi-digit ENIAC accumulator. */
export interface Accumulator {
  /** Decade ring counters, index 0 = ones, 1 = tens, etc. */
  decades: DecadeCounter[];
  /** Sign: true = positive, false = negative. */
  sign: boolean;
  /** Number of decades (digits). */
  digitCount: number;
}

/** Per-digit trace showing what happened during addition. */
export interface DigitTrace {
  /** Which decade position (0 = ones, 1 = tens, ...). */
  position: number;
  /** Digit value before the operation. */
  digitBefore: number;
  /** Number of pulses sent to this decade (addend digit + incoming carry). */
  pulsesReceived: number;
  /** Digit value after the operation. */
  digitAfter: number;
  /** Whether this decade generated a carry (wrapped 9→0). */
  carryOut: boolean;
  /** Full pulse result with step-by-step trace. */
  pulseResult: PulseResult;
}

/** Complete trace of an accumulator addition. */
export interface AdditionTrace {
  /** The accumulator state after addition. */
  accumulator: Accumulator;
  /** Per-digit trace data (one entry per decade). */
  digitTraces: DigitTrace[];
  /** The carry chain: carries[i] = true if decade i produced a carry. */
  carries: boolean[];
  /** The value that was added. */
  addend: number;
  /** Final overflow carry (true if the MSB decade carried). */
  overflow: boolean;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create an ENIAC accumulator initialized to a given value.
 *
 * @param value - Initial decimal value (default 0). Must be non-negative.
 * @param digitCount - Number of decades (default 4 for visualization).
 * @returns A new Accumulator.
 */
export function createAccumulator(
  value: number = 0,
  digitCount: number = 4,
): Accumulator {
  if (value < 0 || !Number.isInteger(value)) {
    throw new Error(`value must be a non-negative integer, got ${value}`);
  }
  if (digitCount < 1) {
    throw new Error(`digitCount must be at least 1, got ${digitCount}`);
  }

  const maxValue = Math.pow(10, digitCount) - 1;
  if (value > maxValue) {
    throw new Error(
      `value ${value} exceeds maximum for ${digitCount} digits (${maxValue})`
    );
  }

  const decades: DecadeCounter[] = [];
  let remaining = value;

  for (let i = 0; i < digitCount; i++) {
    const digit = remaining % 10;
    decades.push(createDecadeCounter(digit));
    remaining = Math.floor(remaining / 10);
  }

  return { decades, sign: true, digitCount };
}

/**
 * Read the decimal value stored in an accumulator.
 *
 * @param acc - The accumulator to read.
 * @returns The unsigned decimal value.
 */
export function accumulatorValue(acc: Accumulator): number {
  let value = 0;
  for (let i = acc.decades.length - 1; i >= 0; i--) {
    value = value * 10 + acc.decades[i].currentDigit;
  }
  return value;
}

// ---------------------------------------------------------------------------
// Addition via Pulse Counting
// ---------------------------------------------------------------------------

/**
 * Add a value to an accumulator using ENIAC's pulse counting method.
 *
 * Processes each digit from least significant to most significant,
 * sending pulses to each decade ring counter and propagating carries.
 *
 * @param acc - The accumulator to add to.
 * @param addend - The value to add (non-negative integer).
 * @returns AdditionTrace with updated accumulator and per-digit trace data.
 *
 * @example
 * ```ts
 * let acc = createAccumulator(42, 4);
 * let trace = accumulatorAdd(acc, 75);
 * // accumulatorValue(trace.accumulator) === 117
 * // trace.digitTraces[1].carryOut === true (tens wrapped 9→0)
 * // trace.carries === [false, true, false, false]
 * ```
 */
export function accumulatorAdd(
  acc: Accumulator,
  addend: number,
): AdditionTrace {
  if (addend < 0 || !Number.isInteger(addend)) {
    throw new Error(`addend must be a non-negative integer, got ${addend}`);
  }

  const newDecades: DecadeCounter[] = [];
  const digitTraces: DigitTrace[] = [];
  const carries: boolean[] = [];
  let carryIn = false;
  let remaining = addend;

  for (let i = 0; i < acc.digitCount; i++) {
    const addendDigit = remaining % 10;
    remaining = Math.floor(remaining / 10);

    const totalPulses = addendDigit + (carryIn ? 1 : 0);
    const digitBefore = acc.decades[i].currentDigit;

    const pulseResult = pulseDecadeCounter(acc.decades[i], totalPulses);

    newDecades.push(pulseResult.counter);
    carries.push(pulseResult.carry);

    digitTraces.push({
      position: i,
      digitBefore,
      pulsesReceived: totalPulses,
      digitAfter: pulseResult.counter.currentDigit,
      carryOut: pulseResult.carry,
      pulseResult,
    });

    carryIn = pulseResult.carry;
  }

  return {
    accumulator: {
      decades: newDecades,
      sign: acc.sign,
      digitCount: acc.digitCount,
    },
    digitTraces,
    carries,
    addend,
    overflow: carryIn,
  };
}
