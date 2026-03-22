/**
 * hasher.ts -- SHA256 File Hashing for Change Detection
 * =====================================================
 *
 * This module computes SHA256 hashes for package source files. The hash of a
 * package is a single string that changes whenever any source file in the
 * package is modified, added, or removed.
 *
 * ## How hashing works
 *
 * 1. Collect all source files in the package directory, filtered by the
 *    language's relevant extensions. Always include the BUILD file.
 * 2. Sort the file list lexicographically (by relative path) for determinism.
 * 3. SHA256-hash each file's contents individually.
 * 4. Concatenate all individual hashes into one string.
 * 5. SHA256-hash that concatenated string to produce the final package hash.
 *
 * This two-level hashing means:
 * - Reordering files doesn't change the hash (we sort first).
 * - Adding or removing a file changes the hash (the concatenated string changes).
 * - Modifying any file's contents changes the hash.
 *
 * ## Dependency hashing
 *
 * A package should be rebuilt if any of its transitive dependencies changed.
 * `hashDeps` takes a package name, the dependency graph, and the per-package
 * hashes, then produces a single hash representing the state of all dependencies.
 *
 * ## Why SHA256?
 *
 * SHA256 is a cryptographic hash function that produces a 256-bit (32-byte)
 * digest. It's fast enough for our purposes and has an astronomically low
 * collision probability -- the chance of two different files producing the
 * same hash is roughly 1 in 2^256.
 */

import * as crypto from "node:crypto";
import * as fs from "node:fs";
import * as path from "node:path";
import type { Package } from "./discovery.js";
import type { DirectedGraph } from "./resolver.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/**
 * Source file extensions that matter for each language.
 *
 * If any file with these extensions changes, the package needs rebuilding.
 * We only track extensions that contain actual source code or configuration
 * that affects the build output.
 */
export const SOURCE_EXTENSIONS: Record<string, Set<string>> = {
  python: new Set([".py", ".toml", ".cfg"]),
  ruby: new Set([".rb", ".gemspec"]),
  go: new Set([".go"]),
  rust: new Set([".rs", ".toml"]),
  typescript: new Set([".ts", ".json"]),
  elixir: new Set([".ex", ".exs"]),
};

/**
 * Special filenames to always include regardless of extension.
 *
 * Some files don't have standard extensions but are still important
 * for builds (like Makefiles or lock files).
 */
export const SPECIAL_FILENAMES: Record<string, Set<string>> = {
  python: new Set(),
  ruby: new Set(["Gemfile", "Rakefile"]),
  go: new Set(["go.mod", "go.sum"]),
  rust: new Set(["Cargo.lock"]),
  typescript: new Set(["package-lock.json"]),
  elixir: new Set(["mix.lock"]),
};

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Recursively collect all files in a directory.
 *
 * This is a simple recursive directory walker that returns all files
 * (not directories) found under the given root.
 */
function walkFiles(dir: string): string[] {
  const results: string[] = [];
  let entries: fs.Dirent[];

  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return results;
  }

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...walkFiles(fullPath));
    } else if (entry.isFile()) {
      results.push(fullPath);
    }
  }

  return results;
}

/**
 * Collect all source files in a package directory.
 *
 * Files are filtered by the language's relevant extensions and special
 * filenames. BUILD files are always included.
 *
 * @param pkg - The package to collect files for.
 * @returns A sorted list of absolute paths.
 */
export function collectSourceFiles(pkg: Package): string[] {
  const extensions = SOURCE_EXTENSIONS[pkg.language] ?? new Set<string>();
  const specialNames = SPECIAL_FILENAMES[pkg.language] ?? new Set<string>();

  const files: string[] = [];

  for (const filepath of walkFiles(pkg.path)) {
    const basename = path.basename(filepath);
    const ext = path.extname(filepath);

    // Always include BUILD files (any variant).
    if (
      basename === "BUILD" ||
      basename === "BUILD_mac" ||
      basename === "BUILD_linux" ||
      basename === "BUILD_windows" ||
      basename === "BUILD_mac_and_linux"
    ) {
      files.push(filepath);
      continue;
    }

    // Check extension.
    if (extensions.has(ext)) {
      files.push(filepath);
      continue;
    }

    // Check special filenames.
    if (specialNames.has(basename)) {
      files.push(filepath);
      continue;
    }
  }

  // Sort by relative path for determinism.
  files.sort((a, b) => {
    const relA = path.relative(pkg.path, a);
    const relB = path.relative(pkg.path, b);
    return relA.localeCompare(relB);
  });

  return files;
}

/**
 * Compute the SHA256 hex digest of a single file's contents.
 *
 * Reads the file in one go and returns the hex-encoded hash.
 * For very large files, a streaming approach would be better, but
 * source files are typically small enough that this is fine.
 */
export function hashFile(filepath: string): string {
  const content = fs.readFileSync(filepath);
  return crypto.createHash("sha256").update(content).digest("hex");
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Compute a SHA256 hash representing all source files in the package.
 *
 * The hash changes if any source file is added, removed, or modified.
 *
 * @param pkg - The package to hash.
 * @returns A hex-encoded SHA256 hash string.
 */
export function hashPackage(pkg: Package): string {
  const files = collectSourceFiles(pkg);

  if (files.length === 0) {
    // No source files -- hash the empty string for consistency.
    return crypto.createHash("sha256").update("").digest("hex");
  }

  // Hash each file, concatenate, hash again.
  const fileHashes = files.map((f) => hashFile(f));
  const combined = fileHashes.join("");
  return crypto.createHash("sha256").update(combined).digest("hex");
}

/**
 * Compute a SHA256 hash of all transitive dependency hashes.
 *
 * If any transitive dependency's source files changed, this hash will
 * change too, triggering a rebuild of the dependent package.
 *
 * In our graph, edges go dep -> pkg (dependency points to dependent),
 * so a package's dependencies are found by walking the reverse direction
 * (transitiveDependents).
 *
 * @param packageName - The package whose dependencies we're hashing.
 * @param graph - The dependency graph.
 * @param packageHashes - Mapping from package name to its source hash.
 * @returns A hex-encoded SHA256 hash string.
 */
export function hashDeps(
  packageName: string,
  graph: DirectedGraph,
  packageHashes: Map<string, string>,
): string {
  if (!graph.hasNode(packageName)) {
    return crypto.createHash("sha256").update("").digest("hex");
  }

  const transitiveDeps = graph.transitiveDependents(packageName);

  if (transitiveDeps.size === 0) {
    return crypto.createHash("sha256").update("").digest("hex");
  }

  // Sort dependency names for determinism, concatenate their hashes.
  const sortedDeps = Array.from(transitiveDeps).sort();
  const combined = sortedDeps
    .map((dep) => packageHashes.get(dep) ?? "")
    .join("");
  return crypto.createHash("sha256").update(combined).digest("hex");
}
