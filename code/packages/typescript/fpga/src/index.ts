/**
 * FPGA -- Field-Programmable Gate Array abstraction.
 *
 * This package models the architecture of an FPGA, from the atomic LUT
 * (Look-Up Table) up through slices, CLBs (Configurable Logic Blocks),
 * routing fabric, and I/O blocks.
 *
 * The key insight: **a truth table is a program**. A LUT stores a truth
 * table in SRAM and uses a MUX tree to evaluate it. By connecting LUTs
 * through a programmable routing fabric, any digital circuit can be
 * implemented -- and reprogrammed -- without changing the hardware.
 *
 * Modules:
 *     lut:            LUT (K-input look-up table)
 *     slice:          Slice (2 LUTs + 2 FFs + carry chain)
 *     clb:            CLB (2 slices)
 *     switch-matrix:  SwitchMatrix (programmable routing crossbar)
 *     io-block:       IOBlock (bidirectional I/O pad)
 *     bitstream:      Bitstream (JSON configuration format)
 *     fabric:         FPGA (top-level fabric model)
 */

// LUT
export { LUT } from "./lut.js";

// Slice
export { Slice } from "./slice.js";
export type { SliceOutput } from "./slice.js";

// CLB
export { CLB } from "./clb.js";
export type { CLBOutput } from "./clb.js";

// Switch Matrix
export { SwitchMatrix } from "./switch-matrix.js";

// I/O Block
export { IOBlock, IOMode } from "./io-block.js";

// Bitstream
export { Bitstream } from "./bitstream.js";
export type { SliceConfig, CLBConfig, RouteConfig, IOConfig } from "./bitstream.js";

// Fabric
export { FPGA } from "./fabric.js";
