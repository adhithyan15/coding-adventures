/**
 * Google TPU -- device simulator with Scalar/Vector/MXU pipeline.
 *
 * === TPU Architecture ===
 *
 * The TPU is fundamentally different from GPUs. Instead of thousands of
 * small cores executing thread programs, the TPU has:
 *
 * 1. **One large MXU** (Matrix Multiply Unit) -- a 128x128 systolic array
 *    that multiplies entire matrices in hardware.
 * 2. **A vector unit** -- handles element-wise operations (activation
 *    functions, normalization, softmax).
 * 3. **A scalar unit** -- handles control flow, address calculation,
 *    and loop counters.
 *
 * These three units form a **pipeline**: while the MXU processes one
 * matrix tile, the vector unit post-processes the previous tile, and
 * the scalar unit prepares the next tile.
 *
 *     +--------------------------------------------+
 *     |              Google TPU                      |
 *     |                                              |
 *     |  +----------------------------------------+  |
 *     |  |        Sequencer (control unit)         |  |
 *     |  +----+----------+----------+-------------+  |
 *     |       |          |          |                 |
 *     |  +----+--+  +----+----+  +-+------------+    |
 *     |  |Scalar |  | Vector  |  |    MXU       |    |
 *     |  | Unit  |  |  Unit   |  |  (128x128)   |    |
 *     |  +-------+  +---------+  +--------------+    |
 *     |                                               |
 *     |  +------------------------------------------+ |
 *     |  |      HBM2e (32 GB, 1.2 TB/s)             | |
 *     |  +------------------------------------------+ |
 *     +-----------------------------------------------+
 *
 * === No Thread Blocks ===
 *
 * TPUs don't have threads, warps, or thread blocks. The programming model
 * is completely different:
 *
 *     GPU: "Run this program on 65,536 threads"
 *     TPU: "Multiply this 1024x512 matrix by this 512x768 matrix"
 */

import { Clock } from "@coding-adventures/clock";
import type { ClockEdge } from "@coding-adventures/clock";
import {
  MatrixMultiplyUnit,
  makeMXUConfig,
  type ComputeUnit,
} from "@coding-adventures/compute-unit";

import { SimpleGlobalMemory } from "./global-memory.js";
import {
  type DeviceConfig,
  type DeviceStats,
  type DeviceTrace,
  type KernelDescriptor,
  type TPUConfig,
  makeDeviceStats,
  makeDeviceTrace,
} from "./protocols.js";
import { TPUSequencer } from "./work-distributor.js";

/**
 * Google TPU device simulator.
 *
 * Features a Scalar/Vector/MXU pipeline, HBM memory, and an optional
 * ICI interconnect for multi-chip communication.
 */
export class GoogleTPU {
  private readonly _config: DeviceConfig;
  private readonly _clock: Clock;
  private readonly _mxu: MatrixMultiplyUnit;
  private readonly _sequencer: TPUSequencer;
  private readonly _globalMemory: SimpleGlobalMemory;

  private _cycle: number;
  private _kernelsLaunched: number;

  constructor(opts: { config?: DeviceConfig; mxuSize?: number } = {}) {
    const mxuSize = opts.mxuSize ?? 4;

    if (opts.config) {
      this._config = opts.config;
    } else {
      this._config = {
        name: `Google TPU (MXU ${mxuSize}x${mxuSize})`,
        architecture: "google_mxu",
        numComputeUnits: 1,
        cuConfig: null,
        l2CacheSize: 0,
        l2CacheLatency: 0,
        l2CacheAssociativity: 0,
        l2CacheLineSize: 128,
        globalMemorySize: 16 * 1024 * 1024,
        globalMemoryBandwidth: 1200.0,
        globalMemoryLatency: 300,
        memoryChannels: 4,
        hostBandwidth: 500.0,
        hostLatency: 100,
        unifiedMemory: false,
        maxConcurrentKernels: 1,
        workDistributionPolicy: "sequential",
      } satisfies DeviceConfig & { vectorUnitWidth?: number };
    }

    this._clock = new Clock(1_000_000_000);

    // Create MXU
    const mxuConfig = makeMXUConfig();
    this._mxu = new MatrixMultiplyUnit(mxuConfig);

    // Sequencer
    const isTPUConfig = (c: DeviceConfig): c is TPUConfig =>
      "vectorUnitWidth" in c;
    const vecWidth = isTPUConfig(this._config)
      ? this._config.vectorUnitWidth
      : mxuSize;

    this._sequencer = new TPUSequencer(this._mxu, {
      mxuSize,
      vectorWidth: vecWidth,
      scalarLatency: 5,
      mxuLatency: 20,
      vectorLatency: 10,
    });

    // Global memory (HBM)
    this._globalMemory = new SimpleGlobalMemory({
      capacity: this._config.globalMemorySize,
      bandwidth: this._config.globalMemoryBandwidth,
      latency: this._config.globalMemoryLatency,
      channels: this._config.memoryChannels,
      hostBandwidth: this._config.hostBandwidth,
      hostLatency: this._config.hostLatency,
      unified: this._config.unifiedMemory,
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

  memcpyHostToDevice(dst: number, data: Uint8Array): number {
    return this._globalMemory.copyFromHost(dst, data);
  }

  memcpyDeviceToHost(src: number, size: number): [Uint8Array, number] {
    return this._globalMemory.copyToHost(src, size);
  }

  // --- Operation launch ---

  launchKernel(kernel: KernelDescriptor): void {
    this._sequencer.submitOperation(kernel);
    this._kernelsLaunched += 1;
  }

  // --- Simulation ---

  step(clockEdge?: ClockEdge): DeviceTrace {
    this._cycle += 1;
    const edge = clockEdge ?? this._clock.tick();

    // Advance the Scalar -> MXU -> Vector pipeline
    const seqActions = this._sequencer.step();

    // Also step the MXU compute unit
    const cuTrace = this._mxu.step(edge);

    return makeDeviceTrace({
      cycle: this._cycle,
      deviceName: this._config.name,
      distributorActions: seqActions,
      pendingBlocks: this._sequencer.pendingCount,
      activeBlocks: this._sequencer.idle ? 0 : 1,
      cuTraces: [cuTrace],
      deviceOccupancy: this._sequencer.idle ? 0.0 : 1.0,
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
    return this._sequencer.idle;
  }

  reset(): void {
    this._mxu.reset();
    this._sequencer.reset();
    this._globalMemory.reset();
    this._cycle = 0;
    this._kernelsLaunched = 0;
  }

  // --- Observability ---

  get stats(): DeviceStats {
    return makeDeviceStats({
      totalCycles: this._cycle,
      totalKernelsLaunched: this._kernelsLaunched,
      totalBlocksDispatched: this._sequencer.totalDispatched,
      globalMemoryStats: this._globalMemory.stats,
    });
  }

  get computeUnits(): ComputeUnit[] {
    return [this._mxu];
  }

  get globalMemory(): SimpleGlobalMemory {
    return this._globalMemory;
  }
}
