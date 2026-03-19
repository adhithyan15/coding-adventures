/**
 * Logic Gates — Layer 1 of the computing stack.
 *
 * Fundamental logic gate implementations: AND, OR, NOT, XOR, NAND, NOR, XNOR.
 * Also includes NAND-derived gates (all gates built from NAND only),
 * multi-input variants, and sequential logic (latches, flip-flops, registers).
 */

// Fundamental gates
export { NOT, AND, OR, XOR } from "./gates.js";

// Composite gates
export { NAND, NOR, XNOR } from "./gates.js";

// NAND-derived gates
export {
  nandNot,
  nandAnd,
  nandOr,
  nandXor,
  nandNor,
  nandXnor,
} from "./gates.js";

// Multi-input gates
export { andN, orN } from "./gates.js";

// Multiplexer and demultiplexer
export { mux, dmux } from "./gates.js";

// Types and validation
export { type Bit, validateBit } from "./gates.js";

// Sequential logic
export {
  srLatch,
  dLatch,
  dFlipFlop,
  register,
  shiftRegister,
  counter,
} from "./sequential.js";

// Sequential logic types
export type {
  FlipFlopState,
  CounterState,
} from "./sequential.js";
