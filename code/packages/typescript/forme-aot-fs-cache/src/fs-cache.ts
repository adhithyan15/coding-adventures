/**
 * fs-cache.ts — disk-backed `CacheIO` (FM06 §4 storage backend).
 *
 * Implements the `CacheIO` contract from
 * `forme-aot-incremental-cache` over `node:fs`.  Two-level sharded
 * layout under `cacheDir`:
 *
 *   cacheDir/
 *     <first 2 hex chars of key>/
 *       <remaining 62 hex chars of key>.cache
 *
 * The first 2 hex chars (8 bits) split keys across 256 sub-dirs —
 * keeps any single directory under ~10k entries for caches up to
 * 2.5M total keys, comfortably below typical filesystem dir limits
 * (ext4: 64k, APFS: practically unbounded but degrades, NTFS: 4G
 * but slow above 300k).
 *
 * ## Key validation
 *
 * Cache keys from `forme-aot-incremental-cache` are sha256 hex
 * strings (`/^[0-9a-f]{64}$/`).  We **validate every key** before
 * touching the filesystem — anything outside that grammar is
 * rejected.  This makes path traversal impossible (the key never
 * contains `..` or `/`) and guards against future callers that
 * might wire a different cache layer to this IO.
 *
 * ## Atomic writes (default on)
 *
 * `put(key, value, meta)` writes to a temp file then `fs.rename`s
 * onto the final path.  POSIX guarantees `rename` is atomic within
 * a single filesystem, so concurrent puts for the same key never
 * leave a partial-write reader.  Cost: one extra file create per
 * write.  Set `atomicWrites: false` to skip when you trust the
 * caller (single-process, single-thread).
 *
 * ## Symlink safety
 *
 * All file IO uses the `fs.promises` API which does NOT follow
 * symlinks for write operations on the *parent* path components by
 * default in modern Node.  We never call `realpath`, never resolve
 * `..`, and reject non-sha256-shaped keys, so the worst an attacker
 * controlling the `cacheDir` could do is point it at a directory
 * they want us to write into — and they had to give us write
 * permission for that anyway.
 *
 * @module fs-cache
 */

import { promises as fs } from "node:fs";
import * as path from "node:path";
import type {
  CacheIO, CachePutMeta,
} from "@coding-adventures/forme-aot-incremental-cache";

// ─── Public types ────────────────────────────────────────────────────────

export interface FsCacheOptions {
  /**
   * Directory under which cache entries are stored.  MUST exist
   * before calling `createFsCacheIO` — we don't auto-create
   * (a missing dir is a configuration error worth surfacing
   * loudly).  Shard sub-dirs (256 of them) ARE created on demand.
   */
  readonly cacheDir: string;
  /**
   * Write to a temp file then `rename` onto the final path.  Default
   * `true`.  Set to `false` to skip when concurrent writes to the
   * same key are impossible (single-process, single-thread, or
   * caller-coordinated).
   */
  readonly atomicWrites?: boolean;
}

// ─── Public factory ──────────────────────────────────────────────────────

const HEX64_RE = /^[0-9a-f]{64}$/;

/**
 * Construct a disk-backed `CacheIO`.  Throws synchronously (via
 * the returned `Promise` rejection on first call) if `cacheDir`
 * doesn't exist or isn't a directory.
 *
 * Each call returns an independent instance; instances are stateless
 * apart from the `cacheDir` reference.  Concurrent instances over
 * the same `cacheDir` are safe — atomic writes prevent partial-state
 * reads.
 */
export function createFsCacheIO(options: FsCacheOptions): CacheIO {
  const cacheDir = options.cacheDir;
  const atomicWrites = options.atomicWrites ?? true;

  return {
    get: async (key) => {
      assertValidKey(key);
      const file = keyToFilePath(cacheDir, key);
      try {
        return await fs.readFile(file, "utf8");
      } catch (e) {
        // ENOENT is a normal "not in cache" — return null.  Any
        // other error (EACCES, EIO, …) is a configuration / disk
        // problem that should surface.
        if ((e as NodeJS.ErrnoException).code === "ENOENT") return null;
        throw e;
      }
    },

    put: async (key, value, _meta: CachePutMeta) => {
      assertValidKey(key);
      await ensureCacheDirValid(cacheDir);
      const file = keyToFilePath(cacheDir, key);
      // Ensure shard dir exists (256 of them; created on demand).
      await fs.mkdir(path.dirname(file), { recursive: true });

      if (atomicWrites) {
        // Write to <file>.tmp.<pid>.<rand> then rename atomically.
        // The temp name uses a fresh random suffix per call so two
        // concurrent puts to the same key don't collide on the
        // temp file (the rename is what's atomic; the temp file is
        // single-writer).
        const tmpFile = `${file}.tmp.${process.pid}.${randomSuffix()}`;
        try {
          await fs.writeFile(tmpFile, value, { encoding: "utf8" });
          await fs.rename(tmpFile, file);
        } catch (e) {
          // Best-effort cleanup of the temp file on write failure.
          // We swallow the unlink error because the original write
          // error is the one the caller needs to see.
          try { await fs.unlink(tmpFile); } catch { /* ignore */ }
          throw e;
        }
      } else {
        await fs.writeFile(file, value, { encoding: "utf8" });
      }
    },

    list: async () => {
      // Walk shard dirs one level deep; collect all `.cache` files.
      // Order: lexicographic, deterministic.
      let shards: string[];
      try {
        shards = (await fs.readdir(cacheDir)).filter((d) => /^[0-9a-f]{2}$/.test(d)).sort();
      } catch (e) {
        if ((e as NodeJS.ErrnoException).code === "ENOENT") return Object.freeze([]);
        throw e;
      }

      const keys: string[] = [];
      for (const shard of shards) {
        const shardPath = path.join(cacheDir, shard);
        let entries: string[];
        try {
          entries = await fs.readdir(shardPath);
        } catch {
          continue;
        }
        for (const entry of entries.sort()) {
          // Only `<62 hex>.cache` filenames count — ignore stray
          // temp files (`*.tmp.*`) and anything else.
          const m = /^([0-9a-f]{62})\.cache$/.exec(entry);
          if (m === null) continue;
          keys.push(shard + m[1]);
        }
      }
      return Object.freeze(keys);
    },
  };
}

// ─── Key validation + path mapping ───────────────────────────────────────

function assertValidKey(key: string): void {
  if (!HEX64_RE.test(key)) {
    throw new TypeError(
      `forme-aot-fs-cache: invalid cache key ${JSON.stringify(key)} — expected /^[0-9a-f]{64}$/`,
    );
  }
}

/**
 * Map a sha256-hex key to its on-disk path.  The first 2 hex chars
 * become a shard sub-directory; the remaining 62 form the filename
 * with `.cache` suffix.
 */
function keyToFilePath(cacheDir: string, key: string): string {
  // Caller has already validated the key shape — no need to
  // re-defend.  But `path.join` will normalise away any sneaky `..`
  // even if assertValidKey were bypassed.
  return path.join(cacheDir, key.slice(0, 2), `${key.slice(2)}.cache`);
}

// ─── cacheDir validation ─────────────────────────────────────────────────

/**
 * Verify that `cacheDir` exists and is a directory.  Cached after the
 * first successful check (we don't expect the dir to disappear mid-
 * run; if it does, subsequent ops will surface ENOENT cleanly).
 *
 * We do this lazily on first `put` rather than synchronously in the
 * factory so that misconfigured `cacheDir` doesn't crash callers at
 * import time — they get an awaitable rejection at first use.
 */
/**
 * Module-scoped Set of cacheDir strings we've already validated.
 * Bounded in practice by the number of distinct cacheDir paths a
 * process uses (typically 1-2, so unbounded growth is theoretical).
 */
const validatedDirs = new Set<string>();

async function ensureCacheDirValid(cacheDir: string): Promise<void> {
  if (validatedDirs.has(cacheDir)) return;
  let stat;
  try {
    stat = await fs.stat(cacheDir);
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") {
      throw new Error(
        `forme-aot-fs-cache: cacheDir ${JSON.stringify(cacheDir)} does not exist — create it before calling createFsCacheIO`,
      );
    }
    throw e;
  }
  if (!stat.isDirectory()) {
    throw new Error(
      `forme-aot-fs-cache: cacheDir ${JSON.stringify(cacheDir)} exists but is not a directory`,
    );
  }
  validatedDirs.add(cacheDir);
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/**
 * Random 12-hex-char suffix for temp filenames.  Math.random is fine
 * here — we're only avoiding collision with other concurrent writes
 * in the same process, not defending against an attacker who could
 * predict the suffix (they'd have to be inside our process already,
 * which is game-over for any defence).
 */
function randomSuffix(): string {
  return Math.floor(Math.random() * 0xffffffffffff).toString(16).padStart(12, "0");
}
