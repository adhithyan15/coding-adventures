/**
 * Clock -- the heartbeat of every digital circuit.
 *
 * This package simulates the system clock that drives all sequential logic
 * in a computer. It provides:
 *
 * - Clock: A square-wave generator that alternates between 0 and 1
 * - ClockDivider: Derives slower clocks from a fast master clock
 * - MultiPhaseClock: Generates multiple non-overlapping clock phases
 */

export { Clock, ClockDivider, MultiPhaseClock } from "./clock.js";
export type { ClockEdge, ClockListener } from "./clock.js";
