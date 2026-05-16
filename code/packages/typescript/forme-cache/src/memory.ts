/**
 * In-memory cache backend.  Lives only in process memory; gone on
 * dispose.  The intended use is tests and the orchestrator's
 * watch-mode "front cache" (FM03 §7.5) — not for cross-run persistence.
 *
 * Implementation is a `Map<string, CacheEntry>`.  Integrity is verified
 * on every `get` even though in-memory writes can't be tampered with —
 * the discipline is to keep the contract identical across backends so
 * downstream tests can swap implementations without changing behaviour.
 */

import { verifyEntry } from "./integrity.js";
import type { CacheBackend, CacheEntry } from "./types.js";

class MemoryCacheImpl implements CacheBackend {
  private entries = new Map<string, CacheEntry>();
  private disposed = false;

  async get(key: string): Promise<CacheEntry | null> {
    this.assertNotDisposed();
    const entry = this.entries.get(key);
    if (!entry) return null;
    if (!verifyEntry(entry)) {
      // Corrupt — invalidate as part of the read.  Same convention
      // FilesystemCache uses, so swap-in-and-out doesn't change
      // observable behaviour.
      this.entries.delete(key);
      return null;
    }
    return entry;
  }

  async put(key: string, entry: CacheEntry): Promise<void> {
    this.assertNotDisposed();
    this.entries.set(key, entry);
  }

  async invalidate(key: string): Promise<void> {
    this.assertNotDisposed();
    this.entries.delete(key);
  }

  async invalidatePrefix(prefix: string): Promise<void> {
    this.assertNotDisposed();
    if (prefix.length === 0) {
      this.entries.clear();
      return;
    }
    for (const key of this.entries.keys()) {
      if (key.startsWith(prefix)) this.entries.delete(key);
    }
  }

  async gc(olderThanMs: number): Promise<number> {
    this.assertNotDisposed();
    const cutoff = Date.now() - olderThanMs;
    let removed = 0;
    for (const [key, entry] of this.entries) {
      if (entry.writtenMs < cutoff) {
        this.entries.delete(key);
        removed++;
      }
    }
    return removed;
  }

  async dispose(): Promise<void> {
    this.disposed = true;
    this.entries.clear();
  }

  private assertNotDisposed(): void {
    if (this.disposed) {
      throw new Error("MemoryCache: backend has been disposed");
    }
  }
}

/** Build a fresh in-memory cache backend. */
export function memoryCache(): CacheBackend {
  return new MemoryCacheImpl();
}
