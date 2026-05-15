/**
 * Stage-local cache (FM01 §4.5).
 *
 * Three operations:
 *
 *   - `getOrCompute(key, compute)` — return cached value or compute,
 *     store, and return.  This is the primary API; stages should
 *     almost never call `get`/`set` directly.
 *   - `invalidate(key)` — drop a single entry.
 *   - `keyFor(parts)` — derive a stable key string from a tuple of
 *     primitive parts, with proper escaping of separators.
 *
 * Implementations are managed by the orchestrator (FM03) and scoped
 * per-stage, per-pipeline-run.  This package only exposes the
 * interface and a simple in-memory implementation suitable for tests
 * and development.
 *
 * === In-memory implementation ===
 *
 * `inMemoryCache()` is a Map-backed cache that lives only as long as
 * the surrounding process.  It honours the `getOrCompute` contract by
 * memoising the computation promise itself — concurrent calls with
 * the same key share a single computation, which prevents the common
 * "two requests racing to populate the same key" bug.
 *
 * The cache is intentionally untyped at the storage layer — `unknown`
 * in, `unknown` out, with the caller's type assertion at the boundary.
 * Typing per-key would force every consumer through a registry just to
 * declare key types, which is more friction than the safety buys at
 * stage-local scope.  The orchestrator's persistent cache (`forme-cache`)
 * is where typed serialisation lives.
 */

/** Stage-local cache contract. */
export interface Cache {
  /**
   * Return the cached value for `key`, computing and storing it if
   * absent.  Concurrent callers with the same key share one
   * computation — the resulting promise is memoised, not the awaited
   * value, so two simultaneous misses do not race.
   */
  getOrCompute<T>(key: string, compute: () => Promise<T>): Promise<T>;
  /** Drop an entry.  No-op if the key is not present. */
  invalidate(key: string): Promise<void>;
  /**
   * Derive a stable key string from primitive parts.  Each part is
   * stringified and joined with a separator that is escaped within
   * each part — `keyFor(["a:b", "c"]) !== keyFor(["a", "b:c"])`.
   */
  keyFor(parts: readonly (string | number)[]): string;
}

// ─── In-memory implementation ─────────────────────────────────────────────

const KEY_SEP = "\x1F"; // ASCII Unit Separator — won't appear in normal text

class InMemoryCacheImpl implements Cache {
  // We store *promises* rather than awaited values to coalesce
  // concurrent misses (see module header).  When a stored promise
  // rejects, we drop the entry so a retry can re-attempt cleanly.
  private readonly entries = new Map<string, Promise<unknown>>();

  async getOrCompute<T>(key: string, compute: () => Promise<T>): Promise<T> {
    const existing = this.entries.get(key);
    if (existing !== undefined) return existing as Promise<T>;
    const fresh = (async () => {
      try {
        return await compute();
      } catch (err) {
        // Drop on rejection so the next call can retry.  Without this,
        // a single transient error would poison the key forever.
        this.entries.delete(key);
        throw err;
      }
    })();
    this.entries.set(key, fresh);
    return fresh;
  }

  async invalidate(key: string): Promise<void> {
    this.entries.delete(key);
  }

  keyFor(parts: readonly (string | number)[]): string {
    return parts
      .map(p => typeof p === "string" ? escapeKeyPart(p) : String(p))
      .join(KEY_SEP);
  }
}

function escapeKeyPart(s: string): string {
  // Replace any literal Unit Separator in the input so distinct part
  // boundaries can't be impersonated by a malicious key.
  return s.includes(KEY_SEP) ? s.replace(/\x1F/g, "\\x1F") : s;
}

/** Build a fresh in-memory cache.  Each instance is independent. */
export function inMemoryCache(): Cache {
  return new InMemoryCacheImpl();
}
