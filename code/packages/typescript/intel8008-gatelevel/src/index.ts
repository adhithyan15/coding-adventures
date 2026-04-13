/**
 * Intel 8008 Gate-Level Simulator — public API.
 *
 * All computations route through logic gate functions (AND, OR, XOR, NOT)
 * from the logic-gates and arithmetic packages. This gives a faithful
 * simulation of the 8008's actual digital-logic behavior.
 *
 * ## Main export
 *
 * ```typescript
 * import { Intel8008GateLevel } from "@coding-adventures/intel8008-gatelevel";
 *
 * const cpu = new Intel8008GateLevel();
 * const program = new Uint8Array([0x06, 0x01, 0x3E, 0x02, 0x80, 0x76]);
 * const traces = cpu.run(program);
 * console.log(cpu.a);  // 3
 * ```
 *
 * ## Layer in the computing stack
 *
 * ```
 * logic-gates      (AND, OR, XOR, NOT — CMOS transistor pairs)
 *   ↓
 * arithmetic       (half adder, full adder, ripple-carry adder, ALU)
 *   ↓
 * intel8008-gatelevel  ← YOU ARE HERE
 *   ↓
 * intel8008-simulator  (behavioral reference, same API)
 * ```
 *
 * ## Sub-modules (exported for testing and educational use)
 *
 * - `ProgramCounter`  — 14-bit PC with half-adder increment chain
 * - `PushDownStack`   — 8-level push-down stack built from D flip-flop registers
 * - `RegisterFile`    — 7-register file (A,B,C,D,E,H,L) via D flip-flops
 * - `FlagRegister`    — 4-bit flag register (CY, Z, S, P)
 * - `GateALU8`        — 8-bit ALU via ripple-carry adder chain
 * - `decode`          — Combinational opcode decoder using AND/OR/NOT gates
 * - `intToBits`       — Integer → bit array (LSB first)
 * - `bitsToInt`       — Bit array → integer
 * - `computeParity`   — Parity check via XOR reduction + NOT
 */

// Main simulator class + types
export { Intel8008GateLevel } from "./cpu.js";
export type { Flags, Trace } from "./cpu.js";

// Sub-modules for educational use and testing
export { ProgramCounter } from "./pc.js";
export { PushDownStack } from "./stack.js";
export { RegisterFile, FlagRegister } from "./registers.js";
export { GateALU8 } from "./alu.js";
export type { GateFlags } from "./alu.js";
export { decode } from "./decoder.js";
export type { DecoderOutput } from "./decoder.js";
export { intToBits, bitsToInt, computeParity } from "./bits.js";
