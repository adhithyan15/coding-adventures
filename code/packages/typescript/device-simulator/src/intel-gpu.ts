/**
 * Intel GPU -- device simulator with Xe-Slices.
 *
 * === Intel GPU Architecture (Xe-HPG / Arc) ===
 *
 * Intel organizes Xe-Cores into **Xe-Slices**, with each slice sharing
 * a large L1 cache. This is similar to AMD's Shader Engines but at a
 * different granularity.
 *
 *     +----------------------------------------------------+
 *     |                Intel GPU                             |
 *     |  +------------------------------------------------+ |
 *     |  |     Command Streamer (distributor)              | |
 *     |  +------------------------+-----------------------+ |
 *     |                           |                          |
 *     |  +------------------------+---------------------+    |
 *     |  |         Xe-Slice 0                            |   |
 *     |  |  +---------+ +---------+ +---------+ +------+ |  |
 *     |  |  |XeCore 0 | |XeCore 1 | |XeCore 2 | |XeC 3| |  |
 *     |  |  +---------+ +---------+ +---------+ +------+ |  |
 *     |  |  L1 Cache (192 KB shared across Xe-Cores)      |  |
 *     |  +------------------------------------------------+  |
 *     |  ... (4-8 Xe-Slices)                                  |
 *     |                                                       |
 *     |  +--------------------------------------------------+ |
 *     |  |         L2 Cache (16 MB shared)                   | |
 *     |  +------------------------+-------------------------+ |
 *     |                           |                            |
 *     |  +------------------------+-------------------------+ |
 *     |  |        GDDR6 (16 GB, 512 GB/s)                   | |
 *     |  +--------------------------------------------------+ |
 *     +-------------------------------------------------------+
 */

import { Clock } from "@coding-adventures/clock";
import type { ClockEdge } from "@coding-adventures/clock";
import { Cache, CacheConfig } from "@coding-adventures/cache";
import {
  XeCore,
  makeXeCoreConfig,
  type ComputeUnit,
} from "@coding-adventures/compute-unit";

import { SimpleGlobalMemory } from "./global-memory.js";
import {
  type DeviceConfig,
  type DeviceStats,
  type DeviceTrace,
  type KernelDescriptor,
  type IntelGPUConfig,
  makeDeviceConfig,
  makeDeviceStats,
  makeDeviceTrace,
} from "./protocols.js";
import { GPUWorkDistributor } from "./work-distributor.js";

// =========================================================================
// XeSlice -- a group of Xe-Cores sharing an L1 cache
// =========================================================================

/**
 * A group of Xe-Cores sharing an L1 cache.
 *
 * In real Intel hardware, a Xe-Slice contains 4 Xe-Cores that share
 * a 192 KB L1 cache. The shared L1 enables cooperative data reuse.
 */
export class XeSlice {
  readonly sliceId: number;
  readonly xeCores: XeCore[];

  constructor(sliceId: number, xeCores: XeCore[]) {
    this.sliceId = sliceId;
    this.xeCores = xeCores;
  }

  get idle(): boolean {
    return this.xeCores.every((core) => core.idle);
  }
}

// =========================================================================
// IntelGPU -- the device simulator
// =========================================================================

/**
 * Intel GPU device simulator.
 *
 * Features Xe-Slice grouping, shared L1 per slice, L2 cache, and
 * the Command Streamer for work distribution.
 */
export class IntelGPU {
  private readonly _config: DeviceConfig;
  private readonly _clock: Clock;
  private readonly _allCores: XeCore[];
  private readonly _xeSlices: XeSlice[];
  private readonly _l2: Cache | null;
  private readonly _globalMemory: SimpleGlobalMemory;
  private readonly _distributor: GPUWorkDistributor;

  private _cycle: number;
  private _kernelsLaunched: number;

  constructor(opts: { config?: DeviceConfig; numCores?: number } = {}) {
    const numCores = opts.numCores ?? 4;

    if (opts.config) {
      this._config = opts.config;
    } else {
      this._config = makeDeviceConfig({
        name: `Intel GPU (${numCores} Xe-Cores)`,
        architecture: "intel_xe_core",
        numComputeUnits: numCores,
        l2CacheSize: 4096,
        l2CacheLatency: 180,
        l2CacheAssociativity: 4,
        l2CacheLineSize: 64,
        globalMemorySize: 16 * 1024 * 1024,
        globalMemoryBandwidth: 512.0,
        globalMemoryLatency: 350,
        memoryChannels: 4,
        hostBandwidth: 32.0,
        hostLatency: 100,
        unifiedMemory: false,
        maxConcurrentKernels: 16,
        workDistributionPolicy: "round_robin",
      });
    }

    this._clock = new Clock(2_100_000_000);

    // Create Xe-Cores
    const coreConfig = makeXeCoreConfig();
    this._allCores = [];
    for (let i = 0; i < this._config.numComputeUnits; i++) {
      this._allCores.push(new XeCore(coreConfig));
    }

    // Group into Xe-Slices
    const isIntelConfig = (c: DeviceConfig): c is IntelGPUConfig =>
      "numXeSlices" in c;
    const coresPerSlice = isIntelConfig(this._config)
      ? this._config.sliceConfig.xeCoresPerSlice
      : Math.max(1, Math.floor(this._config.numComputeUnits / 2));

    this._xeSlices = [];
    for (let i = 0; i < this._allCores.length; i += coresPerSlice) {
      const sliceCores = this._allCores.slice(i, i + coresPerSlice);
      this._xeSlices.push(new XeSlice(this._xeSlices.length, sliceCores));
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

    // Work distributor (Command Streamer)
    this._distributor = new GPUWorkDistributor(
      this._allCores,
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

    for (const core of this._allCores) {
      const trace = core.step(edge);
      cuTraces.push(trace);
      totalActiveWarps += trace.activeWarps;
      totalMaxWarps += trace.totalWarps;
    }

    const deviceOccupancy =
      totalMaxWarps > 0 ? totalActiveWarps / totalMaxWarps : 0.0;
    const activeBlocks = this._allCores.filter(
      (core) => !core.idle,
    ).length;

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
      this._allCores.every((core) => core.idle)
    );
  }

  reset(): void {
    for (const core of this._allCores) {
      core.reset();
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
    return [...this._allCores];
  }

  /** Access to Xe-Slices (Intel-specific). */
  get xeSlices(): XeSlice[] {
    return this._xeSlices;
  }

  get globalMemory(): SimpleGlobalMemory {
    return this._globalMemory;
  }
}
