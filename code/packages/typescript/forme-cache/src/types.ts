/**
 * Cache backend interface and entry shape — FM03 §5.
 *
 * Two layers:
 *
 *   - `CacheEntry` — the stored value.  Carries the serialised payload
 *     bytes plus enough metadata for time-based GC and integrity
 *     verification on read.
 *
 *   - `CacheBackend` — the storage adapter.  Two built-in
 *     implementations ship in this package (`MemoryCache`,
 *     `FilesystemCache`); a future `RemoteCache` backend can plug into
 *     the same interface for distributed builds without touching the
 *     orchestrator.
 *
 * The orchestrator owns key derivation (see `keys.ts`) and integrity
 * verification (see `integrity.ts`); backends are storage-only.
 *
 * === Why payload is bytes, not value ===
 *
 * Each kind has its own canonical encoder.  The orchestrator runs the
 * encoder before calling `put` and the decoder after `get`.  Backends
 * see only opaque bytes — they don't need to know about kinds, types,
 * or schemas, which means a future `RemoteCache` can be implemented
 * without dragging in the type system.
 */

/** A cache entry — the on-disk / in-memory representation of one stored value. */
export interface CacheEntry {
  /** When this entry was written (ms since epoch). */
  readonly writtenMs: number;
  /** Total size of `payload` in bytes.  Duplicated for cheap GC checks. */
  readonly sizeBytes: number;
  /** The serialised payload — kind-specific encoder controlled. */
  readonly payload: Uint8Array;
  /**
   * BLAKE2b-256 hex digest of `payload`.  On read, backends MUST
   * recompute and compare; a mismatch is treated as a cache miss
   * (and the entry should be invalidated to clean up corruption).
   */
  readonly contentHash: string;
}

/**
 * Storage adapter contract.  All methods are async even when the
 * backend is in-memory — keeps the public surface uniform across
 * in-memory, filesystem, and future remote backends.
 */
export interface CacheBackend {
  /**
   * Look up an entry by key.  Returns `null` on miss OR on integrity
   * failure (the orchestrator treats both the same way: re-execute
   * the stage).
   */
  get(key: string): Promise<CacheEntry | null>;

  /** Store an entry.  Overwrites any existing entry under the same key. */
  put(key: string, entry: CacheEntry): Promise<void>;

  /** Drop a single entry.  No-op if the key isn't present. */
  invalidate(key: string): Promise<void>;

  /**
   * Optional: bulk-invalidate every entry whose key starts with the
   * given prefix.  Used by `forme cache clear --stage <name>`.
   * Backends that can't efficiently support this MAY omit the method;
   * the orchestrator falls back to a per-key sweep.
   */
  invalidatePrefix?(prefix: string): Promise<void>;

  /**
   * Garbage-collect entries older than `olderThanMs` (a duration, not
   * a timestamp).  Returns the number of entries removed.  Called on
   * orchestrator startup with a default age of 30 days.
   */
  gc(olderThanMs: number): Promise<number>;

  /**
   * Release any resources the backend holds (open file handles,
   * remote connections).  Called once at orchestrator shutdown.
   */
  dispose(): Promise<void>;
}
