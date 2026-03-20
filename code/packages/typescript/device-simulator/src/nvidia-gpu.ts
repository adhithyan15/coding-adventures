/**
 * NVIDIA GPU -- device simulator with GigaThread Engine.
 *
 * === NVIDIA GPU Architecture ===
 *
 * The NVIDIA GPU is the most widely-used accelerator for machine learning.
 * Its architecture is built around Streaming Multiprocessors (SMs), each
 * of which can independently schedule and execute thousands of threads.
 *
 *     +---------------------------------------------------+
 *     |                  NVIDIA GPU                        |
 *     |                                                    |
 *     |  +----------------------------------------------+  |
 *     |  |        GigaThread Engine (distributor)        |  |
 *     |  +----------------------+-----------------------+  |
 *     |                         |                          |
 *     |  +------+ +------+ +------+ ... +------+          |
 *     |  |SM 0  | |SM 1  | |SM 2  |     |SM N  |          |
 *     |  +--+---+ +--+---+ +--+---+     +--+---+          |
 *     |     +--------+--------+-----------+                |
 *     |                  |                                  |
 *     |  +---------------+------------------------------+  |
 *     |  |            L2 Cache (shared)                  |  |
 *     |  +---------------+------------------------------+  |
 *     |                  |                                  |
 *     |  +---------------+------------------------------+  |
 *     |  |          HBM3 (80 GB, 3.35 TB/s)             |  |
 *     |  +----------------------------------------------+  |
 *     +---------------------------------------------------+
 *
 * === GigaThread Engine ===
 *
 * The GigaThread Engine is the top-level work distributor. When a kernel
 * is launched, it creates thread blocks from the grid dimensions, assigns
 * blocks to SMs with available resources, and continues assigning as SMs
 * complete blocks (**multi-wave** execution).
 */

import { Clock } from "@coding-adventures/clock";
import type { ClockEdge } from "@coding-adventures/clock";
import { Cache, CacheConfig } from "@coding-adventures/cache";
import {
  StreamingMultiprocessor,
  makeSMConfig,
  type ComputeUnit,
} from "@coding-adventures/compute-unit";

import { SimpleGlobalMemory } from "./global-memory.js";
import {
  type DeviceConfig,
  type DeviceStats,
  type DeviceTrace,
  type KernelDescriptor,
  makeDeviceConfig,
  makeDeviceStats,
  makeDeviceTrace,
} from "./protocols.js";
import { GPUWorkDistributor } from "./work-distributor.js";

/**
 * NVIDIA GPU device simulator.
 *
 * Creates multiple SMs, an L2 cache, global memory (HBM), and a
 * GigaThread Engine to distribute thread blocks across SMs.
 *
 * Usage:
 *     import { NvidiaGPU, makeKernelDescriptor } from "@coding-adventures/device-simulator";
 *     import { limm, halt } from "@coding-adventures/gpu-core";
 *
 *     const gpu = new NvidiaGPU({ numSMs: 4 });
 *     const addr = gpu.malloc(1024);
 *     gpu.memcpyHostToDevice(addr, new Uint8Array(1024));
 *     gpu.launchKernel(makeKernelDescriptor({
 *         name: "saxpy",
 *         program: [limm(0, 2.0), halt()],
 *         gridDim: [4, 1, 1],
 *         blockDim: [32, 1, 1],
 *     }));
 *     const traces = gpu.run(1000);
 */
export class NvidiaGPU {
  private readonly _config: DeviceConfig;
  private readonly _clock: Clock;
  private readonly _sms: StreamingMultiprocessor[];
  private readonly _l2: Cache | null;
  private readonly _globalMemory: SimpleGlobalMemory;
  private readonly _distributor: GPUWorkDistributor;

  private _cycle: number;
  private _totalL2Hits: number;
  private _totalL2Misses: number;
  private _kernelsLaunched: number;

  constructor(opts: { config?: DeviceConfig; numSMs?: number } = {}) {
    const numSMs = opts.numSMs ?? 4;

    if (opts.config) {
      this._config = opts.config;
    } else {
      this._config = makeDeviceConfig({
        name: `NVIDIA GPU (${numSMs} SMs)`,
        architecture: "nvidia_sm",
        numComputeUnits: numSMs,
        l2CacheSize: 4096,
        l2CacheLatency: 200,
        l2CacheAssociativity: 4,
        l2CacheLineSize: 64,
        globalMemorySize: 16 * 1024 * 1024,
        globalMemoryBandwidth: 1000.0,
        globalMemoryLatency: 400,
        memoryChannels: 4,
        hostBandwidth: 64.0,
        hostLatency: 100,
        unifiedMemory: false,
        maxConcurrentKernels: 128,
        workDistributionPolicy: "round_robin",
      });
    }

    this._clock = new Clock(1_500_000_000);

    // Create SMs
    const smConfig = makeSMConfig({
      maxWarps: 8,
      numSchedulers: 2,
      sharedMemorySize: 4096,
      registerFileSize: 8192,
    });
    this._sms = [];
    for (let i = 0; i < this._config.numComputeUnits; i++) {
      this._sms.push(new StreamingMultiprocessor(smConfig));
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

    // Work distributor (GigaThread Engine)
    this._distributor = new GPUWorkDistributor(
      this._sms,
      this._config.workDistributionPolicy,
    );

    this._cycle = 0;
    this._totalL2Hits = 0;
    this._totalL2Misses = 0;
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

  /**
   * Advance the entire device by one clock cycle.
   *
   * 1. GigaThread assigns pending blocks to SMs with free resources
   * 2. Each SM steps (scheduler picks warps, engines execute)
   * 3. Collect traces from all SMs
   * 4. Build device-wide trace
   */
  step(clockEdge?: ClockEdge): DeviceTrace {
    this._cycle += 1;

    const edge = clockEdge ?? this._clock.tick();

    // 1. Distribute pending blocks to SMs
    const distActions = this._distributor.step();

    // 2. Step all SMs
    let totalActiveWarps = 0;
    let totalMaxWarps = 0;
    const cuTraces = [];

    for (const sm of this._sms) {
      const trace = sm.step(edge);
      cuTraces.push(trace);
      totalActiveWarps += trace.activeWarps;
      totalMaxWarps += trace.totalWarps;
    }

    // 3. Compute device-level metrics
    const deviceOccupancy =
      totalMaxWarps > 0 ? totalActiveWarps / totalMaxWarps : 0.0;

    const activeBlocks = this._sms.filter((sm) => !sm.idle).length;

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
      this._sms.every((sm) => sm.idle)
    );
  }

  reset(): void {
    for (const sm of this._sms) {
      sm.reset();
    }
    this._globalMemory.reset();
    this._distributor.reset();
    this._cycle = 0;
    this._totalL2Hits = 0;
    this._totalL2Misses = 0;
    this._kernelsLaunched = 0;
  }

  // --- Observability ---

  get stats(): DeviceStats {
    return makeDeviceStats({
      totalCycles: this._cycle,
      activeCycles: this._cycle,
      totalKernelsLaunched: this._kernelsLaunched,
      totalBlocksDispatched: this._distributor.totalDispatched,
      globalMemoryStats: this._globalMemory.stats,
    });
  }

  get computeUnits(): ComputeUnit[] {
    return [...this._sms];
  }

  get globalMemory(): SimpleGlobalMemory {
    return this._globalMemory;
  }
}
