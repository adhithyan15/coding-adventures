/**
 * Cache line — the smallest unit of data in a cache.
 *
 * In a real CPU, data is not moved one byte at a time between memory and the
 * cache. Instead, it moves in fixed-size chunks called **cache lines** (also
 * called cache blocks). A typical cache line is 64 bytes.
 *
 * Analogy: Think of a warehouse that ships goods in standard containers.
 * You can't order a single screw — you get the whole container (cache line)
 * that includes the screw you need plus 63 other bytes of nearby data.
 * This works well because of **spatial locality**: if you accessed byte N,
 * you'll likely access bytes N+1, N+2, ... soon.
 *
 * Each cache line stores:
 *
 *     +-------+-------+-----+------+---------------------------+
 *     | valid | dirty | tag | LRU  |     data (64 bytes)       |
 *     +-------+-------+-----+------+---------------------------+
 *
 * - **valid**: Is this line holding real data? After a reset, all lines are
 *   invalid (empty boxes). A line becomes valid when data is loaded into it.
 *
 * - **dirty**: Has the data been modified since it was loaded from memory?
 *   In a write-back cache, writes go only to the cache (not memory). The
 *   dirty bit tracks whether the line needs to be written back to memory
 *   when evicted. (Like editing a document locally — you need to save it
 *   back to the server before closing.)
 *
 * - **tag**: The high bits of the memory address. Since many addresses map
 *   to the same cache set (like many apartments on the same floor), the tag
 *   distinguishes WHICH address is actually stored here.
 *
 * - **data**: The actual bytes — an array of numbers, each 0-255.
 *
 * - **lastAccess**: A timestamp (cycle count) recording when this line was
 *   last read or written. Used by the LRU replacement policy to decide
 *   which line to evict when the set is full.
 */

// ── CacheLine ───────────────────────────────────────────────────────────

/**
 * A single cache line — one slot in the cache.
 *
 * @example
 * ```ts
 * const line = new CacheLine(64);
 * line.valid;       // false
 * line.fill(42, new Array(64).fill(0xAB), 100);
 * line.valid;       // true
 * line.tag;         // 42
 * line.lastAccess;  // 100
 * ```
 */
export class CacheLine {
  // ── Cache line metadata ─────────────────────────────────────────────

  /** Is this line holding real data? */
  valid = false;

  /** Has this line been modified? (write-back policy tracking) */
  dirty = false;

  /** High bits of the address — identifies which memory block is cached. */
  tag = 0;

  /** Cycle count of last access — used for LRU replacement. */
  lastAccess = 0;

  // ── Data payload ────────────────────────────────────────────────────

  /** The actual bytes stored in this cache line (each 0-255). */
  data: number[];

  /**
   * Create a new invalid cache line with the given size.
   *
   * @param lineSize - Number of bytes per cache line. Defaults to 64,
   *                   which is standard on modern x86 and ARM CPUs.
   */
  constructor(lineSize = 64) {
    this.data = new Array<number>(lineSize).fill(0);
  }

  // ── Operations ──────────────────────────────────────────────────────

  /**
   * Load data into this cache line, marking it valid.
   *
   * This is called when a cache miss brings data from a lower level
   * (L2, L3, or main memory) into this line.
   *
   * @param tag   - The tag bits for the address being cached.
   * @param data  - The bytes to store (must match lineSize).
   * @param cycle - Current clock cycle (for LRU tracking).
   */
  fill(tag: number, data: number[], cycle: number): void {
    this.valid = true;
    this.dirty = false; // freshly loaded data is clean
    this.tag = tag;
    this.data = [...data]; // defensive copy
    this.lastAccess = cycle;
  }

  /**
   * Update the last access time — called on every hit.
   *
   * This is the heartbeat of LRU: the most recently used line
   * gets the highest timestamp, so it's the *last* to be evicted.
   */
  touch(cycle: number): void {
    this.lastAccess = cycle;
  }

  /**
   * Mark this line as invalid (empty).
   *
   * Used during cache flushes or coherence protocol invalidations.
   * The data is not zeroed — it's just marked as not-present.
   */
  invalidate(): void {
    this.valid = false;
    this.dirty = false;
  }

  /** Number of bytes in this cache line. */
  get lineSize(): number {
    return this.data.length;
  }

  /** Compact representation for debugging. */
  toString(): string {
    let state = this.valid ? "V" : "-";
    state += this.dirty ? "D" : "-";
    return `CacheLine(${state}, tag=0x${this.tag.toString(16).toUpperCase()}, lru=${this.lastAccess})`;
  }
}
