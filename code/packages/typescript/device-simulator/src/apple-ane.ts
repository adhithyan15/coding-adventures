/**
 * Apple Neural Engine -- device simulator with unified memory.
 *
 * === Apple ANE Architecture ===
 *
 * The Apple Neural Engine is radically different from GPUs and TPUs.
 * It's a fixed-function accelerator designed for neural network inference,
 * optimized for power efficiency over flexibility.
 *
 *     +----------------------------------------------------+
 *     |           Apple Neural Engine                        |
 *     |                                                      |
 *     |  +------------------------------------------------+  |
 *     |  |       DMA Controller (schedule replayer)        |  |
 *     |  +------+-----+-----+------+---------------------+  |
 *     |         |     |     |      |                         |
 *     |  +------+ +------+ +------+ +------+                |
 *     |  |Core 0| |Core 1| |Core 2| |Core N|                |
 *     |  | MAC  | | MAC  | | MAC  | | MAC  |                |
 *     |  +--+---+ +--+---+ +--+---+ +--+---+                |
 *     |     +--------+--------+--------+                     |
 *     |              |                                        |
 *     |  +-----------+------------------------------------+  |
 *     |  |         Shared SRAM (32 MB)                    |  |
 *     |  +-----------+------------------------------------+  |
 *     |              |                                        |
 *     |  +-----------+------------------------------------+  |
 *     |  |   Unified Memory (shared with CPU & GPU)        |  |
 *     |  |   No copy needed -- just remap page tables      |  |
 *     |  +------------------------------------------------+  |
 *     +------------------------------------------------------+
 *
 * === Unified Memory: The Game Changer ===
 *
 * Apple's unified memory architecture means the ANE, CPU, and GPU all
 * share the same physical memory. When you "copy" data to the ANE, there's
 * no actual data movement -- the system just updates page table mappings.
 * This eliminates the PCIe bottleneck that plagues discrete GPUs:
 *
 *     Discrete GPU: Copy 8 MB over PCIe -> 125 us overhead
 *     Apple ANE:    Remap page tables -> ~0 us overhead
 *
 * === Compiler-Driven Scheduling ===
 *
 * Unlike GPUs (which have hardware warp schedulers) and TPUs (which have
 * a sequencer), the ANE relies entirely on the CoreML compiler to generate
 * a fixed execution schedule. The hardware simply replays this schedule.
 */

import { Clock } from "@coding-adventures/clock";
import type { ClockEdge } from "@coding-adventures/clock";
import {
  NeuralEngineCore,
  makeANECoreConfig,
  type ComputeUnit,
} from "@coding-adventures/compute-unit";

import { SimpleGlobalMemory } from "./global-memory.js";
import {
  type DeviceConfig,
  type DeviceStats,
  type DeviceTrace,
  type KernelDescriptor,
  type ANEConfig,
  makeDeviceStats,
  makeDeviceTrace,
} from "./protocols.js";
import { ANEScheduleReplayer } from "./work-distributor.js";

/**
 * Apple Neural Engine device simulator.
 *
 * Features unified memory (zero-copy host transfers), shared SRAM,
 * compiler-driven schedule replay, and DMA-based data movement.
 */
export class AppleANE {
  private readonly _config: DeviceConfig;
  private readonly _clock: Clock;
  private readonly _cores: NeuralEngineCore[];
  private readonly _globalMemory: SimpleGlobalMemory;
  private readonly _replayer: ANEScheduleReplayer;

  private _cycle: number;
  private _kernelsLaunched: number;

  constructor(opts: { config?: DeviceConfig; numCores?: number } = {}) {
    const numCores = opts.numCores ?? 4;

    if (opts.config) {
      this._config = opts.config;
    } else {
      const aneConfig: ANEConfig = {
        name: `Apple ANE (${numCores} cores)`,
        architecture: "apple_ane_core",
        numComputeUnits: numCores,
        cuConfig: null,
        l2CacheSize: 0,
        l2CacheLatency: 0,
        l2CacheAssociativity: 0,
        l2CacheLineSize: 128,
        globalMemorySize: 16 * 1024 * 1024,
        globalMemoryBandwidth: 200.0,
        globalMemoryLatency: 100,
        memoryChannels: 8,
        hostBandwidth: 200.0,
        hostLatency: 0,
        unifiedMemory: true,
        maxConcurrentKernels: 1,
        workDistributionPolicy: "scheduled",
        sharedSramSize: 4 * 1024 * 1024,
        sramBandwidth: 1000.0,
        sramLatency: 5,
        dmaChannels: 4,
        dmaBandwidth: 100.0,
      };
      this._config = aneConfig;
    }

    this._clock = new Clock(1_000_000_000);

    // Create NE cores
    const coreConfig = makeANECoreConfig();
    this._cores = [];
    for (let i = 0; i < this._config.numComputeUnits; i++) {
      this._cores.push(new NeuralEngineCore(coreConfig));
    }

    // Global memory (unified -- zero-copy)
    this._globalMemory = new SimpleGlobalMemory({
      capacity: this._config.globalMemorySize,
      bandwidth: this._config.globalMemoryBandwidth,
      latency: this._config.globalMemoryLatency,
      channels: this._config.memoryChannels,
      hostBandwidth: this._config.hostBandwidth,
      hostLatency: this._config.hostLatency,
      unified: this._config.unifiedMemory,
    });

    // Schedule replayer (compiler-driven)
    const isANEConfig = (c: DeviceConfig): c is ANEConfig =>
      "dmaBandwidth" in c;

    let dmaLatency = 10;
    let computeLatency = 20;
    if (isANEConfig(this._config)) {
      dmaLatency = Math.max(1, Math.floor(1024 / this._config.dmaBandwidth));
      computeLatency = 20;
    }

    this._replayer = new ANEScheduleReplayer(this._cores, {
      dmaLatency,
      computeLatency,
      activateLatency: 5,
    });

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

  /**
   * Copy from host -- zero-cost on unified memory!
   *
   * On Apple's unified memory, this doesn't actually copy data.
   * The CPU and ANE share the same physical memory. The 'copy'
   * just updates page table mappings.
   */
  memcpyHostToDevice(dst: number, data: Uint8Array): number {
    return this._globalMemory.copyFromHost(dst, data);
  }

  /** Copy to host -- zero-cost on unified memory! */
  memcpyDeviceToHost(src: number, size: number): [Uint8Array, number] {
    return this._globalMemory.copyToHost(src, size);
  }

  // --- Operation launch ---

  /**
   * Submit an operation to the schedule replayer.
   *
   * The compiler (us) generates a complete execution schedule
   * including DMA loads, compute, activation, and DMA stores.
   */
  launchKernel(kernel: KernelDescriptor): void {
    this._replayer.submitOperation(kernel);
    this._kernelsLaunched += 1;
  }

  // --- Simulation ---

  step(clockEdge?: ClockEdge): DeviceTrace {
    this._cycle += 1;
    const edge = clockEdge ?? this._clock.tick();

    // Replay the next step in the compiler-generated schedule
    const scheduleActions = this._replayer.step();

    // Step all cores
    const cuTraces = [];
    for (const core of this._cores) {
      const trace = core.step(edge);
      cuTraces.push(trace);
    }

    const activeCores = this._cores.filter((core) => !core.idle).length;

    return makeDeviceTrace({
      cycle: this._cycle,
      deviceName: this._config.name,
      distributorActions: scheduleActions,
      pendingBlocks: this._replayer.pendingCount,
      activeBlocks: activeCores,
      cuTraces,
      deviceOccupancy:
        this._cores.length > 0 ? activeCores / this._cores.length : 0.0,
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
    return this._replayer.idle;
  }

  reset(): void {
    for (const core of this._cores) {
      core.reset();
    }
    this._globalMemory.reset();
    this._replayer.reset();
    this._cycle = 0;
    this._kernelsLaunched = 0;
  }

  // --- Observability ---

  get stats(): DeviceStats {
    return makeDeviceStats({
      totalCycles: this._cycle,
      totalKernelsLaunched: this._kernelsLaunched,
      totalBlocksDispatched: this._replayer.totalDispatched,
      globalMemoryStats: this._globalMemory.stats,
    });
  }

  get computeUnits(): ComputeUnit[] {
    return [...this._cores];
  }

  get globalMemory(): SimpleGlobalMemory {
    return this._globalMemory;
  }

  /** True -- Apple ANE always uses unified memory. */
  get isUnifiedMemory(): boolean {
    return true;
  }
}
