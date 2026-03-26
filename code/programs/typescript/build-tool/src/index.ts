#!/usr/bin/env npx tsx
/**
 * index.ts -- Command-Line Interface
 * ====================================
 *
 * This is the entry point for the build tool CLI. It ties together all the
 * modules: discovery, resolution, hashing, caching, execution, and reporting.
 *
 * ## Usage
 *
 * ```bash
 * npx tsx src/index.ts                          # Auto-detect root, build changed packages
 * npx tsx src/index.ts --root /path/to/repo     # Specify root explicitly
 * npx tsx src/index.ts --force                   # Rebuild everything
 * npx tsx src/index.ts --dry-run                 # Show what would build without building
 * npx tsx src/index.ts --jobs 4                  # Limit parallel workers
 * npx tsx src/index.ts --language python         # Only build Python packages
 * ```
 *
 * ## The build flow
 *
 * 1. Discover packages (walk recursive BUILD files)
 * 2. Filter by language if specified
 * 3. Resolve dependencies (parse pyproject.toml, .gemspec, go.mod, etc.)
 * 4. Hash all packages
 * 5. Load cache, determine what needs building
 * 6. If --dry-run, print what would build and exit
 * 7. Execute builds (parallel by level)
 * 8. Update and save cache
 * 9. Print report
 * 10. Exit with code 1 if any builds failed
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { parseArgs } from "node:util";
import { discoverPackages } from "./discovery.js";
import { resolveDependencies } from "./resolver.js";
import { getChangedFiles, mapFilesToPackages } from "./gitdiff.js";
import { hashPackage, hashDeps } from "./hasher.js";
import { BuildCache } from "./cache.js";
import { executeBuilds } from "./executor.js";
import { printReport } from "./reporter.js";
import { writePlan, readPlan, CURRENT_SCHEMA_VERSION } from "./plan.js";
import type { BuildPlan, PackageEntry } from "./plan.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Walk up from `start` (or cwd) looking for a `.git` directory.
 *
 * Returns the directory containing `.git`, or null if not found.
 * This is how we auto-detect the repository root.
 */
function findRepoRoot(start?: string): string | null {
  let current = path.resolve(start ?? process.cwd());

  while (true) {
    if (fs.existsSync(path.join(current, ".git"))) {
      return current;
    }
    const parent = path.dirname(current);
    if (parent === current) {
      // Reached filesystem root.
      return null;
    }
    current = parent;
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(argv?: string[]): Promise<number> {
  // Parse command-line arguments using Node's built-in parseArgs.
  const { values } = parseArgs({
    args: argv ?? process.argv.slice(2),
    options: {
      root: { type: "string" },
      force: { type: "boolean", default: false },
      "dry-run": { type: "boolean", default: false },
      jobs: { type: "string" },
      language: { type: "string", default: "all" },
      "diff-base": { type: "string", default: "origin/main" },
      "cache-file": { type: "string", default: ".build-cache.json" },
      "emit-plan": { type: "boolean", default: false },
      "plan-file": { type: "string", default: "build-plan.json" },
      help: { type: "boolean", default: false },
    },
    strict: true,
  });

  if (values.help) {
    console.log(`Usage: build-tool [options]

Options:
  --root <dir>       Repo root directory (auto-detect from .git if not given)
  --force            Rebuild everything regardless of cache
  --dry-run          Show what would build without actually building
  --jobs <n>         Maximum number of parallel build jobs
  --language <lang>  Only build packages of this language (python|ruby|go|typescript|rust|elixir|all)
  --diff-base <ref>  Git ref to diff against (default: origin/main)
  --cache-file <f>   Path to build cache file (default: .build-cache.json)
  --emit-plan        Write a build plan JSON file and exit (no builds executed)
  --plan-file <f>    Path to write/read the build plan (default: build-plan.json)
  --help             Show this help message`);
    return 0;
  }

  // Step 1: Find repo root.
  let root = values.root ?? null;
  if (root === null) {
    root = findRepoRoot();
    if (root === null) {
      console.error("Error: Could not find repo root (.git directory).");
      console.error("Use --root to specify the repo root.");
      return 1;
    }
  }

  root = path.resolve(root);

  // The build starts from code/ directory.
  const codeRoot = path.join(root, "code");
  if (!fs.existsSync(codeRoot)) {
    console.error(`Error: ${codeRoot} does not exist.`);
    return 1;
  }

  // Step 2: Discover packages.
  let packages = discoverPackages(codeRoot);

  if (packages.length === 0) {
    console.error("No packages found.");
    return 0;
  }

  // Step 3: Filter by language.
  const language = values.language ?? "all";
  if (language !== "all") {
    packages = packages.filter((p) => p.language === language);
    if (packages.length === 0) {
      console.error(`No ${language} packages found.`);
      return 0;
    }
  }

  console.log(`Discovered ${packages.length} packages`);

  // Step 4: Resolve dependencies.
  const graph = resolveDependencies(packages);

  // Step 5: Determine which packages need building.
  //
  // Default mode: git-diff based change detection.
  // Git is the source of truth -- no cache file needed.
  // Fallback: hash-based cache (for local dev when not on a branch).
  let affectedSet: Set<string> | null = null;
  let force = values.force ?? false;
  const dryRun = values["dry-run"] ?? false;
  const diffBase = values["diff-base"] ?? "origin/main";

  if (!force) {
    // Try git-diff mode first (the default).
    const changedFiles = getChangedFiles(root, diffBase);
    if (changedFiles.length > 0) {
      const sharedPrefixes = [".github/"];
      const sharedChanged = changedFiles.some(f => sharedPrefixes.some(p => f.startsWith(p)));

      if (sharedChanged) {
        console.log("Git diff: shared files changed -- rebuilding everything");
        force = true;
        affectedSet = null;
      } else {
        const packagePaths = new Map<string, string>();
        for (const pkg of packages) {
          packagePaths.set(pkg.name, pkg.path);
        }
        const changedPackages = mapFilesToPackages(changedFiles, packagePaths, root);
        if (changedPackages.size > 0) {
          affectedSet = graph.affectedNodes(changedPackages);
          console.log(
            `Git diff: ${changedPackages.size} packages changed, ` +
              `${affectedSet.size} affected (including dependents)`,
          );
        } else {
          console.log("Git diff: no package files changed -- nothing to build");
          affectedSet = new Set();
        }
      }
    } else {
      console.log("Git diff unavailable -- falling back to hash-based cache");
    }
  }

  // Step 5b: Emit build plan if requested.
  //
  // The --emit-plan flag writes a JSON file describing the full build plan
  // (packages, dependency edges, affected set, etc.) and exits without
  // executing any builds. This is useful for CI pipelines that separate
  // planning from execution.
  const emitPlan = values["emit-plan"] ?? false;
  const planFile = values["plan-file"] ?? "build-plan.json";

  if (emitPlan) {
    const planPath = path.isAbsolute(planFile)
      ? planFile
      : path.join(root, planFile);

    // Build the list of package entries for the plan.
    const planPackages: PackageEntry[] = packages.map((pkg) => ({
      name: pkg.name,
      rel_path: path.relative(root, pkg.path),
      language: pkg.language,
      build_commands: pkg.buildCommands,
    }));

    // Build the dependency edges from the graph.
    const edges: [string, string][] = [];
    for (const pkg of packages) {
      // graph.successors gives us the packages this one depends on.
      if (graph.hasNode(pkg.name)) {
        for (const successor of graph.successors(pkg.name)) {
          edges.push([pkg.name, successor]);
        }
      }
    }

    // Determine which languages are needed by the affected packages.
    const langsNeeded: Record<string, boolean> = {};
    const relevantPkgs = affectedSet
      ? packages.filter((p) => affectedSet!.has(p.name))
      : packages;
    for (const pkg of relevantPkgs) {
      langsNeeded[pkg.language] = true;
    }

    const plan: BuildPlan = {
      schema_version: CURRENT_SCHEMA_VERSION,
      diff_base: diffBase,
      force,
      affected_packages: affectedSet ? Array.from(affectedSet).sort() : null,
      packages: planPackages,
      dependency_edges: edges,
      languages_needed: langsNeeded,
    };

    writePlan(plan, planPath);
    console.log(`Build plan written to ${planPath}`);
    return 0;
  }

  // Step 6: Hash all packages (needed for cache fallback).
  const packageHashes = new Map<string, string>();
  const depsHashes = new Map<string, string>();

  for (const pkg of packages) {
    packageHashes.set(pkg.name, hashPackage(pkg));
    depsHashes.set(pkg.name, hashDeps(pkg.name, graph, packageHashes));
  }

  // Step 7: Load cache (fallback if git diff didn't work).
  const cacheFile = values["cache-file"] ?? ".build-cache.json";
  const cachePath = path.isAbsolute(cacheFile)
    ? cacheFile
    : path.join(root, cacheFile);

  const cache = new BuildCache();
  cache.load(cachePath);

  // Steps 8-9: Execute builds.
  const maxJobs = values.jobs ? parseInt(values.jobs, 10) : undefined;

  const results = await executeBuilds({
    packages,
    graph,
    cache,
    packageHashes,
    depsHashes,
    force,
    dryRun,
    maxJobs,
    affectedSet,
  });

  // Step 10: Save cache (as secondary record, not primary mechanism).
  if (!dryRun) {
    cache.save(cachePath);
  }

  // Step 11: Print report.
  printReport(results);

  // Step 12: Exit code.
  const hasFailures = Array.from(results.values()).some(
    (r) => r.status === "failed",
  );
  return hasFailures ? 1 : 0;
}

// Run the CLI.
main().then((code) => {
  process.exit(code);
});
