/**
 * cache.ts -- Build Cache Management
 * ===================================
 *
 * This module manages a JSON-based cache file (`.build-cache.json`) that
 * records the state of each package after its last build. By comparing
 * current hashes against cached hashes, we determine which packages need
 * rebuilding.
 *
 * ## Cache format
 *
 * The cache file is a JSON object mapping package names to cache entries:
 *
 * ```json
 * {
 *   "python/logic-gates": {
 *     "package_hash": "abc123...",
 *     "deps_hash": "def456...",
 *     "last_built": "2024-01-15T10:30:00.000Z",
 *     "status": "success"
 *   }
 * }
 * ```
 *
 * ## Atomic writes
 *
 * To prevent corruption if the process is interrupted mid-write, we write to
 * a temporary file (`.build-cache.json.tmp`) first, then atomically rename
 * it to the final path. On POSIX systems, `fs.renameSync` is atomic within
 * the same filesystem.
 *
 * ## When is the cache used?
 *
 * The cache is a FALLBACK mechanism. The primary change detection uses
 * git-diff (see gitdiff.ts). The cache is only consulted when git-diff
 * is unavailable (e.g., when running locally without a remote).
 */

import * as fs from "node:fs";
import * as path from "node:path";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * A single package's cached build state.
 *
 * This records everything we need to determine if a package needs
 * rebuilding: its source hash, dependency hash, when it was last built,
 * and whether that build succeeded.
 */
export interface CacheEntry {
  /** SHA256 of the package's source files. */
  packageHash: string;
  /** SHA256 of transitive dependency hashes. */
  depsHash: string;
  /** ISO 8601 timestamp of the last build. */
  lastBuilt: string;
  /** "success" or "failed". */
  status: string;
}

// ---------------------------------------------------------------------------
// BuildCache class
// ---------------------------------------------------------------------------

/**
 * Read/write interface for the build cache file.
 *
 * ## Usage example
 *
 * ```typescript
 * const cache = new BuildCache();
 * cache.load("/repo/.build-cache.json");
 *
 * if (cache.needsBuild("python/logic-gates", pkgHash, depsHash)) {
 *     // ... run build ...
 *     cache.record("python/logic-gates", pkgHash, depsHash, "success");
 * }
 *
 * cache.save("/repo/.build-cache.json");
 * ```
 */
export class BuildCache {
  /** Internal storage: package name -> cache entry. */
  private _entries: Map<string, CacheEntry> = new Map();

  /**
   * Load cache entries from a JSON file.
   *
   * If the file doesn't exist or is malformed, we start with an empty
   * cache (no error raised -- a missing cache just means everything
   * gets rebuilt).
   *
   * @param filePath - Absolute path to the cache JSON file.
   */
  load(filePath: string): void {
    if (!fs.existsSync(filePath)) {
      this._entries = new Map();
      return;
    }

    try {
      const text = fs.readFileSync(filePath, "utf-8");
      const data = JSON.parse(text);

      this._entries = new Map();
      for (const [name, entryData] of Object.entries(data)) {
        const entry = entryData as Record<string, unknown>;
        // Validate required fields exist before adding to cache.
        if (
          typeof entry.package_hash === "string" &&
          typeof entry.deps_hash === "string" &&
          typeof entry.last_built === "string" &&
          typeof entry.status === "string"
        ) {
          this._entries.set(name, {
            packageHash: entry.package_hash,
            depsHash: entry.deps_hash,
            lastBuilt: entry.last_built,
            status: entry.status,
          });
        }
      }
    } catch {
      // JSON parse error or read error -- start fresh.
      this._entries = new Map();
    }
  }

  /**
   * Save cache entries to a JSON file with atomic write.
   *
   * Writes to a temporary file first, then renames. This prevents
   * corruption if the process is interrupted mid-write.
   *
   * The JSON format uses snake_case keys to match the Python implementation's
   * cache format, ensuring cross-tool compatibility.
   *
   * @param filePath - Absolute path to the cache JSON file.
   */
  save(filePath: string): void {
    const data: Record<
      string,
      Record<string, string>
    > = {};

    // Sort entries by name for deterministic output.
    const sortedNames = Array.from(this._entries.keys()).sort();
    for (const name of sortedNames) {
      const entry = this._entries.get(name)!;
      data[name] = {
        package_hash: entry.packageHash,
        deps_hash: entry.depsHash,
        last_built: entry.lastBuilt,
        status: entry.status,
      };
    }

    const tmpPath = path.join(
      path.dirname(filePath),
      `${path.basename(filePath)}.tmp`,
    );
    fs.writeFileSync(tmpPath, JSON.stringify(data, null, 2) + "\n", "utf-8");
    fs.renameSync(tmpPath, filePath);
  }

  /**
   * Determine if a package needs rebuilding.
   *
   * A package needs rebuilding if:
   * 1. It's not in the cache at all (never built).
   * 2. Its source hash changed (files were modified).
   * 3. Its dependency hash changed (a dependency was modified).
   * 4. Its last build failed.
   *
   * @param name - Package name (e.g., "python/logic-gates").
   * @param pkgHash - Current SHA256 of the package's source files.
   * @param depsHash - Current SHA256 of transitive dependency hashes.
   * @returns True if the package should be rebuilt.
   */
  needsBuild(name: string, pkgHash: string, depsHash: string): boolean {
    const entry = this._entries.get(name);

    // Case 1: Never built before.
    if (!entry) {
      return true;
    }

    // Case 2: Last build failed -- always retry.
    if (entry.status === "failed") {
      return true;
    }

    // Case 3: Source files changed.
    if (entry.packageHash !== pkgHash) {
      return true;
    }

    // Case 4: Dependencies changed.
    if (entry.depsHash !== depsHash) {
      return true;
    }

    // No changes detected -- skip this package.
    return false;
  }

  /**
   * Record a build result in the cache.
   *
   * @param name - Package name.
   * @param pkgHash - SHA256 of the package's source files at build time.
   * @param depsHash - SHA256 of transitive dependency hashes at build time.
   * @param status - "success" or "failed".
   */
  record(
    name: string,
    pkgHash: string,
    depsHash: string,
    status: string,
  ): void {
    this._entries.set(name, {
      packageHash: pkgHash,
      depsHash: depsHash,
      lastBuilt: new Date().toISOString(),
      status,
    });
  }

  /**
   * Read-only access to the cache entries.
   *
   * Returns a new Map to prevent external mutation of the internal state.
   */
  get entries(): Map<string, CacheEntry> {
    return new Map(this._entries);
  }
}
