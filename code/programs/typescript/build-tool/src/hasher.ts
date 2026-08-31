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
 * 2. Normalize repository-relative paths to forward-slash form and sort them.
 * 3. Frame each UTF-8 path with its unsigned 64-bit byte length.
 * 4. Append each file's unsigned 64-bit content length and exact raw bytes.
 * 5. SHA256-hash the unambiguous stream to produce the final package hash.
 *
 * This framed hashing means:
 * - Reordering files doesn't change the hash (we sort first).
 * - Adding or removing a file changes the hash (the framed stream changes).
 * - Modifying any file's contents changes the hash.
 * - Renaming a file changes the hash, even when its contents do not.
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
import { matchPath } from "./glob-match.js";

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
  perl: new Set([".pl", ".pm", ".t", ".xs"]),
  haskell: new Set([".hs", ".cabal"]),
  ocaml: new Set([".ml", ".mli", ".opam"]),
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
  perl: new Set([
    "Makefile.PL",
    "Build.PL",
    "cpanfile",
    "MANIFEST",
    "META.json",
    "META.yml",
  ]),
  haskell: new Set(),
  ocaml: new Set([".ocamlformat", "dune", "dune-project"]),
};

/**
 * Manifest extensions that affect a package independently of declared globs.
 *
 * OCaml package manifests live at the package root. Source extensions such as
 * `.ml` and `.mli` remain governed by the caller's declared source patterns.
 */
const DECLARED_MANIFEST_EXTENSIONS: Record<string, Set<string>> = {
  ocaml: new Set([".opam"]),
};

/**
 * Exact directory components that never contain package source.
 *
 * This registry belongs to source hashing rather than package discovery. A
 * discovered package may legitimately contain a directory named `specs`, for
 * example, while generated output beneath `_build` must never invalidate that
 * package. Keeping the list here prevents the two policies from drifting into
 * one over-broad skip set.
 *
 * Membership is deliberately case-sensitive and component-wise. `_build` is
 * generated output; `_Build` and `_build-example` remain ordinary source
 * directories. Testing `Dirent.name` before recursion also means we never need
 * to open or resolve anything below an excluded component.
 */
const SOURCE_HASH_EXCLUDED_DIRECTORIES = new Set([
  ".git",
  ".hg",
  ".svn",
  ".venv",
  ".tox",
  ".mypy_cache",
  ".pytest_cache",
  ".ruff_cache",
  ".stack-work",
  "__pycache__",
  "node_modules",
  "vendor",
  "dist",
  "dist-newstyle",
  "_build",
  "build",
  "target",
  ".claude",
  "Pods",
  ".gradle",
  ".dart_tool",
  "gradle-build",
  "deps",
  ".build",
  ".cargo",
  "cover",
]);

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
      if (SOURCE_HASH_EXCLUDED_DIRECTORIES.has(entry.name)) continue;
      results.push(...walkFiles(fullPath));
    } else if (entry.isFile()) {
      results.push(fullPath);
    }
  }

  return results;
}

/** Return a package-local path using the contract's portable separator. */
function portableRelativePath(root: string, filepath: string): string {
  return path.relative(root, filepath).split(path.sep).join("/");
}

/** Compare portable paths by their UTF-8 bytes, independent of host locale. */
function comparePortablePaths(left: string, right: string): number {
  return Buffer.compare(
    Buffer.from(left, "utf-8"),
    Buffer.from(right, "utf-8"),
  );
}

/**
 * Derive the package root's normalized repository-relative path.
 *
 * Production packages live below `code/packages` or `code/programs`. The
 * identity fallback keeps isolated unit fixtures deterministic without
 * incorporating an absolute checkout prefix into their digest.
 */
function repositoryRelativePackagePath(pkg: Package): string {
  const parts = path.resolve(pkg.path).split(/[\\/]+/u);
  for (let index = parts.length - 3; index >= 0; index -= 1) {
    if (
      parts[index] === "code" &&
      (parts[index + 1] === "packages" || parts[index + 1] === "programs")
    ) {
      return parts.slice(index).join("/");
    }
  }

  const identity = pkg.name.split("/");
  if (identity.length === 3 && identity[1] === "programs") {
    return `code/programs/${identity[0]}/${identity[2]}`;
  }
  if (identity.length === 2) {
    return `code/packages/${identity[0]}/${identity[1]}`;
  }
  throw new Error("cannot derive repository-relative package path");
}

/** Append one unsigned 64-bit big-endian length to a SHA-256 stream. */
function updateUnsigned64(hash: crypto.Hash, value: number): void {
  const encoded = Buffer.alloc(8);
  encoded.writeBigUInt64BE(BigInt(value));
  hash.update(encoded);
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
  files.sort((a, b) =>
    comparePortablePaths(
      portableRelativePath(pkg.path, a),
      portableRelativePath(pkg.path, b),
    ),
  );

  return files;
}

/**
 * Collect source files using declared glob patterns from a Starlark BUILD file.
 *
 * When a package declares explicit `srcs` patterns (e.g., "src/foo.py",
 * "tests/*.test.ts"), we use those patterns to filter the file tree instead
 * of relying on language-based extension matching.
 *
 * This fixes a subtle bug with the extension-based approach: it could miss
 * files that are important to the build but have unusual extensions, and it
 * could include files that the build doesn't actually use.
 *
 * The glob patterns are matched using the pure-string `matchPath()` function
 * from glob-match.ts, which correctly handles `*`, `?`, and multi-segment
 * wildcard patterns.
 *
 * BUILD files are always included regardless of the patterns, because a
 * change to the BUILD file itself should always trigger a rebuild.
 *
 * @param pkg - The package to collect files for.
 * @param patterns - Glob patterns relative to the package directory
 *                   (e.g., ["src/foo.py", "tests/*.test.ts"]).
 * @returns A sorted list of absolute paths.
 */
export function collectSourceFilesGlob(
  pkg: Package,
  patterns: string[],
): string[] {
  const files: string[] = [];
  const specialNames = SPECIAL_FILENAMES[pkg.language] ?? new Set<string>();
  const manifestExtensions =
    DECLARED_MANIFEST_EXTENSIONS[pkg.language] ?? new Set<string>();
  const packageRoot = path.resolve(pkg.path);

  for (const filepath of walkFiles(pkg.path)) {
    const basename = path.basename(filepath);

    // Always include BUILD files.
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

    // Exact package metadata remains a hashing input even when the declared
    // source patterns omit it. Extension manifests are root-scoped so nested
    // dependency/example metadata does not silently widen the target.
    if (
      specialNames.has(basename) ||
      (path.resolve(path.dirname(filepath)) === packageRoot &&
        manifestExtensions.has(path.extname(filepath)))
    ) {
      files.push(filepath);
      continue;
    }

    // Compute the path relative to the package directory and match
    // against each declared source pattern.
    //
    // We use forward slashes for consistency, since glob patterns
    // always use forward slashes regardless of platform.
    const relPath = portableRelativePath(pkg.path, filepath);

    for (const pattern of patterns) {
      if (matchPath(pattern, relPath)) {
        files.push(filepath);
        break; // No need to check more patterns once we have a match.
      }
    }
  }

  // Sort by relative path for determinism (same as collectSourceFiles).
  files.sort((a, b) =>
    comparePortablePaths(
      portableRelativePath(pkg.path, a),
      portableRelativePath(pkg.path, b),
    ),
  );

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

  // Hashing v1 frames every normalized repository-relative UTF-8 path and
  // exact raw content with unsigned 64-bit byte lengths. File identity and
  // boundaries are therefore unambiguous without hashing absolute checkout
  // locations, decoded text, or host metadata.
  const packageHash = crypto.createHash("sha256");
  const packageRoot = repositoryRelativePackagePath(pkg);
  for (const filepath of files) {
    const portablePath = `${packageRoot}/${portableRelativePath(pkg.path, filepath)}`;
    const pathBytes = Buffer.from(portablePath, "utf-8");
    const content = fs.readFileSync(filepath);
    updateUnsigned64(packageHash, pathBytes.length);
    packageHash.update(pathBytes);
    updateUnsigned64(packageHash, content.length);
    packageHash.update(content);
  }
  return packageHash.digest("hex");
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
