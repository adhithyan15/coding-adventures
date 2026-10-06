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
 * 1. Collect package-local inputs through the checked language registry.
 *    Universal BUILD fronts and root capability metadata are always inputs.
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
import { compilePatterns, matchCompiledPath } from "./glob-match.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** The installed JSON snapshot is the executable's sole selector authority. */
type ScopedInput = {
  scope: "root" | "subtree";
  path_prefix?: string;
  suffixes: string[];
  exact_basenames: string[];
};
type PackageExactInput = { package_root: string; paths: string[] };
type LanguageInput = {
  language: string;
  recursive_suffixes: string[];
  recursive_exact_basenames: string[];
  root_exact_basenames: string[];
  root_variable_suffixes: string[];
  root_exact_relative_paths: string[];
  package_exact_inputs: PackageExactInput[];
  scoped_inputs: ScopedInput[];
};
type SourceInputRegistry = {
  schema_version: number;
  universal_inputs: {
    build_filenames: string[];
    generated_directory_components: string[];
    root_exact_basenames: string[];
  };
  languages: LanguageInput[];
};

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === "object") {
    for (const nested of Object.values(value)) deepFreeze(nested);
    Object.freeze(value);
  }
  return value;
}

const SOURCE_INPUT_REGISTRY = deepFreeze(JSON.parse(
  fs.readFileSync(new URL("./language-source-input-registry.json", import.meta.url), "utf-8"),
) as SourceInputRegistry);
const LANGUAGE_INPUTS = new Map(
  SOURCE_INPUT_REGISTRY.languages.map((entry) => [entry.language, entry]),
);
const BUILD_FILENAMES = new Set(SOURCE_INPUT_REGISTRY.universal_inputs.build_filenames);

/** Expose the actual runtime projection for complete checked-fixture equality. */
export function sourceInputRegistry(): SourceInputRegistry {
  return SOURCE_INPUT_REGISTRY;
}

/** Canonical JSON sorts object keys; registry arrays retain their reviewed order. */
function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return `{${Object.keys(record).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(record[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export function sourceInputRegistryDigest(): string {
  const canonical = Buffer.from(canonicalJson(SOURCE_INPUT_REGISTRY), "utf-8");
  const domain = Buffer.from("coding-adventures/build-tool-language-source-input-registry/v1\0", "ascii");
  const size = Buffer.alloc(8);
  size.writeBigUInt64BE(BigInt(canonical.length));
  return crypto.createHash("sha256").update(domain).update(size).update(canonical).digest("hex");
}

function languageInputs(language: string): LanguageInput {
  const entry = LANGUAGE_INPUTS.get(language);
  if (!entry) throw new Error(`unknown source language: ${language}`);
  return entry;
}

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
const SOURCE_HASH_EXCLUDED_DIRECTORIES = new Set(
  SOURCE_INPUT_REGISTRY.universal_inputs.generated_directory_components,
);

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

/**
 * Fixed package paths are never granted by a name-only fixture fallback.
 * A real conventional root must agree with the caller's language; site roots
 * are limited to the two exact reviewed TypeScript registrations.
 */
function packageExactPaths(pkg: Package, entry: LanguageInput): Set<string> {
  const parts = path.resolve(pkg.path).split(/[\\/]+/u);
  for (let index = parts.length - 2; index >= 0; index -= 1) {
    if (parts[index] !== "code") continue;
    const section = parts[index + 1];
    const root = parts.slice(index).join("/");
    if (section === "packages" || section === "programs") {
      if (parts.length < index + 4 || parts[index + 2] !== pkg.language) return new Set();
    } else if (section === "sites") {
      if (pkg.language !== "typescript" || parts.length !== index + 3) return new Set();
      if (!entry.package_exact_inputs.some((rule) => rule.package_root === root)) return new Set();
    } else {
      return new Set();
    }
    return new Set(entry.package_exact_inputs
      .filter((rule) => rule.package_root === root)
      .flatMap((rule) => rule.paths));
  }
  return new Set();
}

/** Resolve all seven registry roles without treating a scoped selector as global. */
function registryInput(
  relative: string,
  entry: LanguageInput,
  exactPaths: ReadonlySet<string>,
  declared: boolean,
): boolean {
  const basename = relative.split("/").at(-1)!;
  const root = !relative.includes("/");
  if (BUILD_FILENAMES.has(basename)) return true;
  if (root && SOURCE_INPUT_REGISTRY.universal_inputs.root_exact_basenames.includes(basename)) return true;
  if (root && entry.root_exact_basenames.includes(basename)) return true;
  if (root && entry.root_variable_suffixes.some((suffix) => basename.endsWith(suffix))) return true;
  if (entry.root_exact_relative_paths.includes(relative) || exactPaths.has(relative)) return true;
  if (declared) return false;
  if (entry.recursive_suffixes.some((suffix) => basename.endsWith(suffix))) return true;
  if (entry.recursive_exact_basenames.includes(basename)) return true;
  return entry.scoped_inputs.some((rule) => {
    const inScope = rule.scope === "root"
      ? root
      : relative.startsWith(`${rule.path_prefix}/`);
    return inScope && (
      rule.suffixes.some((suffix) => basename.endsWith(suffix)) ||
      rule.exact_basenames.includes(basename)
    );
  });
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
 * In extension mode the checked registry's recursive, root, fixed, and scoped
 * roles select inputs. Exact generated components are pruned before matching.
 *
 * @param pkg - The package to collect files for.
 * @returns A sorted list of absolute paths.
 */
export function collectSourceFiles(pkg: Package): string[] {
  const entry = languageInputs(pkg.language);
  const exactPaths = packageExactPaths(pkg, entry);
  const files: string[] = [];

  for (const filepath of walkFiles(pkg.path)) {
    const relative = portableRelativePath(pkg.path, filepath);
    if (registryInput(relative, entry, exactPaths, false)) {
      files.push(filepath);
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
 * The checked registry's universal and fixed inputs remain selected regardless
 * of patterns. Recursive and scoped roles yield to the declared globs.
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
  // Validate the complete declaration before walking the filesystem. A bad
  // later pattern must not be hidden by an earlier match or an empty tree.
  const compiledPatterns = compilePatterns(patterns);
  const entry = languageInputs(pkg.language);
  const exactPaths = packageExactPaths(pkg, entry);
  const files: string[] = [];

  for (const filepath of walkFiles(pkg.path)) {
    const relPath = portableRelativePath(pkg.path, filepath);
    if (
      registryInput(relPath, entry, exactPaths, true) ||
      compiledPatterns.some((pattern) => matchCompiledPath(pattern, relPath))
    ) {
      files.push(filepath);
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
