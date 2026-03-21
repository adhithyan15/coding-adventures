/**
 * MultiCoreCPU -- multiple cores sharing L3 cache and memory.
 *
 * Each core has private L1I, L1D, and optional L2 caches.
 * All cores share an optional L3 cache, main memory, and an
 * interrupt controller.
 *
 * Architecture:
 *
 *     Core 0: L1I + L1D + L2 (private)
 *     Core 1: L1I + L1D + L2 (private)
 *             |         |
 *        ========================
 *        Shared L3 Cache (optional)
 *        ========================
 *                  |
 *        Memory Controller
 *                  |
 *        Shared Main Memory
 */

import { Cache } from "@coding-adventures/cache";
import type { PipelineSnapshot } from "@coding-adventures/cpu-pipeline";

import type { MultiCoreConfig } from "./config.js";
import { Core } from "./core.js";
import type { ISADecoder } from "./decoder.js";
import { InterruptController } from "./interrupt-controller.js";
import { MemoryController } from "./memory-controller.js";
import { CoreStats } from "./stats.js";

export class MultiCoreCPU {
  private _config: MultiCoreConfig;
  private _cores: Core[];
  private _sharedMemory: Uint8Array;
  private _memCtrl: MemoryController;
  private _l3Cache: Cache | null;
  private _interruptCtrl: InterruptController;
  private _cycle: number = 0;

  private constructor(
    config: MultiCoreConfig,
    cores: Core[],
    sharedMemory: Uint8Array,
    memCtrl: MemoryController,
    l3Cache: Cache | null,
    interruptCtrl: InterruptController,
  ) {
    this._config = config;
    this._cores = cores;
    this._sharedMemory = sharedMemory;
    this._memCtrl = memCtrl;
    this._l3Cache = l3Cache;
    this._interruptCtrl = interruptCtrl;
  }

  /**
   * Creates a multi-core processor.
   *
   * All cores share the same main memory. Each core gets its own ISA decoder
   * from the decoders array. If fewer decoders than cores, the last is reused.
   */
  static create(config: MultiCoreConfig, decoders: ISADecoder[]): MultiCoreCPU {
    let memSize = config.memorySize;
    if (memSize <= 0) memSize = 1048576;
    const sharedMemory = new Uint8Array(memSize);

    let memLatency = config.memoryLatency;
    if (memLatency <= 0) memLatency = 100;
    const memCtrl = new MemoryController(sharedMemory, memLatency);

    let l3: Cache | null = null;
    if (config.l3Cache) {
      l3 = new Cache(config.l3Cache);
    }

    let numCores = config.numCores;
    if (numCores <= 0) numCores = 1;

    const cores: Core[] = [];
    for (let i = 0; i < numCores; i++) {
      const decoder = i < decoders.length ? decoders[i] : decoders[0];

      const coreCfg = { ...config.coreConfig };
      coreCfg.memorySize = memSize;
      coreCfg.memoryLatency = memLatency;

      const c = Core.create(coreCfg, decoder);
      // Replace the core's memory controller with the shared one.
      c._setMemCtrl(memCtrl);
      cores.push(c);
    }

    return new MultiCoreCPU(
      config,
      cores,
      sharedMemory,
      memCtrl,
      l3,
      new InterruptController(numCores),
    );
  }

  /** Loads a program into shared memory for a specific core. */
  loadProgram(coreID: number, program: Uint8Array, startAddress: number): void {
    if (coreID < 0 || coreID >= this._cores.length) return;
    this._memCtrl.loadProgram(program, startAddress);
    this._cores[coreID].pipeline().setPC(startAddress);
  }

  /** Advances all cores by one clock cycle. */
  step(): PipelineSnapshot[] {
    this._cycle++;
    const snapshots: PipelineSnapshot[] = [];
    for (const c of this._cores) {
      snapshots.push(c.step());
    }
    this._memCtrl.tick();
    return snapshots;
  }

  /** Runs all cores until all have halted or maxCycles is reached. */
  run(maxCycles: number): CoreStats[] {
    while (this._cycle < maxCycles) {
      let allHalted = true;
      for (const c of this._cores) {
        if (!c.isHalted()) {
          allHalted = false;
          break;
        }
      }
      if (allHalted) break;
      this.step();
    }
    return this.stats();
  }

  /** Returns the array of cores. */
  cores(): Core[] { return this._cores; }

  /** Returns per-core statistics. */
  stats(): CoreStats[] {
    return this._cores.map(c => c.stats());
  }

  /** Returns the interrupt controller. */
  interruptController(): InterruptController { return this._interruptCtrl; }

  /** Returns the shared memory controller. */
  sharedMemoryController(): MemoryController { return this._memCtrl; }

  /** Returns the global cycle count. */
  cycle(): number { return this._cycle; }

  /** Returns true if every core has halted. */
  allHalted(): boolean {
    return this._cores.every(c => c.isHalted());
  }
}
