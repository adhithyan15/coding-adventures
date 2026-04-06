/**
 * WavefrontEngine -- SIMD parallel execution (AMD GCN/RDNA style).
 *
 * === What is a Wavefront? ===
 *
 * AMD calls their parallel execution unit a "wavefront." It's 64 lanes on GCN
 * (Graphics Core Next) or 32 lanes on RDNA (Radeon DNA). A wavefront is
 * fundamentally different from an NVIDIA warp:
 *
 *     NVIDIA Warp (SIMT):                AMD Wavefront (SIMD):
 *     +--------------------------+       +--------------------------+
 *     | 32 threads               |       | 32 lanes                 |
 *     | Each has its own regs    |       | ONE vector register file  |
 *     | Logically own PC         |       | ONE program counter       |
 *     | HW manages divergence    |       | Explicit EXEC mask        |
 *     +--------------------------+       +--------------------------+
 *
 * The critical architectural difference:
 *
 *     SIMT (NVIDIA): "32 independent threads that HAPPEN to run together"
 *     SIMD (AMD):    "1 instruction that operates on a 32-wide vector"
 *
 * In SIMT, thread 7 has its own R0 register. In SIMD, there IS no "thread 7"
 * -- there's lane 7 of vector register v0, which is v0[7].
 *
 * === AMD's Two Register Files ===
 *
 * AMD wavefronts have TWO types of registers, which is architecturally unique:
 *
 *     Vector GPRs (VGPRs):              Scalar GPRs (SGPRs):
 *     +------------------------+        +------------------------+
 *     | v0: [l0][l1]...[l31]  |        | s0:  42.0              |
 *     | v1: [l0][l1]...[l31]  |        | s1:  3.14              |
 *     | ...                    |        | ...                    |
 *     | v255:[l0][l1]...[l31] |        | s103: 0.0              |
 *     +------------------------+        +------------------------+
 *     One value PER LANE                One value for ALL LANES
 *
 * SGPRs are used for values that are the SAME across all lanes: constants,
 * loop counters, memory base addresses. This is efficient -- compute the
 * address ONCE in scalar, then use it in every lane.
 *
 * === The EXEC Mask ===
 *
 * AMD uses a register called EXEC to control which lanes execute each
 * instruction. Unlike NVIDIA's hardware-managed divergence, the EXEC mask
 * is explicitly set by instructions.
 */

import {
  type FloatFormat,
  type FloatBits,
  FP32,
  floatToBits,
  bitsToFloat,
} from "@coding-adventures/fp-arithmetic";

import {
  GPUCore,
  GenericISA,
  type Instruction,
  type InstructionSet,
} from "@coding-adventures/gpu-core";

import {
  type DivergenceInfo,
  type EngineTrace,
  ExecutionModel,
} from "./protocols.js";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/**
 * Configuration for an AMD-style SIMD wavefront engine.
 *
 * Real-world reference values:
 *
 *     Architecture | Wave Width | VGPRs | SGPRs | LDS
 *     -------------+------------+-------+-------+---------
 *     AMD GCN      | 64         | 256   | 104   | 64 KB
 *     AMD RDNA     | 32         | 256   | 104   | 64 KB
 *     Our default  | 32         | 256   | 104   | 64 KB
 */
export interface WavefrontConfig {
  readonly waveWidth: number;
  readonly numVgprs: number;
  readonly numSgprs: number;
  readonly ldsSize: number;
  readonly floatFormat: FloatFormat;
  readonly isa: InstructionSet;
}

function assertRegisterIndex(index: number, limit: number, label: string): void {
  if (!Number.isSafeInteger(index) || index < 0 || index >= limit) {
    throw new Error(`${label} index ${String(index)} out of bounds for size ${limit}.`);
  }
}

function materializeUnitTraces(
  laneTraces: ReadonlyArray<string | undefined>,
): Record<number, string> {
  const entries: Array<readonly [number, string]> = [];
  for (let laneId = 0; laneId < laneTraces.length; laneId++) {
    const trace = laneTraces[laneId];
    if (trace !== undefined) {
      entries.push([laneId, trace]);
    }
  }
  return Object.fromEntries(entries) as Record<number, string>;
}

/**
 * Create a WavefrontConfig with sensible defaults.
 */
export function makeWavefrontConfig(
  partial: Partial<WavefrontConfig> = {},
): WavefrontConfig {
  return {
    waveWidth: 32,
    numVgprs: 256,
    numSgprs: 104,
    ldsSize: 65536,
    floatFormat: FP32,
    isa: new GenericISA(),
    ...partial,
  };
}

// ---------------------------------------------------------------------------
// Vector Register File -- one value per lane per register
// ---------------------------------------------------------------------------

/**
 * AMD-style vector register file: numVgprs registers x waveWidth lanes.
 *
 * Each "register" is actually a vector of waveWidth values. When you
 * write to v3[lane 5], you're writing to one slot in a 2D array:
 *
 *     +--------------------------------------------+
 *     |         Lane 0   Lane 1   Lane 2  ...      |
 *     | v0:    [ 1.0  ] [ 2.0  ] [ 3.0  ]  ...    |
 *     | v1:    [ 0.5  ] [ 0.5  ] [ 0.5  ]  ...    |
 *     | v2:    [ 0.0  ] [ 0.0  ] [ 0.0  ]  ...    |
 *     | ...                                        |
 *     +--------------------------------------------+
 *
 * This is fundamentally different from NVIDIA where each thread has
 * its own separate register file.
 */
export class VectorRegisterFile {
  readonly numVgprs: number;
  readonly waveWidth: number;
  readonly fmt: FloatFormat;
  private _data: FloatBits[][];

  constructor(
    numVgprs: number,
    waveWidth: number,
    fmt: FloatFormat = FP32,
  ) {
    this.numVgprs = numVgprs;
    this.waveWidth = waveWidth;
    this.fmt = fmt;
    // 2D storage: _data[reg_index][lane_index] = FloatBits
    this._data = Array.from({ length: numVgprs }, () =>
      Array.from({ length: waveWidth }, () => floatToBits(0.0, fmt)),
    );
  }

  /** Read one lane of a vector register as a number. */
  read(vreg: number, lane: number): number {
    assertRegisterIndex(vreg, this._data.length, "vector register");
    assertRegisterIndex(lane, this.waveWidth, "lane");
    return bitsToFloat(this._data[vreg][lane]);
  }

  /** Write a number to one lane of a vector register. */
  write(vreg: number, lane: number, value: number): void {
    assertRegisterIndex(vreg, this._data.length, "vector register");
    assertRegisterIndex(lane, this.waveWidth, "lane");
    this._data[vreg][lane] = floatToBits(value, this.fmt);
  }

  /** Read all lanes of a vector register. */
  readAllLanes(vreg: number): number[] {
    assertRegisterIndex(vreg, this._data.length, "vector register");
    return this._data[vreg].map((bits) => bitsToFloat(bits));
  }
}

// ---------------------------------------------------------------------------
// Scalar Register File -- one value shared across all lanes
// ---------------------------------------------------------------------------

/**
 * AMD-style scalar register file: numSgprs single-value registers.
 *
 * Scalar registers hold values that are the SAME for all lanes:
 * constants, loop counters, memory base addresses.
 *
 *     +--------------------------+
 *     | s0:   42.0               |  <- same for all lanes
 *     | s1:   3.14159            |
 *     | s2:   0.0                |
 *     | ...                      |
 *     | s103: 0.0                |
 *     +--------------------------+
 */
export class ScalarRegisterFile {
  readonly numSgprs: number;
  readonly fmt: FloatFormat;
  private _data: FloatBits[];

  constructor(numSgprs: number, fmt: FloatFormat = FP32) {
    this.numSgprs = numSgprs;
    this.fmt = fmt;
    this._data = Array.from({ length: numSgprs }, () =>
      floatToBits(0.0, fmt),
    );
  }

  /** Read a scalar register as a number. */
  read(sreg: number): number {
    return bitsToFloat(this._data[sreg]);
  }

  /** Write a number to a scalar register. */
  write(sreg: number, value: number): void {
    this._data[sreg] = floatToBits(value, this.fmt);
  }
}

// ---------------------------------------------------------------------------
// WavefrontEngine -- the SIMD parallel execution engine
// ---------------------------------------------------------------------------

/**
 * SIMD wavefront execution engine (AMD GCN/RDNA style).
 *
 * One instruction stream, one wide vector ALU, explicit EXEC mask.
 * Internally uses GPUCore per lane for instruction execution, but
 * exposes the AMD-style vector/scalar register interface.
 *
 * === Key Differences from WarpEngine ===
 *
 * 1. ONE program counter (not per-thread PCs).
 * 2. Vector registers are a 2D array (vreg x lane), not per-thread.
 * 3. Scalar registers are shared across all lanes.
 * 4. EXEC mask is explicitly controlled, not hardware-managed.
 * 5. No divergence stack -- mask management is programmer/compiler's job.
 */
export class WavefrontEngine {
  private readonly _config: WavefrontConfig;
  private _cycle: number = 0;
  private _program: Instruction[] = [];
  private _execMask: boolean[];
  private _vrf: VectorRegisterFile;
  private _srf: ScalarRegisterFile;
  private _lanes: GPUCore[];
  private _allHalted: boolean = false;

  constructor(config: WavefrontConfig) {
    this._config = config;

    // The EXEC mask: true = lane is active, false = lane is masked off.
    this._execMask = Array.from({ length: config.waveWidth }, () => true);

    // Vector and scalar register files (AMD-style)
    this._vrf = new VectorRegisterFile(
      config.numVgprs,
      config.waveWidth,
      config.floatFormat,
    );
    this._srf = new ScalarRegisterFile(config.numSgprs, config.floatFormat);

    // Internal: one GPUCore per lane for instruction execution.
    this._lanes = Array.from(
      { length: config.waveWidth },
      () =>
        new GPUCore({
          isa: config.isa,
          fmt: config.floatFormat,
          numRegisters: Math.min(config.numVgprs, 256),
          memorySize: Math.floor(
            config.ldsSize / Math.max(config.waveWidth, 1),
          ),
        }),
    );
  }

  // --- Properties ---

  get name(): string {
    return "WavefrontEngine";
  }

  get width(): number {
    return this._config.waveWidth;
  }

  get executionModel(): ExecutionModel {
    return ExecutionModel.SIMD;
  }

  /** The current EXEC mask (which lanes are active). */
  get execMask(): boolean[] {
    return [...this._execMask];
  }

  get halted(): boolean {
    return this._allHalted;
  }

  get config(): WavefrontConfig {
    return this._config;
  }

  /** Access to the vector register file. */
  get vrf(): VectorRegisterFile {
    return this._vrf;
  }

  /** Access to the scalar register file. */
  get srf(): ScalarRegisterFile {
    return this._srf;
  }

  // --- Program loading ---

  loadProgram(program: Instruction[]): void {
    this._program = [...program];
    for (const lane of this._lanes) {
      lane.loadProgram(this._program);
    }
    this._execMask = Array.from(
      { length: this._config.waveWidth },
      () => true,
    );
    this._allHalted = false;
    this._cycle = 0;
  }

  // --- Register setup ---

  /**
   * Set a per-lane vector register value.
   *
   * Writes to both the VRF (our AMD-style register file) and
   * the internal GPUCore for that lane (for execution).
   */
  setLaneRegister(lane: number, vreg: number, value: number): void {
    if (lane < 0 || lane >= this._config.waveWidth) {
      throw new RangeError(
        `Lane ${lane} out of range [0, ${this._config.waveWidth})`,
      );
    }
    this._vrf.write(vreg, lane, value);
    this._lanes[lane].registers.writeFloat(vreg, value);
  }

  /**
   * Set a scalar register value (shared across all lanes).
   */
  setScalarRegister(sreg: number, value: number): void {
    if (sreg < 0 || sreg >= this._config.numSgprs) {
      throw new RangeError(
        `Scalar register ${sreg} out of range [0, ${this._config.numSgprs})`,
      );
    }
    this._srf.write(sreg, value);
  }

  /**
   * Explicitly set the EXEC mask.
   */
  setExecMask(mask: boolean[]): void {
    if (mask.length !== this._config.waveWidth) {
      throw new Error(
        `Mask length ${mask.length} != wave_width ${this._config.waveWidth}`,
      );
    }
    this._execMask = [...mask];
  }

  // --- Execution ---

  step(clockEdge: { cycle: number }): EngineTrace {
    this._cycle += 1;

    if (this._allHalted) {
      return this._makeHaltedTrace();
    }

    const maskBefore = [...this._execMask];

    // Execute on active lanes only
    const laneTraces = new Array<string | undefined>(this._config.waveWidth);

    for (let laneId = 0; laneId < this._config.waveWidth; laneId++) {
      const laneCore = this._lanes[laneId];
      if (this._execMask[laneId] && !laneCore.halted) {
        try {
          const trace = laneCore.step();
          laneTraces[laneId] = trace.description;
          if (trace.halted) {
            laneTraces[laneId] = "HALTED";
          }
        } catch {
          laneTraces[laneId] = "(error)";
        }
      } else if (laneCore.halted) {
        laneTraces[laneId] = "(halted)";
      } else {
        // Lane is masked off -- still advance its PC to stay in sync
        if (!laneCore.halted) {
          try {
            laneCore.step();
            laneTraces[laneId] = "(masked -- result discarded)";
          } catch {
            laneTraces[laneId] = "(masked -- error)";
          }
        } else {
          laneTraces[laneId] = "(halted)";
        }
      }
    }

    // Sync VRF with internal core registers for active lanes
    for (let laneId = 0; laneId < this._config.waveWidth; laneId++) {
      if (this._execMask[laneId]) {
        const syncCount = Math.min(this._config.numVgprs, 32);
        for (let vreg = 0; vreg < syncCount; vreg++) {
          const val = this._lanes[laneId].registers.readFloat(vreg);
          this._vrf.write(vreg, laneId, val);
        }
      }
    }

    // Check if all lanes halted
    if (this._lanes.every((lane) => lane.halted)) {
      this._allHalted = true;
    }

    let activeCount = 0;
    for (let i = 0; i < this._config.waveWidth; i++) {
      if (this._execMask[i] && !this._lanes[i].halted) {
        activeCount++;
      }
    }
    const total = this._config.waveWidth;

    // Build description
    const skipStates = new Set([
      "(masked -- result discarded)",
      "(halted)",
      "(error)",
      "(masked -- error)",
      "HALTED",
    ]);
    let firstDesc = "no active lanes";
    for (let i = 0; i < this._config.waveWidth; i++) {
      const desc = laneTraces[i];
      if (desc !== undefined && !skipStates.has(desc)) {
        firstDesc = desc;
        break;
      }
    }

    const currentMask = Array.from(
      { length: this._config.waveWidth },
      (_, i) => this._execMask[i] && !this._lanes[i].halted,
    );

    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: `${firstDesc} -- ${activeCount}/${total} lanes active`,
      unitTraces: materializeUnitTraces(laneTraces),
      activeMask: currentMask,
      activeCount,
      totalCount: total,
      utilization: total > 0 ? activeCount / total : 0.0,
      divergenceInfo: {
        activeMaskBefore: maskBefore,
        activeMaskAfter: [...this._execMask],
        reconvergencePc: -1,
        divergenceDepth: 0,
      },
    };
  }

  run(maxCycles: number = 10000): EngineTrace[] {
    const traces: EngineTrace[] = [];
    for (let cycleNum = 1; cycleNum <= maxCycles; cycleNum++) {
      const trace = this.step({ cycle: cycleNum });
      traces.push(trace);
      if (this._allHalted) {
        return traces;
      }
    }
    if (!this._allHalted) {
      throw new Error(
        `WavefrontEngine: max_cycles (${maxCycles}) reached`,
      );
    }
    return traces;
  }

  private _makeHaltedTrace(): EngineTrace {
    return {
      cycle: this._cycle,
      engineName: this.name,
      executionModel: this.executionModel,
      description: "All lanes halted",
      unitTraces: materializeUnitTraces(
        Array.from(
          { length: this._config.waveWidth },
          () => "(halted)",
        ),
      ),
      activeMask: Array.from(
        { length: this._config.waveWidth },
        () => false,
      ),
      activeCount: 0,
      totalCount: this._config.waveWidth,
      utilization: 0.0,
    };
  }

  reset(): void {
    for (const lane of this._lanes) {
      lane.reset();
      if (this._program.length > 0) {
        lane.loadProgram(this._program);
      }
    }
    this._execMask = Array.from(
      { length: this._config.waveWidth },
      () => true,
    );
    this._allHalted = false;
    this._cycle = 0;
    this._vrf = new VectorRegisterFile(
      this._config.numVgprs,
      this._config.waveWidth,
      this._config.floatFormat,
    );
    this._srf = new ScalarRegisterFile(
      this._config.numSgprs,
      this._config.floatFormat,
    );
  }

  toString(): string {
    const active = this._execMask.filter(Boolean).length;
    return (
      `WavefrontEngine(width=${this._config.waveWidth}, ` +
      `active_lanes=${active}, halted=${this._allHalted})`
    );
  }
}
