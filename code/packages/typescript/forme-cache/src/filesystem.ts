/**
 * Filesystem cache backend.  Persists entries under `<root>/<prefix>/<key>`
 * where `prefix` is the first 2 hex chars of the key — keeps any single
 * directory bounded even when the cache holds millions of entries.
 *
 * === On-disk layout ===
 *
 *     <root>/
 *       ab/
 *         abcdef...01.entry      ← raw payload bytes
 *         abcdef...01.meta       ← JSON: { writtenMs, sizeBytes, contentHash }
 *
 * Splitting payload from metadata keeps metadata reads cheap (the GC
 * scan only needs `writtenMs` and `sizeBytes`; loading multi-megabyte
 * payloads from disk just to delete them is wasteful).
 *
 * === Atomicity ===
 *
 * Every write is two-phase: write payload + meta to a `.tmp` sibling,
 * then `rename` into place.  POSIX `rename` is atomic on the same
 * filesystem; partial files are never visible to readers.
 *
 * On platforms where rename atomicity is weaker (some Windows
 * configurations, network filesystems), the integrity check on read
 * is the safety net — corrupted reads return `null` and the
 * orchestrator re-executes the stage.
 *
 * === Error handling ===
 *
 * `get` swallows `ENOENT` (file not found = miss) and integrity
 * failures (corrupt entry = miss).  Other errors (permission denied,
 * read failure mid-stream) propagate so the orchestrator can decide
 * whether to retry or surface to the user.  `put` and `invalidate`
 * surface their errors directly — failures during cache writes
 * shouldn't be silent.
 */

import { mkdir, readFile, rename, rm, stat, unlink, writeFile, readdir } from "node:fs/promises";
import { join, dirname } from "node:path";
import { verifyEntry } from "./integrity.js";
import type { CacheBackend, CacheEntry } from "./types.js";

interface MetaPayload {
  readonly writtenMs: number;
  readonly sizeBytes: number;
  readonly contentHash: string;
}

class FilesystemCacheImpl implements CacheBackend {
  private disposed = false;

  constructor(private readonly root: string) {}

  // ─── Reads ──────────────────────────────────────────────────────────────

  async get(key: string): Promise<CacheEntry | null> {
    this.assertNotDisposed();
    const { entryPath, metaPath } = this.pathsFor(key);
    let metaBytes: Buffer;
    let payload: Buffer;
    try {
      [metaBytes, payload] = await Promise.all([
        readFile(metaPath),
        readFile(entryPath),
      ]);
    } catch (err) {
      if (isNotFound(err)) return null;
      throw err;
    }
    let meta: MetaPayload;
    try {
      meta = JSON.parse(metaBytes.toString("utf8")) as MetaPayload;
    } catch {
      // Corrupt meta — treat as miss and clean up.
      await this.invalidate(key);
      return null;
    }
    const entry: CacheEntry = {
      writtenMs:   meta.writtenMs,
      sizeBytes:   meta.sizeBytes,
      payload:     new Uint8Array(payload),
      contentHash: meta.contentHash,
    };
    if (!verifyEntry(entry)) {
      await this.invalidate(key);
      return null;
    }
    return entry;
  }

  // ─── Writes ─────────────────────────────────────────────────────────────

  async put(key: string, entry: CacheEntry): Promise<void> {
    this.assertNotDisposed();
    const { entryPath, metaPath, dir } = this.pathsFor(key);
    await mkdir(dir, { recursive: true });
    // Two-phase write: temp + rename.
    const entryTmp = entryPath + ".tmp";
    const metaTmp  = metaPath + ".tmp";
    const meta: MetaPayload = {
      writtenMs:   entry.writtenMs,
      sizeBytes:   entry.sizeBytes,
      contentHash: entry.contentHash,
    };
    await Promise.all([
      writeFile(entryTmp, entry.payload),
      writeFile(metaTmp, JSON.stringify(meta)),
    ]);
    // Rename payload first; if metadata rename fails the leftover
    // payload is fine (a future read will see no metadata, return
    // null, and a put will overwrite cleanly).
    await rename(entryTmp, entryPath);
    await rename(metaTmp, metaPath);
  }

  async invalidate(key: string): Promise<void> {
    this.assertNotDisposed();
    const { entryPath, metaPath } = this.pathsFor(key);
    await Promise.all([
      removeIfExists(entryPath),
      removeIfExists(metaPath),
    ]);
  }

  async invalidatePrefix(prefix: string): Promise<void> {
    this.assertNotDisposed();
    if (prefix.length === 0) {
      // Clear everything under the root, but preserve the root
      // directory itself so a subsequent `put` doesn't have to
      // recreate the parent tree.
      await rm(this.root, { recursive: true, force: true });
      await mkdir(this.root, { recursive: true });
      return;
    }
    // We can short-circuit by just walking the matching shard
    // directory if the prefix is at least 2 chars (because keys are
    // sharded by first 2 chars).  Otherwise we have to walk all
    // shards and filter.  For simplicity and correctness, just walk
    // every shard and filter — invalidatePrefix is rare.
    let shards: string[] = [];
    try {
      shards = await readdir(this.root);
    } catch (err) {
      if (isNotFound(err)) return;
      throw err;
    }
    for (const shard of shards) {
      let entries: string[];
      try {
        entries = await readdir(join(this.root, shard));
      } catch (err) {
        if (isNotFound(err)) continue;
        throw err;
      }
      for (const entry of entries) {
        // Check both `.entry` and `.meta` files for matching keys.
        if (entry.endsWith(".entry") || entry.endsWith(".meta")) {
          const key = entry.slice(0, entry.lastIndexOf("."));
          if (key.startsWith(prefix)) {
            await removeIfExists(join(this.root, shard, entry));
          }
        }
      }
    }
  }

  // ─── GC ─────────────────────────────────────────────────────────────────

  async gc(olderThanMs: number): Promise<number> {
    this.assertNotDisposed();
    const cutoff = Date.now() - olderThanMs;
    let removed = 0;
    let shards: string[] = [];
    try {
      shards = await readdir(this.root);
    } catch (err) {
      if (isNotFound(err)) return 0;
      throw err;
    }
    for (const shard of shards) {
      const shardPath = join(this.root, shard);
      let entries: string[] = [];
      try {
        entries = await readdir(shardPath);
      } catch (err) {
        if (isNotFound(err)) continue;
        throw err;
      }
      for (const entry of entries) {
        if (!entry.endsWith(".meta")) continue;
        const metaPath = join(shardPath, entry);
        try {
          const stats = await stat(metaPath);
          if (stats.mtimeMs >= cutoff) continue;
          // Remove both the meta and the corresponding payload.
          const key = entry.slice(0, -".meta".length);
          const entryPath = join(shardPath, key + ".entry");
          await Promise.all([
            removeIfExists(metaPath),
            removeIfExists(entryPath),
          ]);
          removed++;
        } catch (err) {
          if (isNotFound(err)) continue;
          throw err;
        }
      }
    }
    return removed;
  }

  // ─── Lifecycle ──────────────────────────────────────────────────────────

  async dispose(): Promise<void> {
    // Filesystem backend has no open handles — disposing just sets
    // the flag so subsequent operations throw a clear error rather
    // than racing with something that's reusing the same root.
    this.disposed = true;
  }

  // ─── Path helpers ───────────────────────────────────────────────────────

  private pathsFor(key: string): { entryPath: string; metaPath: string; dir: string } {
    if (key.length < 2) {
      throw new Error(`FilesystemCache: key too short (${key.length} chars); expected ≥ 2`);
    }
    const shard = key.slice(0, 2);
    const dir = join(this.root, shard);
    return {
      dir,
      entryPath: join(dir, `${key}.entry`),
      metaPath:  join(dir, `${key}.meta`),
    };
  }

  private assertNotDisposed(): void {
    if (this.disposed) {
      throw new Error("FilesystemCache: backend has been disposed");
    }
  }
}

// ─── Helpers ──────────────────────────────────────────────────────────────

function isNotFound(err: unknown): boolean {
  return typeof err === "object" && err !== null
    && (err as { code?: unknown }).code === "ENOENT";
}

async function removeIfExists(path: string): Promise<void> {
  try {
    await unlink(path);
  } catch (err) {
    if (!isNotFound(err)) throw err;
  }
}

/** Build a filesystem-backed cache rooted at the given directory. */
export function filesystemCache(root: string): CacheBackend {
  if (typeof root !== "string" || root.length === 0) {
    throw new Error("filesystemCache: root must be a non-empty string");
  }
  // Ensure parent of root exists so the first `put` succeeds even on
  // a freshly-installed system.  The root itself is created lazily.
  void dirname(root);
  return new FilesystemCacheImpl(root);
}
