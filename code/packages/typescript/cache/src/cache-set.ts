/**
 * Cache set — a group of cache lines that share the same set index.
 *
 * A cache set is like a row of labeled boxes on a shelf. When the CPU
 * accesses memory, the address tells us *which shelf* (set) to look at.
 * Within that shelf, we check each box (way) to see if our data is there.
 *
 * In a **4-way set-associative** cache, each set has 4 lines (ways).
 * When all 4 are full and we need to bring in new data, we must **evict**
 * one. The LRU (Least Recently Used) policy picks the line that hasn't
 * been accessed for the longest time — the logic being "if you haven't
 * used it lately, you probably won't need it soon."
 *
 * Associativity is a key design tradeoff:
 * - **Direct-mapped** (1-way): Fast lookup, but high conflict misses.
 *   Like a parking lot where each car is assigned exactly one spot — if
 *   two cars map to the same spot, one must leave even if other spots
 *   are empty.
 * - **Fully associative** (N-way = total lines): No conflicts, but
 *   expensive to search every line on every access.
 * - **Set-associative** (2/4/8/16-way): The sweet spot. Each address
 *   maps to a set, and within that set, any way can hold it.
 *
 *     Set 0: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
 *     Set 1: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
 *     Set 2: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
 *     ...
 *
 */

import { CacheLine } from "./cache-line.js";

// ── Configuration ─────────────────────────────────────────────────────

/** Valid write policies for a cache level. */
export type WritePolicy = "write-back" | "write-through";

/**
 * Configuration for a cache level — the knobs you turn to get L1/L2/L3.
 *
 * By adjusting these parameters, the exact same Cache class can simulate
 * anything from a tiny 1KB direct-mapped L1 to a massive 32MB 16-way L3.
 *
 * Real-world examples:
 *     ARM Cortex-A78: L1D = 64KB, 4-way, 64B lines, 1 cycle
 *     Intel Alder Lake: L1D = 48KB, 12-way, 64B lines, 5 cycles
 *     Apple M4: L1D = 128KB, 8-way, 64B lines, ~3 cycles
 */
export class CacheConfig {
  readonly name: string;
  readonly totalSize: number;
  readonly lineSize: number;
  readonly associativity: number;
  readonly accessLatency: number;
  readonly writePolicy: WritePolicy;

  /**
   * Create and validate a cache configuration.
   *
   * @param name           - Human-readable name for this cache level ("L1D", "L2", etc.)
   * @param totalSize      - Total capacity in bytes (e.g., 65536 for 64KB).
   * @param lineSize       - Bytes per cache line. Must be a power of 2.
   * @param associativity  - Number of ways per set. 1 = direct-mapped.
   * @param accessLatency  - Clock cycles to access this level on a hit.
   * @param writePolicy    - "write-back" (defer writes) or "write-through" (immediate).
   */
  constructor(
    name: string,
    totalSize: number,
    lineSize = 64,
    associativity = 4,
    accessLatency = 1,
    writePolicy: WritePolicy = "write-back",
  ) {
    // ── Validate configuration parameters ─────────────────────────
    // Cache sizes and line sizes must be powers of 2 — this is a
    // hardware constraint because address bit-slicing only works
    // cleanly with power-of-2 sizes.

    if (totalSize <= 0) {
      throw new Error(`total_size must be positive, got ${totalSize}`);
    }
    if (lineSize <= 0 || (lineSize & (lineSize - 1)) !== 0) {
      throw new Error(
        `line_size must be a positive power of 2, got ${lineSize}`,
      );
    }
    if (associativity <= 0) {
      throw new Error(
        `associativity must be positive, got ${associativity}`,
      );
    }
    if (totalSize % (lineSize * associativity) !== 0) {
      throw new Error(
        `total_size (${totalSize}) must be divisible by ` +
          `line_size * associativity (${lineSize * associativity})`,
      );
    }
    if (writePolicy !== "write-back" && writePolicy !== "write-through") {
      throw new Error(
        `write_policy must be 'write-back' or 'write-through', got '${writePolicy}'`,
      );
    }
    if (accessLatency < 0) {
      throw new Error(
        `access_latency must be non-negative, got ${accessLatency}`,
      );
    }

    this.name = name;
    this.totalSize = totalSize;
    this.lineSize = lineSize;
    this.associativity = associativity;
    this.accessLatency = accessLatency;
    this.writePolicy = writePolicy;
  }

  /** Total number of cache lines = totalSize / lineSize. */
  get numLines(): number {
    return this.totalSize / this.lineSize;
  }

  /** Number of sets = numLines / associativity. */
  get numSets(): number {
    return this.numLines / this.associativity;
  }
}

// ── Cache Set ─────────────────────────────────────────────────────────

/**
 * One set in the cache — contains N ways (lines).
 *
 * Implements LRU (Least Recently Used) replacement: when all ways are
 * full and we need to bring in new data, evict the line that was
 * accessed least recently.
 *
 * Think of it like a desk with N book slots. When all slots are full
 * and you need a new book, you put away the one you haven't read in
 * the longest time.
 */
export class CacheSet {
  readonly lines: CacheLine[];

  /**
   * Create a cache set with the given number of ways.
   *
   * @param associativity - Number of ways (lines) in this set.
   * @param lineSize      - Bytes per cache line.
   */
  constructor(associativity: number, lineSize: number) {
    this.lines = Array.from(
      { length: associativity },
      () => new CacheLine(lineSize),
    );
  }

  // ── Lookup ──────────────────────────────────────────────────────────

  /**
   * Check if a tag is present in this set.
   *
   * Searches all ways for a valid line with a matching tag. This is
   * what happens in hardware with a parallel tag comparator — all
   * ways are checked simultaneously.
   *
   * @param tag - The tag bits from the address.
   * @returns `[hit, wayIndex]` — hit is true if found; wayIndex is the
   *          index of the matching line (or null if miss).
   */
  lookup(tag: number): [boolean, number | null] {
    for (let i = 0; i < this.lines.length; i++) {
      const line = this.lines[i];
      if (line.valid && line.tag === tag) {
        return [true, i];
      }
    }
    return [false, null];
  }

  // ── Access ──────────────────────────────────────────────────────────

  /**
   * Access this set for a given tag. Returns [hit, line].
   *
   * On a hit, updates the line's LRU timestamp so it becomes the
   * most recently used. On a miss, returns the LRU victim line
   * (the caller decides what to do — typically allocate new data).
   *
   * @param tag   - The tag bits from the address.
   * @param cycle - Current clock cycle for LRU tracking.
   * @returns `[hit, line]` — if hit, the matching line. If miss, the LRU
   *          victim line (which may need writeback if dirty).
   */
  access(tag: number, cycle: number): [boolean, CacheLine] {
    const [hit, wayIndex] = this.lookup(tag);
    if (hit) {
      const line = this.lines[wayIndex!];
      line.touch(cycle);
      return [true, line];
    }
    // Miss — return the LRU line (candidate for eviction)
    const lruIndex = this.findLru();
    return [false, this.lines[lruIndex]];
  }

  // ── Allocation (filling after a miss) ───────────────────────────────

  /**
   * Bring new data into this set after a cache miss.
   *
   * First tries to find an invalid (empty) way. If all ways are
   * valid, evicts the LRU line. Returns the evicted line if it was
   * dirty (the caller must write it back to the next level).
   *
   * @param tag   - Tag for the new data.
   * @param data  - The bytes to store.
   * @param cycle - Current clock cycle.
   * @returns The evicted CacheLine if it was dirty (needs writeback),
   *          or null if no dirty writeback is needed.
   *
   * Think of it like clearing a desk slot for a new book:
   * 1. If there's an empty slot, use it (no eviction needed).
   * 2. If all slots are full, pick the least-recently-read book.
   * 3. If that book had notes scribbled in it (dirty), you need
   *    to save those notes before putting the book away.
   */
  allocate(tag: number, data: number[], cycle: number): CacheLine | null {
    // Step 1: Look for an invalid (empty) way
    for (const line of this.lines) {
      if (!line.valid) {
        line.fill(tag, data, cycle);
        return null; // no eviction needed
      }
    }

    // Step 2: All ways full — evict the LRU line
    const lruIndex = this.findLru();
    const victim = this.lines[lruIndex];

    // Step 3: Check if the victim is dirty (needs writeback)
    let evicted: CacheLine | null = null;
    if (victim.dirty) {
      // Create a copy of the evicted line for writeback
      evicted = new CacheLine(victim.data.length);
      evicted.valid = true;
      evicted.dirty = true;
      evicted.tag = victim.tag;
      evicted.data = [...victim.data];
      evicted.lastAccess = victim.lastAccess;
    }

    // Step 4: Overwrite the victim with new data
    victim.fill(tag, data, cycle);

    return evicted;
  }

  // ── LRU Selection ───────────────────────────────────────────────────

  /**
   * Find the least recently used way index.
   *
   * LRU replacement is simple: each line records its last access
   * time (cycle count). The line with the smallest timestamp is
   * the one that hasn't been touched for the longest time.
   *
   * In real hardware, true LRU is expensive for high associativity
   * (tracking N! orderings). CPUs often use pseudo-LRU (tree-PLRU)
   * or RRIP as approximations. For simulation, true LRU is fine.
   *
   * Special case: invalid lines are always preferred over valid ones
   * (an empty slot is "older" than any real data).
   *
   * @returns Index of the LRU way in this.lines.
   */
  findLru(): number {
    let bestIndex = 0;
    let bestTime = Infinity;
    for (let i = 0; i < this.lines.length; i++) {
      const line = this.lines[i];
      // Invalid lines are always the best candidates
      if (!line.valid) {
        return i;
      }
      if (line.lastAccess < bestTime) {
        bestTime = line.lastAccess;
        bestIndex = i;
      }
    }
    return bestIndex;
  }
}
