/**
 * Transistors — Layer 2 of the computing stack.
 *
 * This package provides transistor-level circuit simulation including:
 * - Vacuum tube triode (1906) with Child-Langmuir law — the historical predecessor
 * - MOSFET transistors (NMOS, PMOS) with region detection and current calculation
 * - BJT transistors (NPN, PNP) with Ebers-Moll model
 * - CMOS logic gates (NOT, NAND, NOR, AND, OR, XOR)
 * - TTL logic gates (NAND, RTL Inverter)
 * - Analog amplifier analysis (common-source, common-emitter)
 * - Electrical analysis (noise margins, power, timing, technology scaling)
 */

// Types and enums
export {
  MOSFETRegion,
  BJTRegion,
  TransistorType,
  defaultMOSFETParams,
  defaultBJTParams,
  defaultCircuitParams,
} from "./types.js";
export type {
  MOSFETParams,
  BJTParams,
  CircuitParams,
  GateOutput,
  AmplifierAnalysis,
  NoiseMargins,
  PowerAnalysis,
  TimingAnalysis,
} from "./types.js";

// Vacuum tube (triode) — the historical predecessor to all transistors
export {
  defaultTriodeParams,
  triodePlateCurrent,
  isConducting,
} from "./vacuum-tube.js";
export type { TriodeParams } from "./vacuum-tube.js";

// MOSFET transistors
export { NMOS, PMOS } from "./mosfet.js";

// BJT transistors
export { NPN, PNP } from "./bjt.js";

// CMOS logic gates
export {
  validateBit,
  CMOSInverter,
  CMOSNand,
  CMOSNor,
  CMOSAnd,
  CMOSOr,
  CMOSXor,
  CMOSXnor,
} from "./cmos_gates.js";

// TTL logic gates
export { TTLNand, RTLInverter } from "./ttl_gates.js";

// Amplifier analysis
export {
  analyzeCommonSourceAmp,
  analyzeCommonEmitterAmp,
} from "./amplifier.js";

// Electrical analysis
export {
  computeNoiseMargins,
  analyzePower,
  analyzeTiming,
  compareCmosVsTtl,
  demonstrateCmosScaling,
} from "./analysis.js";
