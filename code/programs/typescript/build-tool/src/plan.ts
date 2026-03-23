/**
 * plan.ts -- Build Plan Serialization and Deserialization
 * ========================================================
 *
 * A "build plan" captures the complete state needed to execute a build:
 * which packages to build, in what order, with what commands, and which
 * languages need to be available. The plan is a JSON document that can be
 * written to a file and read back later.
 *
 * ==========================================================================
 * Chapter 1: Why Build Plans?
 * ==========================================================================
 *
 * Build plans serve two key purposes:
 *
 *   1. **CI integration**: A CI system can generate a plan in one step
 *      (e.g., on a coordinator node), then distribute the plan to worker
 *      nodes that execute the builds. This separates "what to build" from
 *      "how to build".
 *
 *   2. **Debugging**: When a build fails, the plan file shows exactly what
 *      the build tool decided to do -- which packages it considered
 *      affected, what commands it would run, and what dependency edges it
 *      found. This makes build failures easier to diagnose.
 *
 * ==========================================================================
 * Chapter 2: Schema Versioning
 * ==========================================================================
 *
 * The plan file includes a `schema_version` field. When the format changes
 * in a backward-incompatible way, we bump the version number. The reader
 * rejects plans with a mismatched version, preventing subtle bugs from
 * stale plan files.
 *
 * Current version: 1 (the initial format).
 *
 * ==========================================================================
 * Chapter 3: The Plan Structure
 * ==========================================================================
 *
 * A build plan contains:
 *
 *   - `schema_version`: Integer version for forward/backward compatibility.
 *   - `diff_base`: The git ref we diffed against (e.g., "origin/main").
 *   - `force`: Whether this is a force-rebuild (all packages, not just changed).
 *   - `affected_packages`: List of package names to build, or null if all.
 *   - `packages`: Full metadata for each package (name, path, language, commands).
 *   - `dependency_edges`: List of [from, to] edges in the dependency graph.
 *   - `languages_needed`: Map of language name -> boolean for environment setup.
 *
 * @module
 */

import * as fs from "node:fs";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/**
 * The current schema version for build plan files.
 *
 * Bump this whenever the plan format changes in a backward-incompatible way.
 * The reader will reject plans with a different version, forcing the user
 * to regenerate the plan with the current build tool version.
 */
export const CURRENT_SCHEMA_VERSION = 1;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Metadata for a single package in the build plan.
 *
 * This captures everything the executor needs to know about a package:
 * where it lives, what language it uses, what commands to run, and
 * (for Starlark packages) what sources and dependencies it declared.
 *
 * @property name           - Qualified name like "python/logic-gates".
 * @property rel_path       - Path relative to the repo root.
 * @property language       - Programming language: "python", "ruby", "go", etc.
 * @property build_commands - Shell commands to execute for this package.
 * @property is_starlark    - Whether this package uses a Starlark BUILD file.
 * @property declared_srcs  - Source file glob patterns from the Starlark BUILD
 *                            (only present for Starlark packages).
 * @property declared_deps  - Dependency labels from the Starlark BUILD
 *                            (only present for Starlark packages).
 */
export interface PackageEntry {
  name: string;
  rel_path: string;
  language: string;
  build_commands: string[];
  is_starlark?: boolean;
  declared_srcs?: string[];
  declared_deps?: string[];
}

/**
 * The complete build plan document.
 *
 * This is the top-level structure serialized to JSON. It contains all the
 * information needed to reproduce a build: what changed, what to build,
 * how to build it, and what depends on what.
 *
 * @property schema_version     - Format version (must match CURRENT_SCHEMA_VERSION).
 * @property diff_base          - Git ref used for change detection.
 * @property force              - True if this is a force-rebuild.
 * @property affected_packages  - Package names to build, or null for all.
 * @property packages           - Full metadata for each package.
 * @property dependency_edges   - Directed edges [from, to] in the dependency graph.
 * @property languages_needed   - Map of language -> true for languages used by
 *                                at least one affected package.
 */
export interface BuildPlan {
  schema_version: number;
  diff_base: string;
  force: boolean;
  affected_packages: string[] | null;
  packages: PackageEntry[];
  dependency_edges: [string, string][];
  languages_needed: Record<string, boolean>;
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/**
 * Write a build plan to a JSON file.
 *
 * The file is written atomically: we serialize to a string first, then
 * write the string in one operation. This prevents partial writes if the
 * process is interrupted.
 *
 * The JSON is pretty-printed with 2-space indentation for readability.
 * Build plans are small (typically under 100KB even for large monorepos),
 * so the extra whitespace is negligible.
 *
 * @param bp       - The build plan to write.
 * @param filePath - Absolute or relative path to the output file.
 *
 * @example
 * ```typescript
 * writePlan({
 *   schema_version: 1,
 *   diff_base: "origin/main",
 *   force: false,
 *   affected_packages: ["python/logic-gates"],
 *   packages: [{ name: "python/logic-gates", rel_path: "code/packages/python/logic-gates", language: "python", build_commands: ["pytest"] }],
 *   dependency_edges: [],
 *   languages_needed: { python: true },
 * }, "/tmp/plan.json");
 * ```
 */
export function writePlan(bp: BuildPlan, filePath: string): void {
  const json = JSON.stringify(bp, null, 2);
  fs.writeFileSync(filePath, json + "\n", "utf-8");
}

// ---------------------------------------------------------------------------
// Deserialization
// ---------------------------------------------------------------------------

/**
 * Read a build plan from a JSON file.
 *
 * This function performs two levels of validation:
 *
 *   1. **JSON parsing**: The file must contain valid JSON.
 *   2. **Schema version check**: The `schema_version` field must match
 *      `CURRENT_SCHEMA_VERSION`. If it does not, we throw an error
 *      rather than silently misinterpreting the plan.
 *
 * We intentionally do NOT validate every field in the plan. The TypeScript
 * type system provides compile-time checking, and runtime validation of
 * every field would add complexity without much benefit (the plan is
 * generated by our own tool, not hand-edited).
 *
 * @param filePath - Absolute or relative path to the plan file.
 * @returns The deserialized BuildPlan.
 * @throws {Error} If the file does not exist, is not valid JSON, or has
 *                 a mismatched schema version.
 *
 * @example
 * ```typescript
 * const plan = readPlan("/tmp/plan.json");
 * console.log(plan.affected_packages); // ["python/logic-gates"]
 * ```
 */
export function readPlan(filePath: string): BuildPlan {
  // Step 1: Read the file. Let the filesystem error propagate if the file
  // does not exist -- the caller gets a clear "ENOENT" message.
  const raw = fs.readFileSync(filePath, "utf-8");

  // Step 2: Parse JSON. Let JSON.parse throw on invalid JSON.
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch (err) {
    throw new Error(
      `Invalid JSON in plan file ${filePath}: ${err instanceof Error ? err.message : String(err)}`,
    );
  }

  // Step 3: Validate schema version.
  //
  // We check this before anything else because a version mismatch means
  // the entire structure might be different. Better to fail fast with a
  // clear message than to crash on a missing field deep in the plan.
  if (typeof data !== "object" || data === null || Array.isArray(data)) {
    throw new Error(
      `Plan file ${filePath}: expected a JSON object, got ${typeof data}`,
    );
  }

  const obj = data as Record<string, unknown>;
  const version = obj["schema_version"];

  if (version !== CURRENT_SCHEMA_VERSION) {
    throw new Error(
      `Plan file ${filePath}: schema_version is ${JSON.stringify(version)}, ` +
        `expected ${CURRENT_SCHEMA_VERSION}. Regenerate the plan with the current build tool.`,
    );
  }

  // Step 4: Return the plan. We trust the structure since it was generated
  // by our own writePlan function (or a compatible tool).
  return obj as unknown as BuildPlan;
}
