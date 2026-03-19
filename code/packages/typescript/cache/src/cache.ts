/**
 * Cache — a single configurable level of the cache hierarchy.
 *
 * This module implements the core cache logic. The same class is used for
 * L1, L2, and L3 — the only difference is the configuration (size,
 * associativity, latency). This reflects real hardware: an L1 and an L3
 * use the same SRAM cell design, just at different scales.
 *
 * ## Address Decomposition
 *
 * When the CPU accesses memory address 0x1A2B3C4D, the cache must figure
 * out three things:
 *
 * 1. **Offset** (lowest bits): Which byte *within* the cache line?
 *    - For 64-byte lines: 6 bits (2^6 = 64)
 *    - Example: offset = 0x0D = byte 13 of the line
 *
 * 2. **Set Index** (middle bits): Which set should we look in?
 *    - For 256 sets: 8 bits (2^8 = 256)
 *    - Example: set_index = 0xF1 = set 241
 *
 * 3. **Tag** (highest bits): Which memory block is this?
 *    - All remaining bits above offset + set_index
 *    - Example: tag = 0x1A2B3 (uniquely identifies the block)
 *
 * Visual for a 64KB, 4-way, 64B-line cache (256 sets):
 *
 *     Address: | tag (18 bits) | set index (8 bits) | offset (6 bits) |
 *              |  31 ... 14    |     13 ... 6       |    5 ... 0      |
 *
 * This bit-slicing is why cache sizes must be powers of 2 — it lets the
 * hardware extract fields with simple bit masks instead of division.
 *
 * ## Read Path
 *
 *     CPU reads address 0x1000
 *          |
 *          v
 *     Decompose: tag=0x4, set=0, offset=0
 *          |
 *          v
 *     Look in Set 0: compare tag 0x4 against all ways
 *          |
 *     +----+----+
 *     |         |
 *     HIT      MISS
 *     |         |
 *     Return   Go to next level (L2/L3/memory)
 *     data     Bring data back, allocate in this cache
 *              Maybe evict an old line (LRU)
 */

import { CacheLine } from "./cache-line.js";
import { CacheConfig, CacheSet } from "./cache-set.js";
import { CacheStats } from "./stats.js";

// ── Access Record ─────────────────────────────────────────────────────

/**
 * Record of a single cache access — for debugging and performance analysis.
 *
 * Every read() or write() call returns one of these, telling you exactly
 * what happened: was it a hit? Which set? Was anything evicted? How many
 * cycles did it cost?
 *
 * This is like a receipt for each memory transaction.
 */
export interface CacheAccess {
  /** The full memory address that was accessed. */
  address: number;

  /** True if the data was found in the cache (no need to go further). */
  hit: boolean;

  /** The tag bits extracted from the address. */
  tag: number;

  /** The set index bits — which set in the cache was consulted. */
  setIndex: number;

  /** The offset bits — byte position within the cache line. */
  offset: number;

  /** Clock cycles this access took (latency). */
  cycles: number;

  /**
   * If a line was evicted during this access, it's stored here.
   * Only set for dirty evictions that need writeback.
   */
  evicted: CacheLine | null;
}

// ── Cache ─────────────────────────────────────────────────────────────

/**
 * A single level of cache — configurable to be L1, L2, or L3.
 *
 * This is the workhorse of the cache simulator. Give it a CacheConfig
 * and it handles address decomposition, set lookup, LRU replacement,
 * and statistics tracking.
 *
 * @example
 * ```ts
 * const config = new CacheConfig("L1D", 1024, 64, 4, 1);
 * const cache = new Cache(config);
 * const access = cache.read(0x100, 1, 0);
 * access.hit;  // false
 * const access2 = cache.read(0x100, 1, 1);
 * access2.hit; // true
 * ```
 */
export class Cache {
  readonly config: CacheConfig;
  readonly stats: CacheStats;
  readonly sets: CacheSet[];

  /**
   * Precomputed bit positions for address decomposition.
   * These are used as shift amounts and masks in decomposeAddress.
   *
   *   offsetBits = log2(lineSize)    e.g., log2(64) = 6
   *   setBits    = log2(numSets)     e.g., log2(256) = 8
   *
   * For a direct-mapped cache with 1 set per line, setBits = log2(numLines).
   * For a 1-set cache (fully associative), setBits = 0.
   */
  private readonly _offsetBits: number;
  private readonly _setBits: number;
  private readonly _setMask: number;

  /**
   * Initialize the cache with the given configuration.
   *
   * Creates all sets, precomputes bit positions for address
   * decomposition, and initializes statistics.
   *
   * @param config - Cache parameters (size, associativity, latency, etc.)
   */
  constructor(config: CacheConfig) {
    this.config = config;
    this.stats = new CacheStats();

    // Create the set array
    const numSets = config.numSets;
    this.sets = Array.from(
      { length: numSets },
      () => new CacheSet(config.associativity, config.lineSize),
    );

    // Precompute bit positions for address decomposition
    this._offsetBits = Math.log2(config.lineSize);
    this._setBits = numSets > 1 ? Math.log2(numSets) : 0;
    this._setMask = numSets - 1; // e.g., 0xFF for 256 sets
  }

  // ── Address Decomposition ───────────────────────────────────────────

  /**
   * Split a memory address into [tag, setIndex, offset].
   *
   * This is pure bit manipulation — no division needed because all
   * sizes are powers of 2.
   *
   * Example for 64KB cache, 64B lines, 256 sets:
   *     address = 0x1A2B3C4D
   *     offset     = address & 0x3F              = 0x0D (13)
   *     setIndex   = (address >> 6) & 0xFF       = 0xF1 (241)
   *     tag        = address >> 14               = 0x68AC
   *
   * @param address - Full memory address (unsigned integer).
   * @returns [tag, setIndex, offset] tuple.
   */
  decomposeAddress(address: number): [number, number, number] {
    const offset = address & ((1 << this._offsetBits) - 1);
    const setIndex = (address >>> this._offsetBits) & this._setMask;
    const tag = address >>> (this._offsetBits + this._setBits);
    return [tag, setIndex, offset];
  }

  // ── Read ────────────────────────────────────────────────────────────

  /**
   * Read data from the cache.
   *
   * On a hit, the data is returned immediately with the cache's
   * access latency. On a miss, dummy data is allocated (the caller
   * — typically the hierarchy — is responsible for actually fetching
   * from the next level).
   *
   * @param address - Memory address to read.
   * @param size    - Number of bytes to read (for stats; actual data is
   *                  at cache-line granularity).
   * @param cycle   - Current clock cycle.
   * @returns CacheAccess record describing what happened.
   */
  read(address: number, size = 1, cycle = 0): CacheAccess {
    const [tag, setIndex, offset] = this.decomposeAddress(address);
    const cacheSet = this.sets[setIndex];

    const [hit, line] = cacheSet.access(tag, cycle);

    if (hit) {
      this.stats.recordRead(true);
      return {
        address,
        hit: true,
        tag,
        setIndex,
        offset,
        cycles: this.config.accessLatency,
        evicted: null,
      };
    }

    // Miss — allocate the line with dummy data.
    // In a real system, the hierarchy fetches from the next level
    // and fills this line. Here we simulate by filling with zeros.
    this.stats.recordRead(false);
    const evicted = cacheSet.allocate(
      tag,
      new Array<number>(this.config.lineSize).fill(0),
      cycle,
    );
    if (evicted !== null) {
      this.stats.recordEviction(true);
    } else if (this._allWaysWereValid(cacheSet, tag)) {
      // A valid but clean line was evicted
      this.stats.recordEviction(false);
    }

    return {
      address,
      hit: false,
      tag,
      setIndex,
      offset,
      cycles: this.config.accessLatency,
      evicted,
    };
  }

  // ── Write ───────────────────────────────────────────────────────────

  /**
   * Write data to the cache.
   *
   * **Write-back policy**: Write only to the cache. Mark the line
   * as dirty. The data is written to the next level only when the
   * line is evicted.
   *
   * **Write-through policy**: Write to both the cache and the next
   * level simultaneously. The line is never dirty.
   *
   * On a write miss, we use **write-allocate**: first bring the
   * line into the cache (like a read miss), then perform the write.
   * This is the most common policy on modern CPUs.
   *
   * @param address - Memory address to write.
   * @param data    - Bytes to write (optional; if null, just marks dirty).
   * @param cycle   - Current clock cycle.
   * @returns CacheAccess record describing what happened.
   */
  write(address: number, data: number[] | null = null, cycle = 0): CacheAccess {
    const [tag, setIndex, offset] = this.decomposeAddress(address);
    const cacheSet = this.sets[setIndex];

    const [hit, line] = cacheSet.access(tag, cycle);

    if (hit) {
      this.stats.recordWrite(true);
      // Write the data into the line
      if (data !== null) {
        for (let i = 0; i < data.length; i++) {
          if (offset + i < line.data.length) {
            line.data[offset + i] = data[i];
          }
        }
      }
      // Mark dirty for write-back; write-through stays clean
      if (this.config.writePolicy === "write-back") {
        line.dirty = true;
      }
      return {
        address,
        hit: true,
        tag,
        setIndex,
        offset,
        cycles: this.config.accessLatency,
        evicted: null,
      };
    }

    // Write miss — allocate (write-allocate policy), then write
    this.stats.recordWrite(false);
    const fillData = new Array<number>(this.config.lineSize).fill(0);
    if (data !== null) {
      for (let i = 0; i < data.length; i++) {
        if (offset + i < fillData.length) {
          fillData[offset + i] = data[i];
        }
      }
    }

    const evicted = cacheSet.allocate(tag, fillData, cycle);
    if (evicted !== null) {
      this.stats.recordEviction(true);
    } else if (this._allWaysWereValid(cacheSet, tag)) {
      this.stats.recordEviction(false);
    }

    // For write-back, mark the newly allocated line as dirty
    // (it has new data that isn't in the next level)
    const [newHit, newLine] = cacheSet.access(tag, cycle);
    if (newHit && this.config.writePolicy === "write-back") {
      newLine.dirty = true;
    }

    return {
      address,
      hit: false,
      tag,
      setIndex,
      offset,
      cycles: this.config.accessLatency,
      evicted,
    };
  }

  // ── Helpers ─────────────────────────────────────────────────────────

  /**
   * Check if all ways in a set are valid (meaning an eviction occurred).
   *
   * After allocate(), the new line is already in place. We check if
   * all ways are now valid — if so, one of them must have been replaced.
   *
   * We exclude the just-allocated tag from the check (it's the new line).
   * Actually, since allocate always fills an invalid slot first, if all
   * are valid now and one has the new tag, then all *were* valid before.
   */
  private _allWaysWereValid(_cacheSet: CacheSet, _currentTag: number): boolean {
    return _cacheSet.lines.every((line) => line.valid);
  }

  /**
   * Invalidate all lines in the cache (cache flush).
   *
   * This is equivalent to a cold start — after invalidation, every
   * access will be a compulsory miss. Used when context-switching
   * between processes or when explicitly flushing (e.g., for I/O
   * coherence).
   */
  invalidate(): void {
    for (const cacheSet of this.sets) {
      for (const line of cacheSet.lines) {
        line.invalidate();
      }
    }
  }

  /**
   * Directly fill a cache line with data (used by hierarchy on miss).
   *
   * This bypasses the normal read/write path — it's used when the
   * hierarchy fetches data from a lower level and wants to install
   * it in this cache.
   *
   * @param address - The address whose line we're filling.
   * @param data    - The full cache line of data from the lower level.
   * @param cycle   - Current clock cycle.
   * @returns Evicted dirty CacheLine if a writeback is needed, else null.
   */
  fillLine(address: number, data: number[], cycle = 0): CacheLine | null {
    const [tag, setIndex] = this.decomposeAddress(address);
    const cacheSet = this.sets[setIndex];
    return cacheSet.allocate(tag, data, cycle);
  }

  /** Human-readable summary of the cache configuration. */
  toString(): string {
    return (
      `Cache(${this.config.name}: ` +
      `${Math.floor(this.config.totalSize / 1024)}KB, ` +
      `${this.config.associativity}-way, ` +
      `${this.config.lineSize}B lines, ` +
      `${this.config.numSets} sets)`
    );
  }
}
