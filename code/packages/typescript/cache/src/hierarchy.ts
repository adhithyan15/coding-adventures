/**
 * Cache hierarchy — multi-level cache system (L1I + L1D + L2 + L3 + memory).
 *
 * A modern CPU doesn't have just one cache — it has a **hierarchy** of
 * progressively larger and slower caches. This is the memory equivalent
 * of keeping frequently used items close to hand:
 *
 *     +---------+     +--------+     +--------+     +--------+     +--------+
 *     |   CPU   | --> |  L1    | --> |   L2   | --> |   L3   | --> |  Main  |
 *     |  core   |     | 1 cyc  |     | 10 cyc |     | 30 cyc |     | Memory |
 *     |         |     | 64KB   |     | 256KB  |     | 8MB    |     | 100cyc |
 *     +---------+     +--------+     +--------+     +--------+     +--------+
 *                      per-core       per-core       shared         shared
 *
 * Analogy:
 * - L1 = the books open on your desk (tiny, instant access)
 * - L2 = the bookshelf in your office (bigger, a few seconds to grab)
 * - L3 = the library downstairs (huge, takes a minute to walk there)
 * - Main memory = the warehouse across town (enormous, takes an hour)
 *
 * When the CPU reads an address:
 * 1. Check L1D. Hit? Return data (1 cycle). Miss? Continue.
 * 2. Check L2. Hit? Return data (10 cycles), and fill L1D. Miss? Continue.
 * 3. Check L3. Hit? Return data (30 cycles), fill L2 and L1D. Miss? Continue.
 * 4. Go to main memory (100 cycles). Fill L3, L2, and L1D.
 *
 * The total latency is the sum of all levels that missed:
 * - L1 hit:                1 cycle
 * - L1 miss, L2 hit:       1 + 10 = 11 cycles
 * - L1+L2 miss, L3 hit:    1 + 10 + 30 = 41 cycles
 * - All miss:               1 + 10 + 30 + 100 = 141 cycles
 *
 * Harvard vs Unified:
 * - **Harvard architecture**: Separate L1 for instructions (L1I) and data (L1D).
 *   This lets the CPU fetch an instruction and load data simultaneously.
 * - **Unified**: L2 and L3 are typically unified (shared between instructions
 *   and data) to avoid wasting space.
 */

import { Cache, CacheAccess } from "./cache.js";

// ── Hierarchy Access Record ─────────────────────────────────────────────

/**
 * Record of an access through the full hierarchy.
 *
 * Tracks which level served the data and the total latency accumulated
 * across all levels that were consulted.
 */
export interface HierarchyAccess {
  /** The memory address that was accessed. */
  address: number;

  /** Name of the level that had the data ("L1D", "L2", "L3", "memory"). */
  servedBy: string;

  /** Total clock cycles from start to data delivery. */
  totalCycles: number;

  /** Which hierarchy level served the data (0=L1, 1=L2, 2=L3, 3=memory). */
  hitAtLevel: number;

  /** Detailed access records from each cache level consulted. */
  levelAccesses: CacheAccess[];
}

// ── Cache Hierarchy ─────────────────────────────────────────────────────

/**
 * Multi-level cache hierarchy — L1I + L1D + L2 + L3 + main memory.
 *
 * Fully configurable: pass any combination of cache levels. You can
 * simulate anything from a simple L1-only system to a full 3-level
 * hierarchy with separate instruction and data L1 caches.
 *
 * @example
 * ```ts
 * import { Cache, CacheConfig, CacheHierarchy } from "@coding-adventures/cache";
 * const l1d = new Cache(new CacheConfig("L1D", 1024, 64, 4, 1));
 * const l2  = new Cache(new CacheConfig("L2", 4096, 64, 8, 10));
 * const hierarchy = new CacheHierarchy({ l1d, l2 });
 * const result = hierarchy.read(0x1000, false, 0);
 * result.servedBy;  // "memory" (first access is always a miss)
 * ```
 */
export class CacheHierarchy {
  readonly l1i: Cache | null;
  readonly l1d: Cache | null;
  readonly l2: Cache | null;
  readonly l3: Cache | null;
  readonly mainMemoryLatency: number;

  /**
   * Build ordered lists of (name, cache) for iteration.
   * The hierarchy is walked top-down (fastest to slowest).
   */
  private readonly _dataLevels: [string, Cache][];
  private readonly _instrLevels: [string, Cache][];

  /**
   * Create a cache hierarchy.
   *
   * @param options.l1i               - L1 instruction cache (optional, for Harvard architecture).
   * @param options.l1d               - L1 data cache (optional but typical).
   * @param options.l2                - L2 cache (optional).
   * @param options.l3                - L3 cache (optional).
   * @param options.mainMemoryLatency - Clock cycles for main memory access.
   */
  constructor(options: {
    l1i?: Cache | null;
    l1d?: Cache | null;
    l2?: Cache | null;
    l3?: Cache | null;
    mainMemoryLatency?: number;
  } = {}) {
    this.l1i = options.l1i ?? null;
    this.l1d = options.l1d ?? null;
    this.l2 = options.l2 ?? null;
    this.l3 = options.l3 ?? null;
    this.mainMemoryLatency = options.mainMemoryLatency ?? 100;

    // Build ordered list of (name, cache) for iteration
    this._dataLevels = [];
    if (this.l1d !== null) this._dataLevels.push(["L1D", this.l1d]);
    if (this.l2 !== null) this._dataLevels.push(["L2", this.l2]);
    if (this.l3 !== null) this._dataLevels.push(["L3", this.l3]);

    this._instrLevels = [];
    if (this.l1i !== null) this._instrLevels.push(["L1I", this.l1i]);
    if (this.l2 !== null) this._instrLevels.push(["L2", this.l2]);
    if (this.l3 !== null) this._instrLevels.push(["L3", this.l3]);
  }

  // ── Read ────────────────────────────────────────────────────────────

  /**
   * Read through the hierarchy. Returns which level served the data.
   *
   * Walks the hierarchy top-down. At each level:
   * - If hit: stop, fill all higher levels, return.
   * - If miss: accumulate latency, continue to next level.
   * - If all miss: data comes from main memory.
   *
   * The **inclusive** fill policy is used: when L3 serves data, it
   * also fills L2 and L1D so subsequent accesses hit at L1.
   *
   * @param address       - Memory address to read.
   * @param isInstruction - If true, use L1I instead of L1D for the
   *                        first level. L2 and L3 are unified.
   * @param cycle         - Current clock cycle.
   * @returns HierarchyAccess with the level that served and total cycles.
   */
  read(address: number, isInstruction = false, cycle = 0): HierarchyAccess {
    const levels = isInstruction ? this._instrLevels : this._dataLevels;

    if (levels.length === 0) {
      // No caches at all — go straight to memory
      return {
        address,
        servedBy: "memory",
        totalCycles: this.mainMemoryLatency,
        hitAtLevel: levels.length,
        levelAccesses: [],
      };
    }

    let totalCycles = 0;
    const accesses: CacheAccess[] = [];
    let servedBy = "memory";
    let hitLevel = levels.length;

    // Walk the hierarchy top-down
    for (let levelIdx = 0; levelIdx < levels.length; levelIdx++) {
      const [name, cache] = levels[levelIdx];
      const access = cache.read(address, 1, cycle);
      totalCycles += cache.config.accessLatency;
      accesses.push(access);

      if (access.hit) {
        servedBy = name;
        hitLevel = levelIdx;
        break;
      }
    }

    if (servedBy === "memory") {
      // Complete miss — add main memory latency
      totalCycles += this.mainMemoryLatency;
    }

    // Fill higher levels (inclusive policy).
    // If L3 served, fill L2 and L1. If L2 served, fill L1.
    // We fill with dummy data (zeros) since we're simulating.
    const dummyData = new Array<number>(this._getLineSize(levels)).fill(0);
    for (let fillIdx = hitLevel - 1; fillIdx >= 0; fillIdx--) {
      const [, fillCache] = levels[fillIdx];
      fillCache.fillLine(address, dummyData, cycle);
    }

    return {
      address,
      servedBy,
      totalCycles,
      hitAtLevel: hitLevel,
      levelAccesses: accesses,
    };
  }

  // ── Write ───────────────────────────────────────────────────────────

  /**
   * Write through the hierarchy.
   *
   * With write-allocate + write-back (the most common policy):
   * 1. If L1D hit: write to L1D, mark dirty. Done.
   * 2. If L1D miss: allocate in L1D (may cause eviction cascade),
   *    write to L1D. The data comes from the next level that has it
   *    or from main memory.
   *
   * The write is always done at the L1D level. On a miss, we walk
   * down to find the data (or go to memory), fill back up, then write.
   *
   * @param address - Memory address to write.
   * @param data    - Bytes to write.
   * @param cycle   - Current clock cycle.
   * @returns HierarchyAccess with the level that served and total cycles.
   */
  write(
    address: number,
    data: number[] | null = null,
    cycle = 0,
  ): HierarchyAccess {
    const levels = this._dataLevels;

    if (levels.length === 0) {
      return {
        address,
        servedBy: "memory",
        totalCycles: this.mainMemoryLatency,
        hitAtLevel: 0,
        levelAccesses: [],
      };
    }

    // Check L1D first (writes always go to the data cache)
    const [firstName, firstCache] = levels[0];
    const access = firstCache.write(address, data, cycle);

    if (access.hit) {
      return {
        address,
        servedBy: firstName,
        totalCycles: firstCache.config.accessLatency,
        hitAtLevel: 0,
        levelAccesses: [access],
      };
    }

    // Write miss at L1 — walk lower levels to find the data
    let totalCycles = firstCache.config.accessLatency;
    const accesses: CacheAccess[] = [access];
    let servedBy = "memory";
    let hitLevel = levels.length;

    for (let levelIdx = 1; levelIdx < levels.length; levelIdx++) {
      const [name, cache] = levels[levelIdx];
      const levelAccess = cache.read(address, 1, cycle);
      totalCycles += cache.config.accessLatency;
      accesses.push(levelAccess);

      if (levelAccess.hit) {
        servedBy = name;
        hitLevel = levelIdx;
        break;
      }
    }

    if (servedBy === "memory") {
      totalCycles += this.mainMemoryLatency;
    }

    return {
      address,
      servedBy,
      totalCycles,
      hitAtLevel: hitLevel,
      levelAccesses: accesses,
    };
  }

  // ── Helpers ─────────────────────────────────────────────────────────

  /** Get the line size from the first level in the hierarchy. */
  private _getLineSize(levels: [string, Cache][]): number {
    if (levels.length > 0) {
      return levels[0][1].config.lineSize;
    }
    return 64; // default
  }

  /** Invalidate all caches in the hierarchy (full flush). */
  invalidateAll(): void {
    for (const cache of [this.l1i, this.l1d, this.l2, this.l3]) {
      if (cache !== null) {
        cache.invalidate();
      }
    }
  }

  /** Reset statistics for all cache levels. */
  resetStats(): void {
    for (const cache of [this.l1i, this.l1d, this.l2, this.l3]) {
      if (cache !== null) {
        cache.stats.reset();
      }
    }
  }

  /** Human-readable summary of the hierarchy. */
  toString(): string {
    const parts: string[] = [];
    if (this.l1i !== null)
      parts.push(`L1I=${Math.floor(this.l1i.config.totalSize / 1024)}KB`);
    if (this.l1d !== null)
      parts.push(`L1D=${Math.floor(this.l1d.config.totalSize / 1024)}KB`);
    if (this.l2 !== null)
      parts.push(`L2=${Math.floor(this.l2.config.totalSize / 1024)}KB`);
    if (this.l3 !== null)
      parts.push(`L3=${Math.floor(this.l3.config.totalSize / 1024)}KB`);
    parts.push(`mem=${this.mainMemoryLatency}cyc`);
    return `CacheHierarchy(${parts.join(", ")})`;
  }
}
