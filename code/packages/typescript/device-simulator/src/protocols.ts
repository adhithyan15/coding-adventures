/**
 * Protocols -- shared types for all device simulators.
 *
 * === What is a Device Simulator? ===
 *
 * A device simulator models a **complete accelerator** -- not just one compute
 * unit, but the entire chip with all its compute units, global memory, caches,
 * and the work distributor that ties them together.
 *
 * Think of it as the difference between simulating one factory floor (Layer 7)
 * versus simulating the entire factory complex:
 *
 *     Layer 7 (Compute Unit):    One SM / CU / MXU -- a single factory floor
 *     Layer 6 (Device):          The whole factory -- all floors + warehouse +
 *                                shipping dock + floor manager's office
 *
 * The device layer adds four new concepts:
 *
 * 1. **Global Memory (VRAM)** -- the large device-wide memory (the warehouse).
 *    All compute units share it. High bandwidth but high latency (~400 cycles).
 *
 * 2. **L2 Cache** -- sits between compute units and global memory. Reduces the
 *    average latency for frequently-accessed data.
 *
 * 3. **Work Distributor** -- takes kernel launches (work orders) and assigns
 *    thread blocks to compute units that have available resources.
 *
 * 4. **Host Interface** -- the connection to the CPU. Data must be copied from
 *    CPU memory to device memory before the GPU can use it (except on Apple's
 *    unified memory, where it's zero-copy).
 *
 * === Memory Hierarchy at the Device Level ===
 *
 *                 +---------------+
 *     CPU RAM --> | Host Interface| --> PCIe / NVLink / unified
 *                 +-------+-------+
 *                         |
 *                 +-------+-------+
 *                 | Global Memory |  24-80 GB, ~400 cycle latency
 *                 |  (HBM/GDDR)  |  1-3 TB/s bandwidth
 *                 +-------+-------+
 *                         |
 *                 +-------+-------+
 *                 |   L2 Cache    |  4-96 MB, ~200 cycle latency
 *                 |  (shared)     |
 *                 +--+---+---+---++
 *                    |   |   |
 *                  CU 0 CU 1 ... CU N   (each with local shared memory)
 */

import type { Instruction } from "@coding-adventures/gpu-core";
import type { ClockEdge } from "@coding-adventures/clock";
import type { ComputeUnit, ComputeUnitTrace } from "@coding-adventures/compute-unit";

// =========================================================================
// MemoryTransaction -- a single wide memory access after coalescing
// =========================================================================

/**
 * A single wide memory transaction after coalescing.
 *
 * When 32 threads in a warp each request 4 bytes, those 128 bytes of
 * requests might coalesce into a single 128-byte transaction (best case)
 * or 32 separate transactions (worst case -- scattered access).
 *
 * === Coalescing Visual ===
 *
 * Best case (1 transaction):
 *     Thread  0  1  2  3  4  ...  31
 *     Addr   [0][4][8][12][16]...[124]
 *            +------------------------+
 *              One 128B transaction
 *
 * Worst case (32 transactions):
 *     Thread  0     1      2      3
 *     Addr   [0]  [512]  [1024]  [1536]  ...
 *             |      |      |      |
 *          Trans 1 Trans 2 Trans 3 Trans 4
 */
export interface MemoryTransaction {
  /** Aligned start address of the transaction. */
  readonly address: number;
  /** Transaction size in bytes (32, 64, or 128). */
  readonly size: number;
  /** Bitmask of which threads are served by this transaction. */
  readonly threadMask: number;
}

// =========================================================================
// GlobalMemoryStats -- tracks memory access patterns and efficiency
// =========================================================================

/**
 * Tracks memory access patterns and efficiency.
 *
 * === Why Track These? ===
 *
 * Memory access patterns are the #1 performance bottleneck on GPUs.
 * A kernel that achieves perfect coalescing uses 32x less bandwidth than
 * one with fully scattered access. These stats tell you whether your
 * memory accesses are efficient.
 *
 * Key metric: **coalescingEfficiency**
 *     = totalRequests / totalTransactions
 *     Ideal = 1.0 (every request coalesces into existing transactions)
 *     Worst = 32.0 for 32-wide warps (nothing coalesces)
 */
export interface GlobalMemoryStats {
  totalReads: number;
  totalWrites: number;
  totalTransactions: number;
  totalRequests: number;
  bytesTransferred: number;
  coalescingEfficiency: number;
  partitionConflicts: number;
  hostToDeviceBytes: number;
  deviceToHostBytes: number;
  hostTransferCycles: number;
}

/**
 * Create a fresh GlobalMemoryStats with all counters at zero.
 */
export function makeGlobalMemoryStats(): GlobalMemoryStats {
  return {
    totalReads: 0,
    totalWrites: 0,
    totalTransactions: 0,
    totalRequests: 0,
    bytesTransferred: 0,
    coalescingEfficiency: 0,
    partitionConflicts: 0,
    hostToDeviceBytes: 0,
    deviceToHostBytes: 0,
    hostTransferCycles: 0,
  };
}

/**
 * Recalculate coalescing efficiency from current counts.
 */
export function updateEfficiency(stats: GlobalMemoryStats): void {
  if (stats.totalTransactions > 0) {
    stats.coalescingEfficiency = stats.totalRequests / stats.totalTransactions;
  }
}

// =========================================================================
// KernelDescriptor -- what gets launched on the device
// =========================================================================

/**
 * Describes a kernel launch (GPU) or operation (TPU/NPU).
 *
 * === Two Worlds ===
 *
 * GPU-style devices (NVIDIA, AMD, Intel) receive a **program** with grid
 * and block dimensions -- "run this code on this many threads."
 *
 * Dataflow-style devices (TPU, NPU) receive an **operation** with input
 * and weight data -- "multiply these matrices" or "apply this activation."
 *
 * The same KernelDescriptor handles both by having fields for each style.
 * GPU devices use the program/grid/block fields. Dataflow devices use the
 * operation/input/weight fields.
 */
export interface KernelDescriptor {
  /** Human-readable name for the kernel. */
  readonly name: string;
  /** Unique identifier for the kernel. */
  readonly kernelId: number;

  // GPU-style fields
  readonly program: Instruction[] | null;
  readonly gridDim: readonly [number, number, number];
  readonly blockDim: readonly [number, number, number];
  readonly sharedMemBytes: number;
  readonly registersPerThread: number;

  // Dataflow-style fields (TPU/NPU)
  readonly operation: string;
  readonly inputData: readonly (readonly number[])[] | null;
  readonly weightData: readonly (readonly number[])[] | null;
  readonly outputAddress: number;
}

/**
 * Create a KernelDescriptor with sensible defaults.
 *
 * Only the fields you care about need to be specified. GPU-style kernels
 * typically set program/gridDim/blockDim. Dataflow kernels set
 * operation/inputData/weightData.
 */
export function makeKernelDescriptor(
  partial: Partial<KernelDescriptor> = {},
): KernelDescriptor {
  return {
    name: "unnamed",
    kernelId: 0,
    program: null,
    gridDim: [1, 1, 1],
    blockDim: [32, 1, 1],
    sharedMemBytes: 0,
    registersPerThread: 32,
    operation: "",
    inputData: null,
    weightData: null,
    outputAddress: 0,
    ...partial,
  };
}

/**
 * Total number of threads across all blocks.
 */
export function totalThreads(k: KernelDescriptor): number {
  const [gx, gy, gz] = k.gridDim;
  const [bx, by, bz] = k.blockDim;
  return gx * gy * gz * bx * by * bz;
}

/**
 * Total number of thread blocks in the grid.
 */
export function totalBlocks(k: KernelDescriptor): number {
  const [gx, gy, gz] = k.gridDim;
  return gx * gy * gz;
}

/**
 * Number of threads in each block.
 */
export function threadsPerBlock(k: KernelDescriptor): number {
  const [bx, by, bz] = k.blockDim;
  return bx * by * bz;
}

// =========================================================================
// DeviceConfig -- full device specification
// =========================================================================

/**
 * Complete device specification.
 *
 * === The Knobs That Define a Device ===
 *
 * Every accelerator is characterized by:
 * - How many compute units it has
 * - How much and how fast its memory is
 * - How it connects to the CPU
 * - How it distributes work
 *
 * === Memory Hierarchy Parameters ===
 *
 *     Host RAM --[hostBandwidth]--> Global Memory (VRAM)
 *                                         |
 *                                 [globalMemoryBandwidth]
 *                                         |
 *                                    L2 Cache
 *                                         |
 *                                 Compute Units (shared memory)
 *                                         |
 *                                    Registers
 */
export interface DeviceConfig {
  // Identity
  readonly name: string;
  readonly architecture: string;

  // Compute
  readonly numComputeUnits: number;
  readonly cuConfig: unknown;

  // Memory hierarchy
  readonly l2CacheSize: number;
  readonly l2CacheLatency: number;
  readonly l2CacheAssociativity: number;
  readonly l2CacheLineSize: number;

  readonly globalMemorySize: number;
  readonly globalMemoryBandwidth: number;
  readonly globalMemoryLatency: number;
  readonly memoryChannels: number;

  // Host interface
  readonly hostBandwidth: number;
  readonly hostLatency: number;
  readonly unifiedMemory: boolean;

  // Scheduling
  readonly maxConcurrentKernels: number;
  readonly workDistributionPolicy: string;
}

/**
 * Create a DeviceConfig with sensible defaults.
 */
export function makeDeviceConfig(
  partial: Partial<DeviceConfig> = {},
): DeviceConfig {
  return {
    name: "Generic Accelerator",
    architecture: "generic",
    numComputeUnits: 4,
    cuConfig: null,
    l2CacheSize: 4 * 1024 * 1024,
    l2CacheLatency: 200,
    l2CacheAssociativity: 16,
    l2CacheLineSize: 128,
    globalMemorySize: 16 * 1024 * 1024,
    globalMemoryBandwidth: 1000.0,
    globalMemoryLatency: 400,
    memoryChannels: 8,
    hostBandwidth: 64.0,
    hostLatency: 1000,
    unifiedMemory: false,
    maxConcurrentKernels: 1,
    workDistributionPolicy: "round_robin",
    ...partial,
  };
}

// =========================================================================
// Vendor-specific configs
// =========================================================================

/**
 * AMD Shader Engine -- mid-level grouping of CUs.
 *
 * AMD organizes CUs into Shader Engines, each sharing a geometry
 * processor and rasterizer. For compute workloads, the main effect
 * is that the Command Processor assigns work at the SE level first.
 */
export interface ShaderEngineConfig {
  readonly cusPerEngine: number;
  readonly sharedL1Size: number;
}

export function makeShaderEngineConfig(
  partial: Partial<ShaderEngineConfig> = {},
): ShaderEngineConfig {
  return {
    cusPerEngine: 16,
    sharedL1Size: 32 * 1024,
    ...partial,
  };
}

/**
 * AMD-specific config with Shader Engine hierarchy.
 */
export interface AmdGPUConfig extends DeviceConfig {
  readonly numShaderEngines: number;
  readonly seConfig: ShaderEngineConfig;
  readonly infinityCacheSize: number;
  readonly infinityCacheLatency: number;
  readonly numAces: number;
}

export function makeAmdGPUConfig(
  partial: Partial<AmdGPUConfig> = {},
): AmdGPUConfig {
  const base = makeDeviceConfig({
    name: "AMD RX 7900 XTX",
    architecture: "amd_cu",
    ...partial,
  });
  return {
    ...base,
    numShaderEngines: 6,
    seConfig: makeShaderEngineConfig(),
    infinityCacheSize: 96 * 1024 * 1024,
    infinityCacheLatency: 50,
    numAces: 4,
    ...partial,
  };
}

/**
 * Intel Xe-Slice -- mid-level grouping of Xe-Cores.
 */
export interface XeSliceConfig {
  readonly xeCoresPerSlice: number;
  readonly l1CachePerSlice: number;
}

export function makeXeSliceConfig(
  partial: Partial<XeSliceConfig> = {},
): XeSliceConfig {
  return {
    xeCoresPerSlice: 4,
    l1CachePerSlice: 192 * 1024,
    ...partial,
  };
}

/**
 * Intel-specific config with Xe-Slice hierarchy.
 */
export interface IntelGPUConfig extends DeviceConfig {
  readonly numXeSlices: number;
  readonly sliceConfig: XeSliceConfig;
}

export function makeIntelGPUConfig(
  partial: Partial<IntelGPUConfig> = {},
): IntelGPUConfig {
  const base = makeDeviceConfig({
    name: "Intel Arc A770",
    architecture: "intel_xe_core",
    ...partial,
  });
  return {
    ...base,
    numXeSlices: 8,
    sliceConfig: makeXeSliceConfig(),
    ...partial,
  };
}

/**
 * One ICI link to another TPU chip.
 */
export interface ICILink {
  readonly targetChipId: number;
  readonly bandwidth: number;
  readonly latency: number;
}

/**
 * TPU-specific config with Vector/Scalar units and ICI.
 */
export interface TPUConfig extends DeviceConfig {
  readonly vectorUnitWidth: number;
  readonly scalarRegisters: number;
  readonly transposeUnit: boolean;
  readonly iciLinks: readonly ICILink[];
}

export function makeTPUConfig(
  partial: Partial<TPUConfig> = {},
): TPUConfig {
  const base = makeDeviceConfig({
    name: "Google TPU v4",
    architecture: "google_mxu",
    ...partial,
  });
  return {
    ...base,
    vectorUnitWidth: 128,
    scalarRegisters: 32,
    transposeUnit: true,
    iciLinks: [],
    ...partial,
  };
}

/**
 * Apple ANE-specific config with DMA and SRAM.
 *
 * The ANE is unique: it shares unified memory with CPU and GPU,
 * eliminating the PCIe transfer bottleneck entirely. The 'copy'
 * operation just remaps page tables -- zero cycles, zero bytes moved.
 */
export interface ANEConfig extends DeviceConfig {
  readonly sharedSramSize: number;
  readonly sramBandwidth: number;
  readonly sramLatency: number;
  readonly dmaChannels: number;
  readonly dmaBandwidth: number;
}

export function makeANEConfig(
  partial: Partial<ANEConfig> = {},
): ANEConfig {
  const base = makeDeviceConfig({
    name: "Apple M3 Max ANE",
    architecture: "apple_ane_core",
    unifiedMemory: true,
    hostLatency: 0,
    ...partial,
  });
  return {
    ...base,
    sharedSramSize: 32 * 1024 * 1024,
    sramBandwidth: 1000.0,
    sramLatency: 5,
    dmaChannels: 4,
    dmaBandwidth: 100.0,
    ...partial,
  };
}

// =========================================================================
// DeviceTrace -- cycle-by-cycle visibility into the whole device
// =========================================================================

/**
 * One cycle of device-wide activity.
 *
 * === Why Trace the Whole Device? ===
 *
 * At the compute unit level (Layer 7), traces show what one SM/CU is doing.
 * At the device level, we need to see all compute units simultaneously, plus
 * the memory system and work distributor.
 *
 * Key questions a DeviceTrace answers:
 * - How many compute units are busy vs idle?
 * - Is the memory system a bottleneck (high bandwidth utilization)?
 * - Is the work distributor keeping up (many pending blocks)?
 * - What's the overall device occupancy?
 */
export interface DeviceTrace {
  readonly cycle: number;
  readonly deviceName: string;

  // Work distribution
  readonly distributorActions: readonly string[];
  readonly pendingBlocks: number;
  readonly activeBlocks: number;

  // Per-CU traces (can be empty for idle CUs)
  readonly cuTraces: readonly ComputeUnitTrace[];

  // Memory system
  readonly l2Hits: number;
  readonly l2Misses: number;
  readonly memoryTransactions: number;
  readonly memoryBandwidthUsed: number;

  // Aggregate metrics
  readonly totalActiveWarps: number;
  readonly deviceOccupancy: number;
  readonly flopsThisCycle: number;
}

/**
 * Create a DeviceTrace with sensible defaults.
 */
export function makeDeviceTrace(
  partial: Partial<DeviceTrace> & { cycle: number; deviceName: string },
): DeviceTrace {
  return {
    distributorActions: [],
    pendingBlocks: 0,
    activeBlocks: 0,
    cuTraces: [],
    l2Hits: 0,
    l2Misses: 0,
    memoryTransactions: 0,
    memoryBandwidthUsed: 0,
    totalActiveWarps: 0,
    deviceOccupancy: 0,
    flopsThisCycle: 0,
    ...partial,
  };
}

/**
 * Human-readable summary of a device trace.
 *
 * Example:
 *     [Cycle 10] NVIDIA H100 -- 45.2% occupancy
 *       Distributor: Block 42 -> SM 7, Block 43 -> SM 12
 *       Pending: 890 blocks, Active: 1056 blocks
 *       L2: 342 hits, 12 misses (96.6% hit rate)
 *       Memory: 8 transactions, 45.2% bandwidth
 *       Active warps: 4234
 */
export function formatDeviceTrace(trace: DeviceTrace): string {
  const lines: string[] = [
    `[Cycle ${trace.cycle}] ${trace.deviceName} -- ${(trace.deviceOccupancy * 100).toFixed(1)}% occupancy`,
  ];

  if (trace.distributorActions.length > 0) {
    const actionsStr = trace.distributorActions.join(", ");
    lines.push(`  Distributor: ${actionsStr}`);
  }

  lines.push(
    `  Pending: ${trace.pendingBlocks} blocks, Active: ${trace.activeBlocks} blocks`,
  );

  const totalL2 = trace.l2Hits + trace.l2Misses;
  if (totalL2 > 0) {
    const hitRate = (trace.l2Hits / totalL2) * 100;
    lines.push(
      `  L2: ${trace.l2Hits} hits, ${trace.l2Misses} misses (${hitRate.toFixed(1)}% hit rate)`,
    );
  }

  lines.push(
    `  Memory: ${trace.memoryTransactions} transactions, ${(trace.memoryBandwidthUsed * 100).toFixed(1)}% bandwidth`,
  );

  lines.push(`  Active warps: ${trace.totalActiveWarps}`);

  return lines.join("\n");
}

// =========================================================================
// DeviceStats -- aggregate metrics across the entire simulation
// =========================================================================

/**
 * Device-wide aggregate statistics.
 *
 * === Performance Analysis ===
 *
 * These stats answer the key performance questions:
 *
 * 1. **Compute utilization**: Are the compute units busy or sitting idle?
 * 2. **Memory bandwidth utilization**: Is the memory system saturated?
 * 3. **Load imbalance**: Are some CUs doing more work than others?
 * 4. **L2 effectiveness**: Is the cache helping?
 */
export interface DeviceStats {
  // Time
  totalCycles: number;
  activeCycles: number;
  idleCycles: number;

  // Compute
  totalFlops: number;
  achievedTflops: number;
  peakTflops: number;
  computeUtilization: number;

  // Memory
  globalMemoryStats: GlobalMemoryStats;
  l2HitRate: number;
  memoryBandwidthUtilization: number;

  // Work distribution
  totalKernelsLaunched: number;
  totalBlocksDispatched: number;
  avgBlocksPerCu: number;
  loadImbalance: number;

  // Per-CU breakdown
  perCuActiveCycles: number[];
  perCuOccupancy: number[];
}

/**
 * Create fresh DeviceStats with all counters at zero.
 */
export function makeDeviceStats(
  partial: Partial<DeviceStats> = {},
): DeviceStats {
  return {
    totalCycles: 0,
    activeCycles: 0,
    idleCycles: 0,
    totalFlops: 0,
    achievedTflops: 0,
    peakTflops: 0,
    computeUtilization: 0,
    globalMemoryStats: makeGlobalMemoryStats(),
    l2HitRate: 0,
    memoryBandwidthUtilization: 0,
    totalKernelsLaunched: 0,
    totalBlocksDispatched: 0,
    avgBlocksPerCu: 0,
    loadImbalance: 0,
    perCuActiveCycles: [],
    perCuOccupancy: [],
    ...partial,
  };
}

// =========================================================================
// AcceleratorDevice -- the unified device interface
// =========================================================================

/**
 * Any accelerator device: GPU, TPU, NPU.
 *
 * This is the top-level interface for Layer 6. The ISA Simulator (Layer 5)
 * and Runtime Simulator (Layer 4) will interact with devices through
 * this interface.
 *
 * Despite radical differences between a GPU (thread-parallel, thousands of
 * cores) and a TPU (dataflow, one large matrix unit), they share a common
 * lifecycle:
 *
 *     1. Allocate device memory
 *     2. Copy data from host to device
 *     3. Launch computation
 *     4. Wait for completion
 *     5. Copy results back to host
 */
export interface AcceleratorDevice {
  /** Device name ('NVIDIA H100', 'Apple M3 Max ANE', etc.). */
  readonly name: string;

  /** Full device configuration. */
  readonly config: DeviceConfig;

  // --- Memory management ---

  /** Allocate device memory. Returns device pointer (address). */
  malloc(size: number): number;

  /** Free device memory allocation. */
  free(address: number): void;

  /** Copy from host to device. Returns cycles consumed. */
  memcpyHostToDevice(dst: number, data: Uint8Array): number;

  /** Copy from device to host. Returns [data, cycles]. */
  memcpyDeviceToHost(src: number, size: number): [Uint8Array, number];

  // --- Kernel launch ---

  /** Submit a kernel for execution. */
  launchKernel(kernel: KernelDescriptor): void;

  // --- Simulation ---

  /** Advance the entire device by one clock cycle. */
  step(clockEdge?: ClockEdge): DeviceTrace;

  /** Run until all kernels complete or maxCycles reached. */
  run(maxCycles: number): DeviceTrace[];

  /** True when all CUs are idle and no pending work remains. */
  readonly idle: boolean;

  /** Reset all state -- CUs, memory, caches, work queues. */
  reset(): void;

  // --- Observability ---

  /** Aggregate statistics across all compute units and memory. */
  readonly stats: DeviceStats;

  /** Direct access to individual compute units. */
  readonly computeUnits: ComputeUnit[];
}
