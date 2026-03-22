/**
 * Intel 4004 Gate-Level Simulator -- every operation routes through real logic gates.
 *
 * All computation flows through: NOT/AND/OR/XOR -> halfAdder -> fullAdder ->
 * rippleCarryAdder -> ALU, and state is stored in D flip-flop registers.
 */

export { Intel4004GateLevel } from "./cpu.js";
export type { GateTrace } from "./cpu.js";
export { GateALU } from "./alu.js";
export { RegisterFile, Accumulator, CarryFlag } from "./registers.js";
export { ProgramCounter } from "./pc.js";
export { HardwareStack } from "./stack.js";
export { RAM } from "./ram.js";
export { decode } from "./decoder.js";
export type { DecodedInstruction } from "./decoder.js";
export { intToBits, bitsToInt } from "./bits.js";
