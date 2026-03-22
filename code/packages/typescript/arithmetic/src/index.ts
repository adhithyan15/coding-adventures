/**
 * Arithmetic — Layer 2 of the computing stack.
 *
 * Half adder, full adder, ripple carry adder, and ALU.
 * Built entirely from logic gates (Layer 1).
 */

export { halfAdder, fullAdder, rippleCarryAdder, rippleCarryAdderTraced } from "./adders.js";
export type { FullAdderSnapshot, RippleCarryResult } from "./adders.js";
export { ALU, ALUOp } from "./alu.js";
export type { ALUResult } from "./alu.js";
