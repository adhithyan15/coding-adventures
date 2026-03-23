/**
 * gitdiff.ts -- Git-based Change Detection
 * ==========================================
 *
 * This module determines which packages changed by comparing the current
 * branch against a base branch (typically `origin/main`) using git.
 *
 * This is the DEFAULT change detection mechanism. It replaces the cache-based
 * approach with a stateless one -- git itself is the source of truth. No
 * cache file needed.
 *
 * ## How it works
 *
 * 1. Run `git diff --name-only <base>...HEAD`
 *    This gives us every file that changed between the base and HEAD.
 *    The three-dot syntax means "changes since the merge base", which is
 *    exactly what we want for PR builds.
 *
 * 2. Map each changed file to a package by matching its path prefix
 *    against discovered package paths.
 *    e.g., "code/packages/python/logic-gates/src/gates.py"
 *          -> package "python/logic-gates"
 *
 * 3. Use the directed graph's `affectedNodes()` to find all packages
 *    that transitively depend on the changed packages.
 *
 * 4. Return the full set of affected packages to build.
 *
 * ## Why git-diff is better than hash-based caching
 *
 * The beauty of this approach: no state to manage, no cache file to commit,
 * and it works perfectly with CI (every PR naturally has a base branch to
 * diff against).
 */

import { execSync } from "node:child_process";
import * as path from "node:path";
import { matchPath } from "./glob-match.js";

/**
 * Get the list of files changed between diffBase and HEAD.
 *
 * Uses `git diff --name-only <base>...HEAD` which shows files changed
 * on the current branch since it diverged from the base. The three-dot
 * syntax means "changes since the merge base", which is exactly what
 * we want for PR builds.
 *
 * For pushes to main, use `HEAD~1` as the base to compare against
 * the previous commit.
 *
 * If the three-dot diff fails (e.g., shallow clone or missing remote ref),
 * we fall back to a two-dot diff.
 *
 * @param repoRoot - The repository root directory (contains .git).
 * @param diffBase - The git ref to compare against (default: "origin/main").
 * @returns List of changed file paths relative to repoRoot. Empty if diff fails.
 */
export function getChangedFiles(
  repoRoot: string,
  diffBase: string = "origin/main",
): string[] {
  try {
    // Try three-dot diff first (preferred for PR builds).
    const result = execSync(
      `git diff --name-only ${diffBase}...HEAD`,
      {
        cwd: repoRoot,
        encoding: "utf-8",
        timeout: 30000,
        stdio: ["pipe", "pipe", "pipe"],
      },
    );

    return result
      .trim()
      .split("\n")
      .filter((line) => line.trim().length > 0);
  } catch {
    // Three-dot failed -- try two-dot fallback.
    try {
      const result = execSync(
        `git diff --name-only ${diffBase} HEAD`,
        {
          cwd: repoRoot,
          encoding: "utf-8",
          timeout: 30000,
          stdio: ["pipe", "pipe", "pipe"],
        },
      );

      return result
        .trim()
        .split("\n")
        .filter((line) => line.trim().length > 0);
    } catch {
      // Both failed -- return empty list.
      return [];
    }
  }
}

/**
 * Map changed file paths to package names.
 *
 * For each changed file, check which package directory it falls under.
 * A file belongs to a package if its path starts with the package's
 * directory path (relative to repo root).
 *
 * ## Strict Starlark Filtering
 *
 * When a package has declared source patterns (from a Starlark BUILD file),
 * we apply an extra filter: a changed file only counts if it matches at
 * least one of the declared `srcs` glob patterns. This prevents spurious
 * rebuilds when non-source files (like documentation or editor configs)
 * change inside a package directory.
 *
 * For plain-shell BUILD files (no declared srcs), any changed file under
 * the package directory triggers a rebuild -- the old behavior.
 *
 * @param changedFiles - File paths relative to repo root.
 * @param packagePaths - Mapping of package name -> absolute package directory.
 * @param repoRoot - The repository root directory.
 * @param declaredSrcs - Optional mapping of package name -> list of glob
 *                       patterns from the Starlark BUILD file's `srcs` field.
 *                       If provided for a package, only files matching at
 *                       least one pattern count as "changed".
 * @returns Set of package names that contain at least one changed file.
 *
 * @example
 * ```
 * changedFiles = ["code/packages/python/logic-gates/src/gates.py"]
 * packagePaths = new Map([["python/logic-gates", "/repo/code/packages/python/logic-gates"]])
 * // Returns: Set(["python/logic-gates"])
 * ```
 */
export function mapFilesToPackages(
  changedFiles: string[],
  packagePaths: Map<string, string>,
  repoRoot: string,
  declaredSrcs?: Map<string, string[]>,
): Set<string> {
  const changed = new Set<string>();

  // Convert package paths to relative strings for prefix matching.
  const relativePkgPaths = new Map<string, string>();
  for (const [name, absPath] of packagePaths) {
    const relPath = path.relative(repoRoot, absPath);
    relativePkgPaths.set(name, relPath);
  }

  for (const filepath of changedFiles) {
    for (const [pkgName, pkgRelPath] of relativePkgPaths) {
      // Step 1: Check if the file falls under this package's directory.
      const isUnderPackage =
        filepath.startsWith(pkgRelPath + "/") || filepath === pkgRelPath;

      if (!isUnderPackage) {
        continue;
      }

      // Step 2: If the package has declared srcs, apply strict filtering.
      //
      // The declared srcs patterns are relative to the package directory,
      // so we need the file path relative to the package directory too.
      const patterns = declaredSrcs?.get(pkgName);
      if (patterns && patterns.length > 0) {
        const relToPackage = filepath.slice(pkgRelPath.length + 1);

        // BUILD files always count -- they define the build itself.
        const basename = relToPackage.split("/").pop() ?? "";
        const isBuildFile =
          basename === "BUILD" ||
          basename === "BUILD_mac" ||
          basename === "BUILD_linux" ||
          basename === "BUILD_windows" ||
          basename === "BUILD_mac_and_linux";

        if (!isBuildFile) {
          // Check if the file matches any declared source pattern.
          const matchesAny = patterns.some((pat) =>
            matchPath(pat, relToPackage),
          );
          if (!matchesAny) {
            continue; // File changed but doesn't match any declared src.
          }
        }
      }

      changed.add(pkgName);
      break; // A file belongs to at most one package.
    }
  }

  return changed;
}
