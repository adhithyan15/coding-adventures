/**
 * discovery.ts -- Package Discovery via Recursive BUILD File Walk
 * ================================================================
 *
 * This module walks a monorepo directory tree to discover packages. A "package"
 * is any directory that contains a BUILD file. The walk is recursive: starting
 * from the root, we list all subdirectories and descend into each one, skipping
 * known non-source directories (.git, .venv, node_modules, etc.).
 *
 * When we find a BUILD file in a directory, we stop recursing there and register
 * that directory as a package. This is the same approach used by Bazel, Buck,
 * and Pants -- no configuration files are needed to route the walk.
 *
 * ## Platform-specific BUILD files
 *
 * The build tool supports platform-specific BUILD files so that packages can
 * define different build commands for different operating systems. The priority
 * order is (most specific wins):
 *
 * - **darwin** (macOS):  BUILD_mac -> BUILD_mac_and_linux -> BUILD
 * - **linux**:           BUILD_linux -> BUILD_mac_and_linux -> BUILD
 * - **win32** (Windows): BUILD_windows -> BUILD
 *
 * This layering lets packages provide Windows-specific build commands via
 * BUILD_windows while sharing a single BUILD_mac_and_linux for the common
 * Unix case, falling back to BUILD when no platform differences exist.
 *
 * Note: Node.js `os.platform()` returns "win32" on Windows, not "windows".
 * We handle both for robustness.
 *
 * ## Language inference
 *
 * We infer the language from the directory path. If the path contains
 * "packages/python/X" or "programs/python/X", the language is "python".
 * Similarly for "ruby", "go", "rust", "typescript", and "elixir". The
 * package name is "{language}/{dir-name}".
 */

import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/**
 * Directories that should never be traversed during package discovery.
 *
 * These are known to contain non-source files (caches, dependencies, build
 * artifacts) that would waste time to scan. Every build tool, linter, and
 * IDE has a similar skip list -- this is ours.
 */
export const SKIP_DIRS: ReadonlySet<string> = new Set([
  ".git",
  ".hg",
  ".svn",
  ".venv",
  ".tox",
  ".mypy_cache",
  ".pytest_cache",
  ".ruff_cache",
  "__pycache__",
  "node_modules",
  "vendor",
  "dist",
  "build",
  "target",
  ".claude",
  "Pods",
]);

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Represents a discovered package in the monorepo.
 *
 * Think of this as a "build target" -- it's a directory with source code
 * and a BUILD file that tells us how to build it.
 *
 * @property name - A qualified name like "python/logic-gates" or "ruby/arithmetic".
 *                  The format is always "{language}/{directory-name}".
 * @property path - Absolute path to the package directory on disk.
 * @property buildCommands - Lines from the BUILD file (commands to execute).
 *                           Blank lines and comments are stripped out.
 * @property language - Inferred language: "python", "ruby", "go", "rust",
 *                      "typescript", "elixir", or "unknown".
 */
export interface Package {
  name: string;
  path: string;
  buildCommands: string[];
  language: string;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Read a file and return non-blank, non-comment lines.
 *
 * Blank lines and lines starting with '#' are stripped out. Leading and
 * trailing whitespace is removed from each line. This is the standard
 * format for BUILD files in this monorepo.
 *
 * @param filepath - Absolute path to the file to read.
 * @returns Array of non-blank, non-comment lines.
 */
export function readLines(filepath: string): string[] {
  if (!fs.existsSync(filepath)) {
    return [];
  }

  const text = fs.readFileSync(filepath, "utf-8");
  const lines: string[] = [];

  for (const line of text.split("\n")) {
    const stripped = line.trim();
    if (stripped && !stripped.startsWith("#")) {
      lines.push(stripped);
    }
  }

  return lines;
}

/**
 * Infer the programming language from the directory path.
 *
 * We look for known language directory names in the path components.
 * The pattern we look for is a parent directory named "python", "ruby",
 * "go", "rust", "typescript", or "elixir" that sits under "packages" or
 * "programs".
 *
 * For example:
 * - "/repo/code/packages/python/logic-gates" -> "python"
 * - "/repo/code/programs/go/build-tool" -> "go"
 * - "/repo/code/packages/rust/arithmetic" -> "rust"
 *
 * @param dirPath - Absolute path to the directory.
 * @returns The inferred language string, or "unknown".
 */
export function inferLanguage(dirPath: string): string {
  // Split the path into its component parts (directories).
  // On Unix: "/a/b/c" -> ["", "a", "b", "c"]
  // On Windows: "C:\\a\\b" -> ["C:", "a", "b"]
  // Split on both forward slash and backslash so this works on Windows
  // (where path.sep is "\") even when paths contain "/" (e.g., from
  // cross-platform plan files or test fixtures).
  const parts = dirPath.split(/[/\\]/);

  const knownLanguages = [
    "python",
    "ruby",
    "go",
    "rust",
    "typescript",
    "elixir",
    "lua",
    "perl",
    "swift",
    "haskell",
    "wasm",
    "csharp",
    "fsharp",
    "dotnet",
  ];
  for (const lang of knownLanguages) {
    if (parts.includes(lang)) {
      return lang;
    }
  }

  return "unknown";
}

/**
 * Build a qualified package name like "python/logic-gates".
 *
 * Uses the language and the directory's basename. This naming convention
 * is consistent across all build tool implementations (Python, Ruby, Go,
 * Rust, Elixir, and now TypeScript).
 *
 * @param dirPath - Absolute path to the package directory.
 * @param language - The inferred language.
 * @returns A qualified name string.
 */
export function inferPackageName(dirPath: string, language: string): string {
  return `${language}/${path.basename(dirPath)}`;
}

/**
 * Return the appropriate BUILD file path for the current platform.
 *
 * This implements the platform-specific BUILD file priority system. The
 * idea is that most packages have a single BUILD file, but when a package
 * needs different commands on different operating systems (e.g., different
 * compiler flags on macOS vs Linux), it can provide platform-specific
 * variants.
 *
 * Priority (most specific wins):
 *
 * 1. Platform-specific: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows)
 * 2. Shared Unix: BUILD_mac_and_linux (macOS or Linux -- for the common case)
 * 3. Generic: BUILD (all platforms)
 * 4. null if no BUILD file exists
 *
 * @param directory - Absolute path to the directory to check.
 * @param platformOverride - Optional platform string for testing. If not
 *                           provided, uses os.platform().
 * @returns Absolute path to the BUILD file, or null if none exists.
 */
export function getBuildFile(
  directory: string,
  platformOverride?: string,
): string | null {
  // Node.js os.platform() returns "darwin" for macOS, "linux" for Linux,
  // and "win32" for Windows (not "windows"!).
  const platform = platformOverride ?? os.platform();

  // Step 1: Check for the most specific platform file.
  //
  // This is a direct lookup -- if the exact platform BUILD file exists,
  // it takes highest priority.
  if (platform === "darwin") {
    const macBuild = path.join(directory, "BUILD_mac");
    if (fs.existsSync(macBuild)) {
      return macBuild;
    }
  }

  if (platform === "linux") {
    const linuxBuild = path.join(directory, "BUILD_linux");
    if (fs.existsSync(linuxBuild)) {
      return linuxBuild;
    }
  }

  if (platform === "win32") {
    const windowsBuild = path.join(directory, "BUILD_windows");
    if (fs.existsSync(windowsBuild)) {
      return windowsBuild;
    }
  }

  // Step 2: Check for the shared Unix file (macOS + Linux).
  //
  // Many packages have identical build commands on macOS and Linux but
  // different ones on Windows. BUILD_mac_and_linux covers the common case
  // without duplicating a BUILD_mac and BUILD_linux.
  if (platform === "darwin" || platform === "linux") {
    const sharedBuild = path.join(directory, "BUILD_mac_and_linux");
    if (fs.existsSync(sharedBuild)) {
      return sharedBuild;
    }
  }

  // Step 3: Fall back to the generic BUILD file.
  //
  // This is the default -- if no platform-specific file exists, use the
  // generic one.
  const genericBuild = path.join(directory, "BUILD");
  if (fs.existsSync(genericBuild)) {
    return genericBuild;
  }

  // Step 4: No BUILD file found at all.
  return null;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Recursively walk the directory tree, collect packages with BUILD files.
 *
 * Starting from `root`, we list all subdirectories and descend into each
 * one (skipping directories in the skip list). When we find a BUILD file,
 * we register that directory as a package and stop recursing into it.
 *
 * This is the same "walk until you find a BUILD file" approach used by
 * Bazel, Buck, and Pants. It means:
 * - No central configuration file listing all packages
 * - Adding a new package = adding a BUILD file
 * - Nested packages are supported (a BUILD file stops descent)
 *
 * @param root - The monorepo root directory (typically the "code/" directory).
 * @param platformOverride - Optional platform string for testing.
 * @returns A list of discovered Package objects, sorted by name.
 */
export function discoverPackages(
  root: string,
  platformOverride?: string,
): Package[] {
  const packages: Package[] = [];
  walkDirs(root, packages, platformOverride);

  // Sort by name for deterministic output. This ensures that the build
  // order is the same regardless of filesystem ordering (which can vary
  // between operating systems and filesystems).
  packages.sort((a, b) => a.name.localeCompare(b.name));

  return packages;
}

/**
 * Recursively walk directories and collect packages.
 *
 * This is the recursive workhorse. For each directory:
 * 1. If its name is in the skip list, ignore it entirely.
 * 2. If it has a BUILD file, register it as a package and stop recursing.
 * 3. Otherwise, list subdirectories and recurse into each one.
 *
 * @param directory - Current directory being examined.
 * @param packages - Accumulator array (mutated in place).
 * @param platformOverride - Optional platform string for testing.
 */
function walkDirs(
  directory: string,
  packages: Package[],
  platformOverride?: string,
): void {
  // Skip known non-source directories.
  if (SKIP_DIRS.has(path.basename(directory))) {
    return;
  }

  const buildFile = getBuildFile(directory, platformOverride);

  if (buildFile !== null) {
    // This directory is a package! Read the BUILD commands.
    const commands = readLines(buildFile);
    const language = inferLanguage(directory);
    const name = inferPackageName(directory, language);

    packages.push({
      name,
      path: directory,
      buildCommands: commands,
      language,
    });
    return;
  }

  // Not a package -- list subdirectories and recurse into each one.
  let entries: fs.Dirent[];
  try {
    entries = fs.readdirSync(directory, { withFileTypes: true });
  } catch {
    // Permission denied or other read error -- skip silently.
    return;
  }

  // Sort entries for deterministic traversal order.
  entries.sort((a, b) => a.name.localeCompare(b.name));

  for (const entry of entries) {
    if (entry.isDirectory()) {
      walkDirs(path.join(directory, entry.name), packages, platformOverride);
    }
  }
}
