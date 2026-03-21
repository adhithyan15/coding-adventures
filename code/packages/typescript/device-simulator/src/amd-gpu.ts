/**
 * AMD GPU -- device simulator with Shader Engines and Infinity Cache.
 *
 * === AMD GPU Architecture ===
 *
 * AMD organizes compute units (CUs) into **Shader Engines** (SEs). This is
 * a mid-level hierarchy that NVIDIA doesn't have -- CUs within the same SE
 * share a geometry processor and rasterizer (for graphics), and for compute
 * workloads, the Command Processor assigns entire work-groups to SEs first.
 *
 *     +------------------------------------------------------+
 *     |                    AMD GPU                             |
 *     |  +--------------------------------------------------+ |
 *     |  |       Command Processor (distributor)             | |
 *     |  +------------------------+-------------------------+ |
 *     |                           |                            |
 *     |  +------------------------+----------------------+     |
 *     |  |      Shader Engine 0                           |    |
 *     |  |  +-----+ +-----+ ... +-----+                  |    |
 *     |  |  |CU 0 | |CU 1 |     |CU N |                  |    |
 *     |  |  +-----+ +-----+     +-----+                  |    |
 *     |  +-----------------------------------------------|    |
 *     |  ... more Shader Engines                               |
 *     |                                                        |
 *     |  +--------------------------------------------------+ |
 *     |  |     Infinity Cache (96 MB, ~50 cycle latency)     | |
 *     |  +------------------------+-------------------------+ |
 *     |                           |                            |
 *     |  +------------------------+-------------------------+ |
 *     |  |           GDDR6 (24 GB, 960 GB/s)                | |
 *     |  +--------------------------------------------------+ |
 *     +------------------------------------------------------+
 *
 * === Infinity Cache ===
 *
 * AMD's Infinity Cache is a large last-level cache (96 MB on RX 7900 XTX).
 * It dramatically reduces the effective memory bandwidth requirement.
 */

import { Clock } from "@coding-adventures/clock";
import type { ClockEdge } from "@coding-adventures/clock";
import { Cache, CacheConfig } from "@coding-adventures/cache";
import {
  AMDComputeUnit,
  makeAMDCUConfig,
  type ComputeUnit,
} from "@coding-adventures/compute-unit";

import { SimpleGlobalMemory } from "./global-memory.js";
import {
  type DeviceConfig,
  type DeviceStats,
  type DeviceTrace,
  type KernelDescriptor,
  type AmdGPUConfig,
  type ShaderEngineConfig,
  makeDeviceConfig,
  makeDeviceStats,
  makeDeviceTrace,
} from "./protocols.js";
import { GPUWorkDistributor } from "./work-distributor.js";

// =========================================================================
// ShaderEngine -- a group of CUs that share resources
// =========================================================================

/**
 * A group of CUs that share resources.
 *
 * In a real AMD GPU, a Shader Engine shares a geometry processor,
 * rasterizer, and some L1 cache. For compute workloads, it mainly
 * affects how the Command Processor assigns work.
 */
export class ShaderEngine {
  readonly engineId: number;
  readonly cus: AMDComputeUnit[];

  constructor(engineId: number, cus: AMDComputeUnit[]) {
    this.engineId = engineId;
    this.cus = cus;
  }

  get idle(): boolean {
    return this.cus.every((cu) => cu.idle);
  }
}

// =========================================================================
// AmdGPU -- the device simulator
// =========================================================================

/**
 * AMD GPU device simulator.
 *
 * Features Shader Engine grouping, Infinity Cache, and multi-queue
 * dispatch via ACEs.
 */
export class AmdGPU {
  private readonly _config: DeviceConfig;
  private readonly _clock: Clock;
  private readonly _allCUs: AMDComputeUnit[];
  private readonly _shaderEngines: ShaderEngine[];
  private readonly _infinityCache: Cache | null;
  private readonly _l2: Cache | null;
  private readonly _globalMemory: SimpleGlobalMemory;
  private readonly _distributor: GPUWorkDistributor;

  private _cycle: number;
  private _kernelsLaunched: number;

  constructor(opts: { config?: DeviceConfig; numCUs?: number } = {}) {
    const numCUs = opts.numCUs ?? 4;

    if (opts.config) {
      this._config = opts.config;
    } else {
      this._config = makeDeviceConfig({
        name: `AMD GPU (${numCUs} CUs)`,
        architecture: "amd_cu",
        numComputeUnits: numCUs,
        l2CacheSize: 4096,
        l2CacheLatency: 150,
        l2CacheAssociativity: 4,
        l2CacheLineSize: 64,
        globalMemorySize: 16 * 1024 * 1024,
        globalMemoryBandwidth: 960.0,
        globalMemoryLatency: 350,
        memoryChannels: 4,
        hostBandwidth: 32.0,
        hostLatency: 100,
        unifiedMemory: false,
        maxConcurrentKernels: 8,
        workDistributionPolicy: "round_robin",
      });
    }

    this._clock = new Clock(1_800_000_000);

    // Create CUs
    const cuConfig = makeAMDCUConfig();
    this._allCUs = [];
    for (let i = 0; i < this._config.numComputeUnits; i++) {
      this._allCUs.push(new AMDComputeUnit(cuConfig));
    }

    // Group into Shader Engines
    const isAmdConfig = (c: DeviceConfig): c is AmdGPUConfig =>
      "numShaderEngines" in c;
    const seSize = isAmdConfig(this._config)
      ? this._config.seConfig.cusPerEngine
      : Math.max(1, Math.floor(this._config.numComputeUnits / 2));

    this._shaderEngines = [];
    for (let i = 0; i < this._allCUs.length; i += seSize) {
      const seCUs = this._allCUs.slice(i, i + seSize);
      this._shaderEngines.push(
        new ShaderEngine(this._shaderEngines.length, seCUs),
      );
    }

    // Infinity Cache
    if (
      isAmdConfig(this._config) &&
      this._config.infinityCacheSize > 0
    ) {
      const icSize = Math.min(
        1 << (Math.ceil(Math.log2(this._config.infinityCacheSize)) - 1),
        4096,
      );
      this._infinityCache = new Cache(
        new CacheConfig(
          "InfinityCache",
          Math.max(icSize, 64),
          64,
          16,
          this._config.infinityCacheLatency,
        ),
      );
    } else {
      this._infinityCache = null;
    }

    // L2 cache
    if (this._config.l2CacheSize > 0) {
      this._l2 = new Cache(
        new CacheConfig(
          "L2",
          this._config.l2CacheSize,
          this._config.l2CacheLineSize,
          this._config.l2CacheAssociativity,
          this._config.l2CacheLatency,
        ),
      );
    } else {
      this._l2 = null;
    }

    // Global memory
    this._globalMemory = new SimpleGlobalMemory({
      capacity: this._config.globalMemorySize,
      bandwidth: this._config.globalMemoryBandwidth,
      latency: this._config.globalMemoryLatency,
      channels: this._config.memoryChannels,
      hostBandwidth: this._config.hostBandwidth,
      hostLatency: this._config.hostLatency,
      unified: this._config.unifiedMemory,
    });

    // Work distributor (Command Processor)
    this._distributor = new GPUWorkDistributor(
      this._allCUs,
      this._config.workDistributionPolicy,
    );

    this._cycle = 0;
    this._kernelsLaunched = 0;
  }

  // --- Identity ---

  get name(): string {
    return this._config.name;
  }

  get config(): DeviceConfig {
    return this._config;
  }

  // --- Memory management ---

  malloc(size: number): number {
    return this._globalMemory.allocate(size);
  }

  free(address: number): void {
    this._globalMemory.free(address);
  }

  memcpyHostToDevice(dst: number, data: Uint8Array): number {
    return this._globalMemory.copyFromHost(dst, data);
  }

  memcpyDeviceToHost(src: number, size: number): [Uint8Array, number] {
    return this._globalMemory.copyToHost(src, size);
  }

  // --- Kernel launch ---

  launchKernel(kernel: KernelDescriptor): void {
    this._distributor.submitKernel(kernel);
    this._kernelsLaunched += 1;
  }

  // --- Simulation ---

  step(clockEdge?: ClockEdge): DeviceTrace {
    this._cycle += 1;
    const edge = clockEdge ?? this._clock.tick();

    const distActions = this._distributor.step();

    let totalActiveWarps = 0;
    let totalMaxWarps = 0;
    const cuTraces = [];

    for (const cu of this._allCUs) {
      const trace = cu.step(edge);
      cuTraces.push(trace);
      totalActiveWarps += trace.activeWarps;
      totalMaxWarps += trace.totalWarps;
    }

    const deviceOccupancy =
      totalMaxWarps > 0 ? totalActiveWarps / totalMaxWarps : 0.0;
    const activeBlocks = this._allCUs.filter((cu) => !cu.idle).length;

    return makeDeviceTrace({
      cycle: this._cycle,
      deviceName: this._config.name,
      distributorActions: distActions,
      pendingBlocks: this._distributor.pendingCount,
      activeBlocks,
      cuTraces,
      totalActiveWarps,
      deviceOccupancy,
    });
  }

  run(maxCycles: number = 10000): DeviceTrace[] {
    const traces: DeviceTrace[] = [];
    for (let i = 0; i < maxCycles; i++) {
      const trace = this.step();
      traces.push(trace);
      if (this.idle) break;
    }
    return traces;
  }

  get idle(): boolean {
    return (
      this._distributor.pendingCount === 0 &&
      this._allCUs.every((cu) => cu.idle)
    );
  }

  reset(): void {
    for (const cu of this._allCUs) {
      cu.reset();
    }
    this._globalMemory.reset();
    this._distributor.reset();
    this._cycle = 0;
    this._kernelsLaunched = 0;
  }

  // --- Observability ---

  get stats(): DeviceStats {
    return makeDeviceStats({
      totalCycles: this._cycle,
      totalKernelsLaunched: this._kernelsLaunched,
      totalBlocksDispatched: this._distributor.totalDispatched,
      globalMemoryStats: this._globalMemory.stats,
    });
  }

  get computeUnits(): ComputeUnit[] {
    return [...this._allCUs];
  }

  /** Access to Shader Engines (AMD-specific). */
  get shaderEngines(): ShaderEngine[] {
    return this._shaderEngines;
  }

  get globalMemory(): SimpleGlobalMemory {
    return this._globalMemory;
  }
}
