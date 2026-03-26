/**
 * ENIAC — decimal arithmetic models for the first general-purpose
 * electronic computer (1945).
 *
 * ENIAC used vacuum tube decade ring counters for decimal arithmetic,
 * not binary. This package provides the circuit models:
 *
 * - Decade ring counter: 10 tubes in a ring representing one digit (0-9)
 * - Accumulator: chained ring counters for multi-digit decimal addition
 */

export {
  createDecadeCounter,
  pulseDecadeCounter,
} from "./decade-counter.js";
export type {
  TubeState,
  DecadeCounter,
  PulseResult,
} from "./decade-counter.js";

export {
  createAccumulator,
  accumulatorValue,
  accumulatorAdd,
} from "./accumulator.js";
export type {
  Accumulator,
  DigitTrace,
  AdditionTrace,
} from "./accumulator.js";
