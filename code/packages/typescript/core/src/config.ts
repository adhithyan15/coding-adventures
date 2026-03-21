/**
 * CoreConfig and preset configurations for processor cores.
 *
 * Every parameter that a real CPU architect would tune is exposed here.
 * Change the branch predictor and you get different accuracy. Double the
 * L1 cache and you get fewer misses. Deepen the pipeline and you get
 * higher clock speeds but worse misprediction penalties.
 */

import { type CacheConfig } from "@coding-adventures/cache";
import {
  type PipelineConfig,
  classic5Stage,
  deep13Stage,
} from "@coding-adventures/cpu-pipeline";

// =========================================================================
// RegisterFileConfig
// =========================================================================

/**
 * Configuration for the general-purpose register file.
 *
 * Real-world register file sizes:
 *
 *     MIPS:     32 registers, 32-bit  (R0 hardwired to zero)
 *     ARMv8:    31 registers, 64-bit  (X0-X30, no zero register)
 *     RISC-V:   32 registers, 32/64-bit (x0 hardwired to zero)
 *     x86-64:   16 registers, 64-bit  (RAX, RBX, ..., R15)
 */
export interface RegisterFileConfig {
  /** Number of general-purpose registers. Typical: 16 or 32. */
  count: number;
  /** Bit width of each register: 32 or 64. */
  width: number;
  /** Whether register 0 is hardwired to zero (RISC-V/MIPS convention). */
  zeroRegister: boolean;
}

/**
 * Returns a sensible default: 16 registers, 32-bit, with R0 hardwired to zero.
 */
export function defaultRegisterFileConfig(): RegisterFileConfig {
  return { count: 16, width: 32, zeroRegister: true };
}

// =========================================================================
// FPUnitConfig
// =========================================================================

/**
 * Configuration for the optional floating-point unit.
 *
 * Not all cores have an FP unit. Microcontrollers and efficiency cores
 * often omit it to save area and power.
 */
export interface FPUnitConfig {
  /** Supported FP formats: "fp16", "fp32", "fp64". */
  formats: string[];
  /** How many cycles an FP operation takes. Typical: 3-5 for add/multiply. */
  pipelineDepth: number;
}

// =========================================================================
// CoreConfig
// =========================================================================

/**
 * Complete configuration for a processor core.
 *
 * This is the "spec sheet" for the core. A CPU architect decides these
 * values based on the target workload, power budget, and die area.
 */
export interface CoreConfig {
  /** Human-readable identifier (e.g., "Simple", "CortexA78Like"). */
  name: string;

  // --- Pipeline ---
  /** Pipeline stage configuration. Defaults to classic 5-stage. */
  pipeline: PipelineConfig;

  // --- Branch Prediction ---
  /**
   * Predictor algorithm:
   * "static_always_taken", "static_always_not_taken", "static_btfnt",
   * "one_bit", "two_bit"
   */
  branchPredictorType: string;
  /** Number of entries in the prediction table. Typical: 256-4096. */
  branchPredictorSize: number;
  /** Number of entries in the Branch Target Buffer. */
  btbSize: number;

  // --- Hazard Handling ---
  /** Enables the hazard detection unit. */
  hazardDetection: boolean;
  /** Enables data forwarding (bypassing) paths. */
  forwarding: boolean;

  // --- Register File ---
  /** Register file configuration. null = use default. */
  registerFile: RegisterFileConfig | null;

  // --- Floating Point ---
  /** Floating-point unit config. null = no FP support. */
  fpUnit: FPUnitConfig | null;

  // --- Cache Hierarchy ---
  /** L1 instruction cache config. null = use default 4KB direct-mapped. */
  l1iCache: CacheConfig | null;
  /** L1 data cache config. null = use default 4KB direct-mapped. */
  l1dCache: CacheConfig | null;
  /** L2 cache config. null = no L2. */
  l2Cache: CacheConfig | null;

  // --- Memory ---
  /** Size of main memory in bytes. Default: 65536 (64KB). */
  memorySize: number;
  /** Access latency for main memory in cycles. Default: 100. */
  memoryLatency: number;
}

/**
 * Returns a minimal, sensible configuration for testing.
 *
 * This is the "teaching core" -- a 5-stage pipeline with static prediction,
 * small caches, and 16 registers.
 */
export function defaultCoreConfig(): CoreConfig {
  return {
    name: "Default",
    pipeline: classic5Stage(),
    branchPredictorType: "static_always_not_taken",
    branchPredictorSize: 256,
    btbSize: 64,
    hazardDetection: true,
    forwarding: true,
    registerFile: null,
    fpUnit: null,
    l1iCache: null,
    l1dCache: null,
    l2Cache: null,
    memorySize: 65536,
    memoryLatency: 100,
  };
}

// =========================================================================
// Preset Configurations
// =========================================================================

/**
 * SimpleConfig returns a minimal teaching core inspired by MIPS R2000 (1985).
 */
export function simpleConfig(): CoreConfig {
  return {
    name: "Simple",
    pipeline: classic5Stage(),
    branchPredictorType: "static_always_not_taken",
    branchPredictorSize: 256,
    btbSize: 64,
    hazardDetection: true,
    forwarding: true,
    registerFile: { count: 16, width: 32, zeroRegister: true },
    fpUnit: null,
    l1iCache: null,
    l1dCache: null,
    l2Cache: null,
    memorySize: 65536,
    memoryLatency: 100,
  };
}

/**
 * CortexA78LikeConfig approximates the ARM Cortex-A78 performance core.
 */
export function cortexA78LikeConfig(): CoreConfig {
  return {
    name: "CortexA78Like",
    pipeline: deep13Stage(),
    branchPredictorType: "two_bit",
    branchPredictorSize: 4096,
    btbSize: 1024,
    hazardDetection: true,
    forwarding: true,
    registerFile: { count: 31, width: 64, zeroRegister: false },
    fpUnit: { formats: ["fp32", "fp64"], pipelineDepth: 4 },
    l1iCache: null,
    l1dCache: null,
    l2Cache: null,
    memorySize: 1048576,
    memoryLatency: 100,
  };
}

// =========================================================================
// MultiCoreConfig
// =========================================================================

/**
 * Configuration for a multi-core processor.
 */
export interface MultiCoreConfig {
  /** Number of processor cores. */
  numCores: number;
  /** Configuration shared by all cores. */
  coreConfig: CoreConfig;
  /** Shared L3 cache config. null = no L3. */
  l3Cache: CacheConfig | null;
  /** Total shared memory in bytes. */
  memorySize: number;
  /** DRAM access latency in cycles. */
  memoryLatency: number;
}

/**
 * Returns a 2-core configuration for testing.
 */
export function defaultMultiCoreConfig(): MultiCoreConfig {
  return {
    numCores: 2,
    coreConfig: simpleConfig(),
    l3Cache: null,
    memorySize: 1048576,
    memoryLatency: 100,
  };
}
