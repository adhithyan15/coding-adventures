/**
 * executor.ts -- Parallel Build Execution
 * ========================================
 *
 * This module runs BUILD commands for packages that need rebuilding. It
 * respects the dependency graph by building packages in topological levels:
 * packages in the same level have no dependencies on each other and can
 * run in parallel.
 *
 * ## Execution strategy
 *
 * 1. Get the `independentGroups()` from the dependency graph -- these are
 *    the parallel levels.
 *
 * 2. For each level, run all packages in that level concurrently using
 *    Promise.all (Node.js is single-threaded but the child processes run
 *    in parallel via the OS).
 *
 * 3. For each package, execute its BUILD commands sequentially via
 *    `child_process.execSync`, with `cwd` set to the package directory.
 *
 * 4. If a package fails (any command returns non-zero), mark all transitive
 *    dependents as "dep-skipped" -- there's no point building them.
 *
 * ## Build results
 *
 * Each package gets a `BuildResult` with its status, stdout/stderr output,
 * and wall-clock duration.
 *
 * ## Why Promise.all instead of a thread pool?
 *
 * Node.js runs JavaScript in a single thread, but child_process.exec()
 * spawns actual OS processes that run in parallel. Promise.all() lets us
 * start multiple child processes simultaneously and wait for all of them.
 * The `maxJobs` parameter limits concurrency by splitting the level into
 * batches.
 */

import { exec } from "node:child_process";
import type { Package } from "./discovery.js";
import type { BuildCache } from "./cache.js";
import type { DirectedGraph } from "./resolver.js";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * The result of building a single package.
 *
 * Every package that goes through the build pipeline gets one of these,
 * regardless of whether it was actually built, skipped, or failed.
 */
export interface BuildResult {
  /** The package's qualified name (e.g., "python/logic-gates"). */
  packageName: string;

  /**
   * Build status:
   * - "built": Successfully built.
   * - "failed": Build command returned non-zero exit code.
   * - "skipped": No changes detected, build not needed.
   * - "dep-skipped": A dependency failed, so this was skipped.
   * - "would-build": Dry run -- would have been built.
   */
  status: "built" | "failed" | "skipped" | "dep-skipped" | "would-build";

  /** Wall-clock seconds spent building (0 for skipped packages). */
  duration: number;

  /** Combined stdout from all BUILD commands. */
  stdout: string;

  /** Combined stderr from all BUILD commands. */
  stderr: string;

  /** Exit code of the last failing command, or 0 for success. */
  returnCode: number;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Execute a single shell command and return its result.
 *
 * Wraps Node's child_process.exec() in a Promise for async/await usage.
 * The command runs in a shell (like bash), with the working directory
 * set to the package directory.
 */
function execCommand(
  command: string,
  cwd: string,
): Promise<{ stdout: string; stderr: string; returnCode: number }> {
  return new Promise((resolve) => {
    exec(
      command,
      {
        cwd,
        timeout: 600000, // 10-minute timeout per command
        maxBuffer: 10 * 1024 * 1024, // 10MB buffer
      },
      (error, stdout, stderr) => {
        if (error) {
          // Node's exec() puts the error code on the error object.
          const code =
            "code" in error && typeof error.code === "number"
              ? error.code
              : 1;
          resolve({
            stdout: stdout ?? "",
            stderr: stderr ?? "",
            returnCode: code,
          });
        } else {
          resolve({ stdout, stderr, returnCode: 0 });
        }
      },
    );
  });
}

/**
 * Execute all BUILD commands for a single package.
 *
 * Commands are run sequentially. If any command fails, we stop and
 * return a "failed" result. All commands run with `cwd` set to the
 * package directory and inherit the current environment.
 */
async function runPackageBuild(pkg: Package): Promise<BuildResult> {
  const start = performance.now();
  const allStdout: string[] = [];
  const allStderr: string[] = [];

  for (const command of pkg.buildCommands) {
    const result = await execCommand(command, pkg.path);
    allStdout.push(result.stdout);
    allStderr.push(result.stderr);

    if (result.returnCode !== 0) {
      const elapsed = (performance.now() - start) / 1000;
      return {
        packageName: pkg.name,
        status: "failed",
        duration: elapsed,
        stdout: allStdout.join(""),
        stderr: allStderr.join(""),
        returnCode: result.returnCode,
      };
    }
  }

  const elapsed = (performance.now() - start) / 1000;
  return {
    packageName: pkg.name,
    status: "built",
    duration: elapsed,
    stdout: allStdout.join(""),
    stderr: allStderr.join(""),
    returnCode: 0,
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Execute BUILD commands for packages, respecting dependency order.
 *
 * Uses `independentGroups()` from the dependency graph to determine
 * which packages can run in parallel. For each level, packages are built
 * concurrently using Promise.all.
 *
 * If a package fails, all its transitive dependents are marked as
 * "dep-skipped".
 *
 * @param options - Build execution options.
 * @param options.packages - All discovered packages.
 * @param options.graph - The dependency graph.
 * @param options.cache - The build cache (for skip detection).
 * @param options.packageHashes - Per-package source hashes.
 * @param options.depsHashes - Per-package dependency hashes.
 * @param options.force - If true, rebuild everything regardless of cache.
 * @param options.dryRun - If true, don't actually build -- just report.
 * @param options.maxJobs - Maximum number of parallel workers.
 * @param options.affectedSet - Set of packages affected by git changes.
 * @returns A map of package names to their BuildResult.
 */
export async function executeBuilds(options: {
  packages: Package[];
  graph: DirectedGraph;
  cache: BuildCache;
  packageHashes: Map<string, string>;
  depsHashes: Map<string, string>;
  force?: boolean;
  dryRun?: boolean;
  maxJobs?: number;
  affectedSet?: Set<string> | null;
}): Promise<Map<string, BuildResult>> {
  const {
    packages,
    graph,
    cache,
    packageHashes,
    depsHashes,
    force = false,
    dryRun = false,
    maxJobs,
    affectedSet = null,
  } = options;

  // Build a lookup from name to Package.
  const pkgByName = new Map<string, Package>();
  for (const pkg of packages) {
    pkgByName.set(pkg.name, pkg);
  }

  // Get the parallel levels from the dependency graph.
  const groups = graph.independentGroups();

  const results = new Map<string, BuildResult>();
  const failedPackages = new Set<string>();

  for (const level of groups) {
    // Determine what to build in this level.
    const toBuild: Package[] = [];

    for (const name of level) {
      if (!pkgByName.has(name)) {
        continue;
      }

      // Check if a dependency failed.
      // In our graph, edges go dep -> pkg, so a package's dependencies
      // are found by checking transitiveDependents (walking reverse edges).
      let depFailed = false;
      for (const dep of graph.transitiveDependents(name)) {
        if (failedPackages.has(dep)) {
          depFailed = true;
          break;
        }
      }

      if (depFailed) {
        results.set(name, {
          packageName: name,
          status: "dep-skipped",
          duration: 0,
          stdout: "",
          stderr: "",
          returnCode: 0,
        });
        continue;
      }

      // Check if we need to build.
      // Priority: git-diff affectedSet > hash-based cache
      if (affectedSet !== null && !affectedSet.has(name)) {
        results.set(name, {
          packageName: name,
          status: "skipped",
          duration: 0,
          stdout: "",
          stderr: "",
          returnCode: 0,
        });
        continue;
      }

      const pkgHash = packageHashes.get(name) ?? "";
      const depHash = depsHashes.get(name) ?? "";

      if (
        affectedSet === null &&
        !force &&
        !cache.needsBuild(name, pkgHash, depHash)
      ) {
        results.set(name, {
          packageName: name,
          status: "skipped",
          duration: 0,
          stdout: "",
          stderr: "",
          returnCode: 0,
        });
        continue;
      }

      if (dryRun) {
        results.set(name, {
          packageName: name,
          status: "would-build",
          duration: 0,
          stdout: "",
          stderr: "",
          returnCode: 0,
        });
        continue;
      }

      toBuild.push(pkgByName.get(name)!);
    }

    if (toBuild.length === 0 || dryRun) {
      continue;
    }

    // Execute this level in parallel (with optional concurrency limit).
    const workers = maxJobs ?? Math.min(toBuild.length, 8);

    // Process in batches to respect maxJobs limit.
    for (let i = 0; i < toBuild.length; i += workers) {
      const batch = toBuild.slice(i, i + workers);
      const batchResults = await Promise.all(
        batch.map((pkg) => runPackageBuild(pkg)),
      );

      for (const result of batchResults) {
        results.set(result.packageName, result);

        // Update cache.
        if (result.status === "built") {
          cache.record(
            result.packageName,
            packageHashes.get(result.packageName) ?? "",
            depsHashes.get(result.packageName) ?? "",
            "success",
          );
        } else if (result.status === "failed") {
          failedPackages.add(result.packageName);
          cache.record(
            result.packageName,
            packageHashes.get(result.packageName) ?? "",
            depsHashes.get(result.packageName) ?? "",
            "failed",
          );
        }
      }
    }
  }

  return results;
}
