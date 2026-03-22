/**
 * Block RAM -- Layer 3 of the computing stack.
 *
 * SRAM cells, arrays, and synchronous RAM modules for FPGA and CPU memory.
 * Builds on logic-gates (Layer 1) to provide:
 *
 * - SRAMCell: single-bit gate-level storage
 * - SRAMArray: 2D grid of SRAM cells with row/column addressing
 * - SinglePortRAM: synchronous single-port memory with read modes
 * - DualPortRAM: true dual-port memory with collision detection
 * - ConfigurableBRAM: FPGA-style reconfigurable Block RAM
 */

// SRAM primitives
export { SRAMCell, SRAMArray } from "./sram.js";

// RAM modules
export { ReadMode, SinglePortRAM, DualPortRAM, WriteCollisionError } from "./ram.js";

// Configurable Block RAM
export { ConfigurableBRAM } from "./bram.js";
