/**
 * Cache statistics tracking — measuring how well the cache is performing.
 *
 * Every cache keeps a scorecard. Just like a baseball player tracks batting
 * average (hits / at-bats), a cache tracks its **hit rate** (cache hits /
 * total accesses). A high hit rate means the cache is doing its job well —
 * most memory requests are being served quickly from the cache rather than
 * going to slower main memory.
 *
 * Key metrics:
 * - **Reads/Writes**: How many times the CPU asked for data or stored data.
 * - **Hits**: How many times the requested data was already in the cache.
 * - **Misses**: How many times we had to go to a slower level to get the data.
 * - **Evictions**: How many times we had to kick out old data to make room.
 * - **Writebacks**: How many evictions involved dirty data that needed to be
 *   written back to the next level (only relevant for write-back caches).
 *
 * Analogy: Think of a library desk (L1 cache). If you keep the right books
 * on your desk, you rarely need to walk to the shelf (L2). Your "hit rate"
 * is how often the book you need is already on your desk.
 */

// ── CacheStats ──────────────────────────────────────────────────────────

/**
 * Tracks performance statistics for a single cache level.
 *
 * Every read or write to the cache updates these counters. After running
 * a simulation, you can inspect hitRate and missRate to see how
 * effective the cache configuration is for a given workload.
 *
 * @example
 * ```ts
 * const stats = new CacheStats();
 * stats.recordRead(true);
 * stats.recordRead(false);
 * stats.hitRate;   // 0.5
 * stats.missRate;  // 0.5
 * ```
 */
export class CacheStats {
  // ── Counters ────────────────────────────────────────────────────────

  reads = 0;
  writes = 0;
  hits = 0;
  misses = 0;
  evictions = 0;
  /** Dirty evictions that needed writeback. */
  writebacks = 0;

  // ── Derived metrics ─────────────────────────────────────────────────

  /** Total number of read + write operations. */
  get totalAccesses(): number {
    return this.reads + this.writes;
  }

  /**
   * Fraction of accesses that were cache hits (0.0 to 1.0).
   *
   * Returns 0.0 if no accesses have been made (avoid division by zero).
   *
   * A hit rate of 0.95 means 95% of memory requests were served from
   * this cache level — excellent for an L1 cache.
   */
  get hitRate(): number {
    if (this.totalAccesses === 0) return 0.0;
    return this.hits / this.totalAccesses;
  }

  /**
   * Fraction of accesses that were cache misses (0.0 to 1.0).
   *
   * Always equals 1.0 - hitRate. Provided for convenience since
   * miss rate is the more commonly discussed metric in architecture
   * papers ("this workload has a 5% L1 miss rate").
   */
  get missRate(): number {
    if (this.totalAccesses === 0) return 0.0;
    return this.misses / this.totalAccesses;
  }

  // ── Recording methods ───────────────────────────────────────────────

  /** Record a read access. Pass `hit = true` for a cache hit. */
  recordRead(hit: boolean): void {
    this.reads += 1;
    if (hit) {
      this.hits += 1;
    } else {
      this.misses += 1;
    }
  }

  /** Record a write access. Pass `hit = true` for a cache hit. */
  recordWrite(hit: boolean): void {
    this.writes += 1;
    if (hit) {
      this.hits += 1;
    } else {
      this.misses += 1;
    }
  }

  /**
   * Record an eviction. Pass `dirty = true` if the evicted line was dirty.
   *
   * A dirty eviction means the data was modified in the cache but not
   * yet written to the next level. The cache controller must "write back"
   * the dirty data before discarding it — this is the extra cost of a
   * write-back policy.
   */
  recordEviction(dirty: boolean): void {
    this.evictions += 1;
    if (dirty) {
      this.writebacks += 1;
    }
  }

  /**
   * Reset all counters to zero.
   *
   * Useful when you want to measure stats for a specific phase of
   * execution (e.g., "what's the hit rate during matrix multiply?"
   * without counting the initial data loading phase).
   */
  reset(): void {
    this.reads = 0;
    this.writes = 0;
    this.hits = 0;
    this.misses = 0;
    this.evictions = 0;
    this.writebacks = 0;
  }

  /** Human-readable summary of cache statistics. */
  toString(): string {
    return (
      `CacheStats(accesses=${this.totalAccesses}, ` +
      `hits=${this.hits}, misses=${this.misses}, ` +
      `hit_rate=${(this.hitRate * 100).toFixed(1)}%, ` +
      `evictions=${this.evictions}, writebacks=${this.writebacks})`
    );
  }
}
