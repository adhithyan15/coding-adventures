/**
 * Work Distributor -- assigns work to compute units.
 *
 * === Three Distribution Strategies ===
 *
 * Different accelerator architectures distribute work in fundamentally
 * different ways. This module implements all three:
 *
 * 1. **GPU Block Distributor** (NVIDIA, AMD, Intel)
 *    - Takes a kernel launch with grid/block dimensions
 *    - Decomposes into thread blocks
 *    - Assigns blocks to compute units that have free resources
 *    - Continues assigning as CUs complete blocks (multi-wave)
 *
 * 2. **TPU Sequencer** (Google TPU)
 *    - Takes HLO operations (matmul, add, relu, etc.)
 *    - Tiles large operations to fit the MXU
 *    - Pipelines through Scalar -> MXU -> Vector units
 *    - One operation at a time (no thread blocks)
 *
 * 3. **ANE Schedule Replayer** (Apple Neural Engine)
 *    - Compiler generates a complete execution schedule at compile time
 *    - The "distributor" simply replays the schedule
 *    - No dynamic scheduling decisions -- everything is predetermined
 *    - DMA loads data to SRAM, cores process it, DMA stores results
 */

import {
  type ComputeUnit,
  ResourceError,
  makeWorkItem,
} from "@coding-adventures/compute-unit";

import {
  type KernelDescriptor,
  totalBlocks,
  threadsPerBlock,
} from "./protocols.js";

// =========================================================================
// GPU Block Distributor
// =========================================================================

/**
 * Distributes thread blocks to compute units.
 *
 * Used by NVIDIA (GigaThread Engine), AMD (Command Processor),
 * and Intel (Command Streamer). The same algorithm works for all
 * three -- they differ only in CU-level resource limits.
 *
 * === Distribution Policies ===
 *
 *     round_robin:  Cycle through CUs evenly. Fair, simple.
 *     fill_first:   Fill one CU before moving to next. Max occupancy per CU.
 *     least_loaded: Assign to CU with fewest active warps. Best balance.
 */
export class GPUWorkDistributor {
  private readonly _cus: ComputeUnit[];
  private readonly _policy: string;
  private _pending: Array<ReturnType<typeof makeWorkItem>>;
  private _rrIndex: number;
  private _totalDispatched: number;

  constructor(computeUnits: ComputeUnit[], policy: string = "round_robin") {
    this._cus = computeUnits;
    this._policy = policy;
    this._pending = [];
    this._rrIndex = 0;
    this._totalDispatched = 0;
  }

  /** Number of blocks waiting to be assigned. */
  get pendingCount(): number {
    return this._pending.length;
  }

  /** Total blocks dispatched so far. */
  get totalDispatched(): number {
    return this._totalDispatched;
  }

  /**
   * Decompose a kernel into thread blocks and queue them.
   *
   * Each thread block becomes a WorkItem. The block's position in
   * the grid is encoded in the workId (linear index).
   *
   * A 3D grid (gx, gy, gz) is linearized:
   *     blockId = bz * gx * gy + by * gx + bx
   */
  submitKernel(kernel: KernelDescriptor): void {
    const numBlocks = totalBlocks(kernel);
    const tpb = threadsPerBlock(kernel);

    for (let blockId = 0; blockId < numBlocks; blockId++) {
      const work = makeWorkItem({
        workId: blockId,
        program: kernel.program ? [...kernel.program] : null,
        threadCount: tpb,
        registersPerThread: kernel.registersPerThread,
        sharedMemBytes: kernel.sharedMemBytes,
      });
      this._pending.push(work);
    }
  }

  /**
   * Try to assign pending blocks to available CUs.
   *
   * Returns a list of human-readable assignment descriptions.
   * Each entry looks like: "Block 42 -> SM 7"
   *
   * For each CU (in policy order):
   *     While there are pending blocks:
   *         Try to dispatch the next block to this CU
   *         If CU rejects it (ResourceError), move to next CU
   *         If CU accepts it, log the assignment
   */
  step(): string[] {
    if (this._pending.length === 0) {
      return [];
    }

    const assignments: string[] = [];
    const order = this._cuOrder();

    for (const cu of order) {
      while (this._pending.length > 0) {
        const block = this._pending[0];
        try {
          cu.dispatch(block);
          this._pending.shift();
          this._totalDispatched += 1;
          assignments.push(`Block ${block.workId} -> ${cu.name}`);
        } catch (e) {
          // CU can't accept this block (full) -- try next CU
          break;
        }
      }
    }

    return assignments;
  }

  /**
   * Return CUs in the order dictated by the policy.
   *
   * round_robin:  Start from rrIndex, wrap around.
   * fill_first:   Just return in order (fill CU 0 first, then CU 1, ...).
   * least_loaded: Sort by idle status (idle CUs first).
   */
  private _cuOrder(): ComputeUnit[] {
    const n = this._cus.length;
    if (n === 0) return [];

    if (this._policy === "fill_first") {
      return [...this._cus];
    }

    if (this._policy === "least_loaded") {
      return [...this._cus].sort((a, b) => {
        const aVal = a.idle ? 0 : 1;
        const bVal = b.idle ? 0 : 1;
        return aVal - bVal;
      });
    }

    // Default: round_robin
    const ordered: ComputeUnit[] = [];
    for (let i = 0; i < n; i++) {
      const idx = (this._rrIndex + i) % n;
      ordered.push(this._cus[idx]);
    }
    this._rrIndex = (this._rrIndex + 1) % n;
    return ordered;
  }

  /** Clear all pending work and reset counters. */
  reset(): void {
    this._pending = [];
    this._rrIndex = 0;
    this._totalDispatched = 0;
  }
}

// =========================================================================
// TPU Sequencer
// =========================================================================

/**
 * A single tile operation in the TPU pipeline.
 */
export interface TileOperation {
  tileId: number;
  operation: string;
  inputData: readonly (readonly number[])[] | null;
  weightData: readonly (readonly number[])[] | null;
  status: string; // "pending" | "scalar" | "mxu" | "vector" | "done"
  cyclesRemaining: number;
}

/**
 * Orchestrates operations through Scalar + Vector + MXU units.
 *
 * === TPU Execution Pipeline ===
 *
 * The TPU processes operations through a three-stage pipeline:
 *
 *     Scalar Unit -> MXU -> Vector Unit
 *
 * Stage 1 (Scalar): Prepare addresses, loop counters, control flow.
 * Stage 2 (MXU):    The heavy lifting -- matrix multiply on the systolic array.
 * Stage 3 (Vector): Post-processing -- activation functions, normalization.
 *
 * These three stages overlap: while the MXU crunches tile N, the Vector
 * unit processes tile N-1, and the Scalar unit prepares tile N+1.
 *
 *     Time ->
 *     Scalar: [tile 0] [tile 1] [tile 2] [tile 3] ...
 *     MXU:           [tile 0] [tile 1] [tile 2] ...
 *     Vector:               [tile 0] [tile 1] ...
 */
export class TPUSequencer {
  private readonly _mxu: ComputeUnit;
  private readonly _mxuSize: number;
  private readonly _vectorWidth: number;
  private readonly _scalarLatency: number;
  private readonly _mxuLatency: number;
  private readonly _vectorLatency: number;

  private _pending: TileOperation[];
  private _scalarTile: TileOperation | null;
  private _mxuTile: TileOperation | null;
  private _vectorTile: TileOperation | null;
  private _completed: TileOperation[];
  private _totalDispatched: number;

  constructor(
    mxu: ComputeUnit,
    opts: {
      mxuSize?: number;
      vectorWidth?: number;
      scalarLatency?: number;
      mxuLatency?: number;
      vectorLatency?: number;
    } = {},
  ) {
    this._mxu = mxu;
    this._mxuSize = opts.mxuSize ?? 128;
    this._vectorWidth = opts.vectorWidth ?? 128;
    this._scalarLatency = opts.scalarLatency ?? 5;
    this._mxuLatency = opts.mxuLatency ?? 20;
    this._vectorLatency = opts.vectorLatency ?? 10;

    this._pending = [];
    this._scalarTile = null;
    this._mxuTile = null;
    this._vectorTile = null;
    this._completed = [];
    this._totalDispatched = 0;
  }

  /** Number of tiles waiting to be processed. */
  get pendingCount(): number {
    return this._pending.length;
  }

  /** Total tiles dispatched so far. */
  get totalDispatched(): number {
    return this._totalDispatched;
  }

  /**
   * Tile a large operation and queue the tiles.
   *
   * If the input matrix is 256x256 but the MXU is 128x128, we need
   * to split it into 4 tiles:
   *
   *     Tile 0: rows 0-127,   cols 0-127
   *     Tile 1: rows 0-127,   cols 128-255
   *     Tile 2: rows 128-255, cols 0-127
   *     Tile 3: rows 128-255, cols 128-255
   */
  submitOperation(kernel: KernelDescriptor): void {
    const inputData = kernel.inputData ?? [[0.0]];
    const weightData = kernel.weightData ?? [[0.0]];

    const rows = inputData.length;
    const cols = weightData[0]?.length ?? 1;
    const mxu = this._mxuSize;

    const numRowTiles = Math.max(1, Math.ceil(rows / mxu));
    const numColTiles = Math.max(1, Math.ceil(cols / mxu));

    let tileId = 0;
    for (let _rt = 0; _rt < numRowTiles; _rt++) {
      for (let _ct = 0; _ct < numColTiles; _ct++) {
        const tile: TileOperation = {
          tileId,
          operation: kernel.operation || "matmul",
          inputData,
          weightData,
          status: "pending",
          cyclesRemaining: this._scalarLatency,
        };
        this._pending.push(tile);
        tileId++;
      }
    }
  }

  /**
   * Advance the pipeline by one cycle.
   *
   * Returns descriptions of what happened this cycle.
   */
  step(): string[] {
    const actions: string[] = [];

    // Vector stage: finish processing
    if (this._vectorTile !== null) {
      this._vectorTile.cyclesRemaining -= 1;
      if (this._vectorTile.cyclesRemaining <= 0) {
        this._vectorTile.status = "done";
        this._completed.push(this._vectorTile);
        actions.push(`Vector: completed tile ${this._vectorTile.tileId}`);
        this._vectorTile = null;
      }
    }

    // MXU stage: process matrix multiply
    if (this._mxuTile !== null) {
      this._mxuTile.cyclesRemaining -= 1;
      if (this._mxuTile.cyclesRemaining <= 0) {
        this._mxuTile.status = "vector";
        this._mxuTile.cyclesRemaining = this._vectorLatency;
        // Move to vector stage (if free)
        if (this._vectorTile === null) {
          this._vectorTile = this._mxuTile;
          this._mxuTile = null;
          actions.push(
            `MXU -> Vector: tile ${this._vectorTile.tileId}`,
          );
        }
      }
    }

    // Scalar stage: prepare next tile
    if (this._scalarTile !== null) {
      this._scalarTile.cyclesRemaining -= 1;
      if (this._scalarTile.cyclesRemaining <= 0) {
        this._scalarTile.status = "mxu";
        this._scalarTile.cyclesRemaining = this._mxuLatency;
        // Move to MXU stage (if free)
        if (this._mxuTile === null) {
          this._mxuTile = this._scalarTile;
          this._scalarTile = null;
          this._totalDispatched += 1;
          actions.push(
            `Scalar -> MXU: tile ${this._mxuTile.tileId}`,
          );
        }
      }
    }

    // Feed from pending queue to scalar stage
    if (this._scalarTile === null && this._pending.length > 0) {
      this._scalarTile = this._pending.shift()!;
      this._scalarTile.status = "scalar";
      this._scalarTile.cyclesRemaining = this._scalarLatency;
      actions.push(`Scalar: started tile ${this._scalarTile.tileId}`);
    }

    return actions;
  }

  /** True when all tiles are processed. */
  get idle(): boolean {
    return (
      this._pending.length === 0 &&
      this._scalarTile === null &&
      this._mxuTile === null &&
      this._vectorTile === null
    );
  }

  /** Clear all state. */
  reset(): void {
    this._pending = [];
    this._scalarTile = null;
    this._mxuTile = null;
    this._vectorTile = null;
    this._completed = [];
    this._totalDispatched = 0;
  }
}

// =========================================================================
// ANE Schedule Replayer
// =========================================================================

/**
 * One step in a compiler-generated ANE schedule.
 *
 * The CoreML compiler pre-determines everything:
 * - Which core processes which tile
 * - When DMA loads happen
 * - When DMA stores happen
 * - The exact order of operations
 */
export interface ScheduleEntry {
  cycle: number;
  action: string; // "dma_load" | "compute" | "dma_store" | "activate"
  coreId: number;
  description: string;
  data: readonly (readonly number[])[] | null;
  weights: readonly (readonly number[])[] | null;
}

/**
 * Replays a compiler-generated execution schedule.
 *
 * === Why No Dynamic Scheduling? ===
 *
 * Unlike GPUs (which have hardware schedulers that decide at runtime
 * which warp to execute), the Apple Neural Engine relies entirely on
 * the compiler. The CoreML compiler analyzes the neural network graph,
 * determines the optimal tiling strategy, generates DMA transfer
 * schedules, and produces a fixed execution plan.
 *
 * This makes the hardware simpler (no complex scheduler) and more
 * power-efficient (no scheduling overhead), but less flexible --
 * the ANE can only run workloads the compiler knows how to schedule.
 */
export class ANEScheduleReplayer {
  private readonly _cus: ComputeUnit[];
  private readonly _dmaLatency: number;
  private readonly _computeLatency: number;
  private readonly _activateLatency: number;

  private _schedule: ScheduleEntry[];
  private _currentStep: number;
  private _totalDispatched: number;

  constructor(
    computeUnits: ComputeUnit[],
    opts: {
      dmaLatency?: number;
      computeLatency?: number;
      activateLatency?: number;
    } = {},
  ) {
    this._cus = computeUnits;
    this._dmaLatency = opts.dmaLatency ?? 10;
    this._computeLatency = opts.computeLatency ?? 20;
    this._activateLatency = opts.activateLatency ?? 5;

    this._schedule = [];
    this._currentStep = 0;
    this._totalDispatched = 0;
  }

  /** Number of schedule steps remaining. */
  get pendingCount(): number {
    return Math.max(0, this._schedule.length - this._currentStep);
  }

  /** Total operations dispatched so far. */
  get totalDispatched(): number {
    return this._totalDispatched;
  }

  /**
   * Generate a schedule from a kernel descriptor.
   *
   * The compiler (us, acting as the compiler) determines:
   * 1. How to tile the input across available cores
   * 2. When to load data via DMA
   * 3. When each core computes
   * 4. When to apply activation functions
   * 5. When to store results via DMA
   */
  submitOperation(kernel: KernelDescriptor): void {
    const inputData = kernel.inputData ?? [[0.0]];
    const weightData = kernel.weightData ?? [[0.0]];

    const numCores = this._cus.length;
    const rows = inputData.length;

    let cycle = 0;
    for (let coreId = 0; coreId < Math.min(numCores, rows); coreId++) {
      // DMA load input
      this._schedule.push({
        cycle,
        action: "dma_load",
        coreId,
        description: `DMA load input tile -> Core ${coreId}`,
        data: inputData,
        weights: null,
      });
      cycle += this._dmaLatency;

      // DMA load weights
      this._schedule.push({
        cycle,
        action: "dma_load",
        coreId,
        description: `DMA load weights -> Core ${coreId}`,
        data: null,
        weights: weightData,
      });
      cycle += this._dmaLatency;

      // Compute
      this._schedule.push({
        cycle,
        action: "compute",
        coreId,
        description: `Core ${coreId}: MAC array compute`,
        data: null,
        weights: null,
      });
      cycle += this._computeLatency;

      // Activate
      this._schedule.push({
        cycle,
        action: "activate",
        coreId,
        description: `Core ${coreId}: activation (ReLU)`,
        data: null,
        weights: null,
      });
      cycle += this._activateLatency;

      // DMA store
      this._schedule.push({
        cycle,
        action: "dma_store",
        coreId,
        description: `DMA store result from Core ${coreId}`,
        data: null,
        weights: null,
      });
      cycle += this._dmaLatency;
    }
  }

  /**
   * Execute the next step in the pre-computed schedule.
   *
   * Returns descriptions of what happened this cycle.
   */
  step(): string[] {
    if (this._currentStep >= this._schedule.length) {
      return [];
    }

    const entry = this._schedule[this._currentStep];
    this._currentStep += 1;
    this._totalDispatched += 1;

    return [entry.description];
  }

  /** True when the entire schedule has been replayed. */
  get idle(): boolean {
    return this._currentStep >= this._schedule.length;
  }

  /** Clear the schedule and reset. */
  reset(): void {
    this._schedule = [];
    this._currentStep = 0;
    this._totalDispatched = 0;
  }
}
