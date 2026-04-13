/**
 * Intel 8008 Behavioral Simulator — Layer 4f of the computing stack.
 *
 * The Intel 8008 (1972) was the world's first 8-bit microprocessor.
 * This package simulates its complete instruction set behaviorally —
 * using host-language arithmetic for speed, not gate simulation.
 *
 * For gate-level simulation, see `@coding-adventures/intel8008-gatelevel`.
 */

export { Intel8008Simulator } from "./simulator.js";
export type { Flags, Trace } from "./simulator.js";
