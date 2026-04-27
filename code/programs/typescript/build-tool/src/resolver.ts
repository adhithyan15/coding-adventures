/**
 * resolver.ts -- Dependency Resolution from Package Metadata
 * ==========================================================
 *
 * This module reads package metadata files (pyproject.toml for Python,
 * .gemspec for Ruby, go.mod for Go, package.json for TypeScript,
 * Cargo.toml for Rust, mix.exs for Elixir) and extracts internal
 * dependencies. It builds a directed graph where edges represent
 * "A depends on B".
 *
 * ## Dependency mapping conventions
 *
 * Each language ecosystem uses a different naming convention for packages
 * in this monorepo:
 *
 * - **Python**: Package names in pyproject.toml use the `coding-adventures-`
 *   prefix with hyphens. For example, `coding-adventures-logic-gates` maps
 *   to the package `python/logic-gates`.
 *
 * - **Ruby**: Gem names in .gemspec use the `coding_adventures_` prefix with
 *   underscores. For example, `coding_adventures_logic_gates` maps to
 *   `ruby/logic_gates`.
 *
 * - **Go**: Module paths in go.mod include the repo path. We map module
 *   paths to `go/X` based on the last path component.
 *
 * - **TypeScript**: Package names in package.json use the `@coding-adventures/`
 *   scoped npm prefix. For example, `@coding-adventures/logic-gates` maps
 *   to `typescript/logic-gates`.
 *
 * - **Rust**: Crate names in Cargo.toml use the directory name (kebab-case).
 *   For example, `logic-gates` maps to `rust/logic-gates`.
 *
 * - **Elixir**: App names in mix.exs use the `coding_adventures_` prefix
 *   with underscores. For example, `coding_adventures_logic_gates` maps to
 *   `elixir/logic-gates`.
 *
 * - **Lua**: Rock names in .rockspec files use the `coding-adventures-`
 *   prefix with hyphens. For example, `coding-adventures-logic-gates` maps
 *   to `lua/logic_gates`.
 *
 * External dependencies (those not matching the monorepo prefix) are
 * silently skipped.
 *
 * ## The Directed Graph
 *
 * We include a minimal DirectedGraph implementation inline to avoid external
 * dependencies. The graph supports:
 * - Adding nodes and edges
 * - Topological sorting via Kahn's algorithm
 * - Transitive closure (all dependencies of a node)
 * - Transitive dependents (all nodes that depend on a given node)
 * - Independent groups (nodes that can be built in parallel)
 * - Affected nodes (given a set of changed nodes, find all that need rebuilding)
 */

import * as fs from "node:fs";
import * as nodePath from "node:path";
import type { Package } from "./discovery.js";

// ===========================================================================
// DirectedGraph -- Inline Minimal Implementation
// ===========================================================================

/**
 * A minimal directed graph for dependency resolution.
 *
 * ## How a directed graph works
 *
 * A directed graph consists of **nodes** (vertices) and **edges** (arrows).
 * Each edge has a direction: from one node to another. In our build system:
 *
 * - Each **node** represents a package (e.g., "python/logic-gates").
 * - Each **edge** goes FROM a dependency TO a dependent:
 *   if "arithmetic" depends on "logic-gates", the edge is:
 *   logic-gates -> arithmetic
 *
 * This convention means nodes with zero in-degree (no incoming edges) have
 * no dependencies and can be built first.
 *
 * ## Data structure
 *
 * We store the graph as two adjacency lists:
 * - `_forward`: maps each node to its successors (nodes it points to)
 * - `_reverse`: maps each node to its predecessors (nodes that point to it)
 *
 * Maintaining both directions lets us efficiently traverse in either
 * direction without rebuilding the graph.
 */
export class DirectedGraph {
  /** Forward adjacency list: node -> set of successors (nodes it depends on) */
  private _forward: Map<string, Set<string>> = new Map();

  /** Reverse adjacency list: node -> set of predecessors (nodes that depend on it) */
  private _reverse: Map<string, Set<string>> = new Map();

  /**
   * Add a node to the graph.
   *
   * If the node already exists, this is a no-op. Every node must be
   * explicitly added before edges can reference it (though addEdge
   * will auto-add nodes too).
   */
  addNode(node: string): void {
    if (!this._forward.has(node)) {
      this._forward.set(node, new Set());
      this._reverse.set(node, new Set());
    }
  }

  /**
   * Add a directed edge from `fromNode` to `toNode`.
   *
   * Both nodes are auto-created if they don't exist yet. The edge
   * means: "fromNode must be built before toNode" (because toNode
   * depends on fromNode).
   */
  addEdge(fromNode: string, toNode: string): void {
    this.addNode(fromNode);
    this.addNode(toNode);
    this._forward.get(fromNode)!.add(toNode);
    this._reverse.get(toNode)!.add(fromNode);
  }

  /** Check if a node exists in the graph. */
  hasNode(node: string): boolean {
    return this._forward.has(node);
  }

  /** Get all nodes in the graph. */
  nodes(): string[] {
    return Array.from(this._forward.keys());
  }

  /** Get the direct successors of a node (nodes it points to). */
  successors(node: string): string[] {
    return Array.from(this._forward.get(node) ?? []);
  }

  /** Get the direct predecessors of a node (nodes that point to it). */
  predecessors(node: string): string[] {
    return Array.from(this._reverse.get(node) ?? []);
  }

  /**
   * Compute all nodes reachable from `node` (not including `node` itself).
   *
   * This is the "transitive closure" -- all nodes you can reach by
   * following edges forward from the given node. In our build system,
   * this gives you all transitive dependents of a package.
   *
   * Algorithm: simple depth-first search (DFS) using an explicit stack.
   * We start with the direct successors and keep following edges until
   * we've visited everything reachable.
   */
  transitiveClosure(node: string): Set<string> {
    if (!this._forward.has(node)) {
      return new Set();
    }

    const visited = new Set<string>();
    const stack = Array.from(this._forward.get(node)!);
    for (const s of stack) visited.add(s);

    while (stack.length > 0) {
      const current = stack.pop()!;
      for (const successor of this._forward.get(current) ?? []) {
        if (!visited.has(successor)) {
          visited.add(successor);
          stack.push(successor);
        }
      }
    }

    return visited;
  }

  /**
   * Compute all nodes that transitively depend on `node`.
   *
   * This walks the REVERSE edges -- finding all nodes that (directly or
   * indirectly) have `node` as a dependency. Used by the hasher to
   * determine if a package needs rebuilding when its dependency changes.
   *
   * Algorithm: same DFS as transitiveClosure but on reverse edges.
   */
  transitiveDependents(node: string): Set<string> {
    if (!this._reverse.has(node)) {
      return new Set();
    }

    const visited = new Set<string>();
    const stack = Array.from(this._reverse.get(node)!);
    for (const s of stack) visited.add(s);

    while (stack.length > 0) {
      const current = stack.pop()!;
      for (const predecessor of this._reverse.get(current) ?? []) {
        if (!visited.has(predecessor)) {
          visited.add(predecessor);
          stack.push(predecessor);
        }
      }
    }

    return visited;
  }

  /**
   * Partition nodes into parallel execution levels using Kahn's algorithm.
   *
   * ## What is Kahn's algorithm?
   *
   * Kahn's algorithm is a method for topological sorting -- arranging nodes
   * in an order where every dependency comes before its dependent. But we
   * go further: we group nodes into LEVELS where all nodes in a level can
   * be processed simultaneously (they have no dependencies on each other).
   *
   * ## How it works
   *
   * 1. Compute the "in-degree" of each node (number of incoming edges).
   *    Nodes with in-degree 0 have no dependencies.
   *
   * 2. All in-degree-0 nodes form Level 0 -- they can be built first,
   *    in parallel.
   *
   * 3. "Remove" Level 0 from the graph (decrement in-degrees of their
   *    successors). Any nodes that now have in-degree 0 form Level 1.
   *
   * 4. Repeat until all nodes are assigned to a level.
   *
   * 5. If we processed fewer nodes than exist in the graph, there's a
   *    cycle (circular dependency), which is an error.
   *
   * ## Example
   *
   * Given: A -> B -> D, A -> C -> D
   * - Level 0: [A]       (no dependencies)
   * - Level 1: [B, C]    (only depend on A, which is done)
   * - Level 2: [D]       (depends on B and C, both done)
   *
   * @returns Array of levels, where each level is an array of node names.
   * @throws Error if the graph contains a cycle.
   */
  independentGroups(): string[][] {
    // Step 1: Compute in-degree for every node.
    const inDegree = new Map<string, number>();
    for (const [node, preds] of this._reverse) {
      inDegree.set(node, preds.size);
    }

    // Step 2: Find all nodes with in-degree 0 (no dependencies).
    let currentLevel = Array.from(inDegree.entries())
      .filter(([_, degree]) => degree === 0)
      .map(([node]) => node)
      .sort();

    const groups: string[][] = [];
    let processed = 0;

    // Step 3: Process levels until no more nodes remain.
    while (currentLevel.length > 0) {
      groups.push(currentLevel);
      processed += currentLevel.length;

      // Step 4: "Remove" current level and find the next one.
      const nextLevelSet = new Set<string>();
      for (const node of currentLevel) {
        for (const successor of this._forward.get(node)!) {
          const newDegree = inDegree.get(successor)! - 1;
          inDegree.set(successor, newDegree);
          if (newDegree === 0) {
            nextLevelSet.add(successor);
          }
        }
      }

      currentLevel = Array.from(nextLevelSet).sort();
    }

    // Step 5: Cycle detection.
    if (processed !== this._forward.size) {
      throw new Error("Dependency graph contains a cycle");
    }

    return groups;
  }

  /**
   * Given a set of changed nodes, find all nodes that need rebuilding.
   *
   * This is the primary method for git-diff based change detection. If
   * you change "logic-gates", the affected set includes "logic-gates"
   * itself plus all its transitive dependents (arithmetic, cpu-simulator,
   * etc.).
   *
   * @param changed - Set of node names that have changed.
   * @returns Set of all affected node names (changed + their dependents).
   */
  affectedNodes(changed: Set<string>): Set<string> {
    const affected = new Set<string>();

    for (const node of changed) {
      if (!this.hasNode(node)) {
        continue;
      }
      affected.add(node);
      for (const dep of this.transitiveClosure(node)) {
        affected.add(dep);
      }
    }

    return affected;
  }
}

// ===========================================================================
// Dependency Parsing -- Python
// ===========================================================================

/**
 * Extract internal dependencies from a Python package's pyproject.toml.
 *
 * Reads the `[project] dependencies` list and maps entries with the
 * `coding-adventures-` prefix to their package names. Version specifiers
 * (>=, <, etc.) are stripped.
 *
 * We use simple line-by-line parsing rather than a TOML library to avoid
 * external dependencies. The strategy:
 * 1. Find the "dependencies = [" line
 * 2. Collect lines until we hit "]"
 * 3. Extract quoted strings and strip version specifiers
 */
function parsePythonDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  const pyprojectPath = nodePath.join(pkg.path, "pyproject.toml");
  if (!fs.existsSync(pyprojectPath)) {
    return [];
  }

  const text = fs.readFileSync(pyprojectPath, "utf-8");
  const internalDeps: string[] = [];

  let inDeps = false;
  for (const line of text.split("\n")) {
    const trimmed = line.trim();

    if (!inDeps) {
      // Look for the start of the dependencies array.
      if (trimmed.startsWith("dependencies") && trimmed.includes("=")) {
        const afterEq = trimmed.split("=").slice(1).join("=").trim();

        if (afterEq.startsWith("[")) {
          if (afterEq.includes("]")) {
            // Single-line array: dependencies = ["foo", "bar"]
            extractDeps(afterEq, knownNames, internalDeps);
            break;
          }
          // Multi-line array starts here.
          inDeps = true;
          extractDeps(afterEq, knownNames, internalDeps);
        }
      }
      continue;
    }

    // We're inside a multi-line dependencies array.
    if (trimmed.includes("]")) {
      extractDeps(trimmed, knownNames, internalDeps);
      break;
    }
    extractDeps(trimmed, knownNames, internalDeps);
  }

  return internalDeps;
}

/**
 * Extract quoted dependency names from a line and map them to internal
 * package names. Version specifiers (>=, <, etc.) are stripped.
 *
 * This helper is used by parsePythonDeps to process individual lines
 * of the dependencies array.
 */
function extractDeps(
  line: string,
  knownNames: Map<string, string>,
  deps: string[],
): void {
  // Match quoted strings: "something" or 'something'
  const re = /["']([^"']+)["']/g;
  let match: RegExpExecArray | null;

  while ((match = re.exec(line)) !== null) {
    // Strip version specifiers: split on >=, <=, >, <, ==, !=, ~=, ;, spaces
    const depName = match[1].split(/[>=<!~\s;]/)[0].trim().toLowerCase();
    const pkgName = knownNames.get(depName);
    if (pkgName) {
      deps.push(pkgName);
    }
  }
}

// ===========================================================================
// Dependency Parsing -- Ruby
// ===========================================================================

/**
 * Extract internal dependencies from a Ruby package's .gemspec file.
 *
 * Looks for lines matching `spec.add_dependency "coding_adventures_X"`
 * and maps them to package names. Ruby uses underscores in gem names.
 */
function parseRubyDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  // Find .gemspec files in the package directory.
  let entries: string[];
  try {
    entries = fs.readdirSync(pkg.path);
  } catch {
    return [];
  }

  const gemspecFile = entries.find((e) => e.endsWith(".gemspec"));
  if (!gemspecFile) {
    return [];
  }

  const text = fs.readFileSync(nodePath.join(pkg.path, gemspecFile), "utf-8");
  const internalDeps: string[] = [];

  // Match: spec.add_dependency "coding_adventures_something"
  const pattern = /spec\.add_dependency\s+"([^"]+)"/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    const gemName = match[1].trim().toLowerCase();
    const pkgName = knownNames.get(gemName);
    if (pkgName) {
      internalDeps.push(pkgName);
    }
  }

  return internalDeps;
}

// ===========================================================================
// Dependency Parsing -- Go
// ===========================================================================

/**
 * Extract internal dependencies from a Go package's go.mod file.
 *
 * Looks for `require` lines and maps module paths to package names.
 * Handles both single-line `require` directives and `require (...)` blocks.
 */
function parseGoDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  const goModPath = nodePath.join(pkg.path, "go.mod");
  if (!fs.existsSync(goModPath)) {
    return [];
  }

  const text = fs.readFileSync(goModPath, "utf-8");
  const internalDeps: string[] = [];

  let inRequireBlock = false;
  for (const line of text.split("\n")) {
    const stripped = line.trim();

    if (stripped === "require (") {
      inRequireBlock = true;
      continue;
    }
    if (stripped === ")") {
      inRequireBlock = false;
      continue;
    }

    if (inRequireBlock || stripped.startsWith("require ")) {
      // Extract module path (first space-separated token).
      const parts = stripped.replace("require ", "").trim().split(/\s+/);
      if (parts.length > 0) {
        const modulePath = parts[0].toLowerCase();
        const pkgName = knownNames.get(modulePath);
        if (pkgName) {
          internalDeps.push(pkgName);
        }
      }
    }
  }

  return internalDeps;
}

// ===========================================================================
// Dependency Parsing -- TypeScript
// ===========================================================================

/**
 * Extract internal dependencies from a TypeScript package.json file.
 *
 * TypeScript packages declare dependencies in package.json:
 *
 *     "dependencies": {
 *         "@coding-adventures/logic-gates": "file:../logic-gates"
 *     }
 *
 * We scan for lines inside a "dependencies" block and extract
 * `@coding-adventures/` references. Version specifiers and `file:`
 * references are ignored -- we only care about the package name.
 */
function parseTypescriptDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  const packageJsonPath = nodePath.join(pkg.path, "package.json");
  if (!fs.existsSync(packageJsonPath)) {
    return [];
  }

  const text = fs.readFileSync(packageJsonPath, "utf-8");
  const internalDeps: string[] = [];

  // Strategy: scan line by line, looking for "dependencies" blocks,
  // then extract @coding-adventures/ references from those blocks.
  let inDeps = false;
  const re = /"(@coding-adventures\/[^"]+)"/g;

  for (const line of text.split("\n")) {
    const trimmed = line.trim();

    if (!inDeps) {
      // Look for "dependencies": { or "dependencies":{
      if (trimmed.includes('"dependencies"') && trimmed.includes("{")) {
        inDeps = true;
      }
      continue;
    }

    // Inside dependencies block.
    if (trimmed.includes("}")) {
      inDeps = false;
      continue;
    }

    let match: RegExpExecArray | null;
    re.lastIndex = 0;
    while ((match = re.exec(trimmed)) !== null) {
      const depName = match[1].toLowerCase();
      const pkgName = knownNames.get(depName);
      if (pkgName) {
        internalDeps.push(pkgName);
      }
    }
  }

  return internalDeps;
}

// ===========================================================================
// Dependency Parsing -- Rust
// ===========================================================================

/**
 * Extract internal dependencies from a Rust Cargo.toml file.
 *
 * Rust Cargo.toml declares workspace-local dependencies with path references:
 *
 *     [dependencies]
 *     logic-gates = { path = "../logic-gates" }
 *
 * We look for lines in the [dependencies] section that contain `path =`
 * and extract the crate name (the key before the `=`). We then look up
 * that name in the known names mapping.
 */
function parseRustDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  const cargoTomlPath = nodePath.join(pkg.path, "Cargo.toml");
  if (!fs.existsSync(cargoTomlPath)) {
    return [];
  }

  const text = fs.readFileSync(cargoTomlPath, "utf-8");
  const internalDeps: string[] = [];

  // Scan for [dependencies] section and extract path-based deps.
  let inDeps = false;
  for (const line of text.split("\n")) {
    const trimmed = line.trim();

    // Detect section headers like [dependencies] or [dev-dependencies]
    if (trimmed.startsWith("[")) {
      inDeps = trimmed === "[dependencies]";
      continue;
    }

    if (!inDeps) {
      continue;
    }

    // Look for lines like: logic-gates = { path = "../logic-gates" }
    if (trimmed.includes("path") && trimmed.includes("=")) {
      const parts = trimmed.split("=");
      if (parts.length >= 2) {
        const crateName = parts[0].trim().toLowerCase();
        const pkgName = knownNames.get(crateName);
        if (pkgName) {
          internalDeps.push(pkgName);
        }
      }
    }
  }

  return internalDeps;
}

// ===========================================================================
// Dependency Parsing -- Elixir
// ===========================================================================

/**
 * Extract internal dependencies from an Elixir mix.exs file.
 *
 * Elixir mix.exs declares internal path dependencies like:
 *
 *     {:coding_adventures_logic_gates, path: "../logic-gates"}
 *
 * We use a regex to capture the atom name starting with `coding_adventures_`.
 */
function parseElixirDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  const mixExsPath = nodePath.join(pkg.path, "mix.exs");
  if (!fs.existsSync(mixExsPath)) {
    return [];
  }

  const text = fs.readFileSync(mixExsPath, "utf-8");
  const internalDeps: string[] = [];

  const pattern = /\{:(coding_adventures_[a-z0-9_]+)/g;
  for (const line of text.split("\n")) {
    let match: RegExpExecArray | null;
    pattern.lastIndex = 0;
    while ((match = pattern.exec(line)) !== null) {
      const appName = match[1].toLowerCase();
      const pkgName = knownNames.get(appName);
      if (pkgName) {
        internalDeps.push(pkgName);
      }
    }
  }

  return internalDeps;
}

// ===========================================================================
// Dependency Parsing -- Lua
// ===========================================================================

/**
 * Extract internal dependencies from a Lua package's .rockspec file.
 *
 * Lua rockspec files declare dependencies in a block like:
 *
 *     dependencies = {
 *         "lua >= 5.4",
 *         "coding-adventures-logic-gates >= 0.1.0",
 *     }
 *
 * We scan for the `dependencies = {` line, collect quoted strings until
 * the closing `}`, strip version specifiers (>=, <=, >, <, ==, ~>), and
 * look up each name in the known names mapping.
 */
function parseLuaDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  // Find .rockspec files in the package directory.
  let entries: string[];
  try {
    entries = fs.readdirSync(pkg.path);
  } catch {
    return [];
  }

  const rockspecFile = entries.find((e) => e.endsWith(".rockspec"));
  if (!rockspecFile) {
    return [];
  }

  const text = fs.readFileSync(nodePath.join(pkg.path, rockspecFile), "utf-8");
  const internalDeps: string[] = [];

  // Strategy: scan line by line for the dependencies block,
  // then extract quoted strings and strip version specifiers.
  let inDeps = false;
  for (const line of text.split("\n")) {
    const trimmed = line.trim();

    if (!inDeps) {
      // Look for the start of the dependencies block.
      if (trimmed.startsWith("dependencies") && trimmed.includes("=") && trimmed.includes("{")) {
        inDeps = true;
        continue;
      }
      continue;
    }

    // We're inside the dependencies block.
    if (trimmed.includes("}")) {
      break;
    }

    // Extract quoted strings: "something >= 1.0" or 'something >= 1.0'
    const re = /["']([^"']+)["']/g;
    let match: RegExpExecArray | null;

    while ((match = re.exec(trimmed)) !== null) {
      // Strip version specifiers: split on >=, <=, >, <, ==, ~>, or whitespace.
      const depName = match[1].split(/[>=<!~\s]/)[0].trim().toLowerCase();
      const pkgName = knownNames.get(depName);
      if (pkgName) {
        internalDeps.push(pkgName);
      }
    }
  }

  return internalDeps;
}

// ===========================================================================
// Dependency Parsing -- Perl
// ===========================================================================

/**
 * Extract internal dependencies from a Perl cpanfile.
 *
 * A cpanfile declares dependencies with one `requires` per line:
 *
 *     requires 'coding-adventures-logic-gates';
 *     requires 'coding-adventures-bitset', '>= 0.01';
 *
 * We scan for lines matching `requires 'coding-adventures-...'` and map
 * them to internal package names. External deps are silently skipped.
 *
 * @param pkg - The Perl package to inspect.
 * @param knownNames - Mapping from CPAN dist name to package name.
 * @returns List of internal package names this package depends on.
 */
function parsePerlDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  const cpanfilePath = nodePath.join(pkg.path, "cpanfile");
  if (!fs.existsSync(cpanfilePath)) {
    return [];
  }

  const text = fs.readFileSync(cpanfilePath, "utf-8");
  const internalDeps: string[] = [];
  const pattern = /requires\s+['"]coding-adventures-([^'"]+)['"]/;

  for (const line of text.split("\n")) {
    const trimmed = line.trim();

    // Skip blank lines and comments.
    if (!trimmed || trimmed.startsWith("#")) {
      continue;
    }

    const match = trimmed.match(pattern);
    if (match) {
      const depName = `coding-adventures-${match[1]}`.toLowerCase();
      const pkgName = knownNames.get(depName);
      if (pkgName) {
        internalDeps.push(pkgName);
      }
    }
  }

  return internalDeps;
}

// ===========================================================================
// Dependency Parsing -- Haskell
// ===========================================================================

/**
 * Extract internal dependencies from a Haskell Cabal file.
 *
 * Looks for dependencies matching `coding-adventures-*`.
 */
function parseHaskellDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  const files: fs.Dirent[] = [];
  try {
    files.push(...fs.readdirSync(pkg.path, { withFileTypes: true }));
  } catch {
    return [];
  }

  const cabalFile = files.find(f => f.isFile() && f.name.endsWith(".cabal"));
  if (!cabalFile) {
    return [];
  }

  const text = fs.readFileSync(nodePath.join(pkg.path, cabalFile.name), "utf-8");
  const internalDeps: string[] = [];
  const pattern = /coding-adventures-([a-z0-9-]+)/g;
  let match;

  while ((match = pattern.exec(text)) !== null) {
    const depName = `coding-adventures-${match[1].toLowerCase()}`;
    const pkgName = knownNames.get(depName);
    if (pkgName && pkgName !== pkg.name) {
      internalDeps.push(pkgName);
    }
  }

  return internalDeps;
}

// ===========================================================================
// Known Names Mapping
// ===========================================================================

/**
 * Build a mapping from ecosystem-specific dependency names to package names.
 *
 * This is the Rosetta Stone that connects different naming conventions:
 *
 * - Python:     "coding-adventures-logic-gates" -> "python/logic-gates"
 * - Ruby:       "coding_adventures_logic_gates" -> "ruby/logic_gates"
 * - Go:         full module path                -> "go/module-name"
 * - TypeScript: "@coding-adventures/logic-gates" -> "typescript/logic-gates"
 * - Rust:       "logic-gates"                   -> "rust/logic-gates"
 * - Elixir:     "coding_adventures_logic_gates" -> "elixir/logic-gates"
 *
 * @param packages - All discovered packages.
 * @returns A Map from dependency names to internal package names.
 */
export function buildKnownNames(packages: Package[]): Map<string, string> {
  return buildKnownNamesForScope(packages, "");
}

function dependencyScope(language: string): string {
  if (language === "csharp" || language === "fsharp" || language === "dotnet") {
    return "dotnet";
  }
  if (language === "wasm") {
    return "wasm";
  }
  return language;
}

function inDependencyScope(packageLanguage: string, scope: string): boolean {
  if (scope === "dotnet") {
    return ["csharp", "fsharp", "dotnet"].includes(packageLanguage);
  }
  if (scope === "wasm") {
    return ["wasm", "rust"].includes(packageLanguage);
  }
  return packageLanguage === scope;
}

function readCargoPackageName(pkgPath: string): string | null {
  const cargoTomlPath = nodePath.join(pkgPath, "Cargo.toml");
  if (!fs.existsSync(cargoTomlPath)) {
    return null;
  }

  const match = fs.readFileSync(cargoTomlPath, "utf-8").match(/^\s*name\s*=\s*"([^"]+)"/m);
  return match ? match[1].trim().toLowerCase() : null;
}

function buildKnownNamesForScope(packages: Package[], scope: string): Map<string, string> {
  const known = new Map<string, string>();
  const knownLanguage = new Map<string, string>();
  const scope = dependencyScope(language);

  function setKnown(
    key: string,
    value: string,
    pkgPath: string,
    pkgLanguage: string,
  ): void {
    const existing = known.get(key);
    if (!existing) {
      known.set(key, value);
      knownLanguage.set(key, pkgLanguage);
      return;
    }

    const existingLanguage = knownLanguage.get(key) ?? "";
    const existingIsProgram = existing.includes("/programs/");
    const currentIsProgram = pkgPath.replace(/\\/g, "/").includes("/programs/");

    if (existingIsProgram && !currentIsProgram) {
      known.set(key, value);
      knownLanguage.set(key, pkgLanguage);
      return;
    }

    if (!existingIsProgram && currentIsProgram) {
      return;
    }

    if (scope === "wasm") {
      if (existingLanguage === "rust") {
        return;
      }
      if (pkgLanguage === "rust") {
        known.set(key, value);
        knownLanguage.set(key, pkgLanguage);
        return;
      }
    }

    if (scope === "dotnet") {
      if (existingLanguage === language) {
        return;
      }
      if (pkgLanguage === language) {
        known.set(key, value);
        knownLanguage.set(key, pkgLanguage);
        return;
      }
    }

    if (!currentIsProgram) {
      known.set(key, value);
      knownLanguage.set(key, pkgLanguage);
    }
  }

  for (const pkg of packages) {
    if (scope && !inDependencyScope(pkg.language, scope)) {
      continue;
    }
    const dirName = nodePath.basename(pkg.path);

    switch (pkg.language) {
      case "python": {
        // Convert dir name to PyPI name: "logic-gates" -> "coding-adventures-logic-gates"
        const pypiName = `coding-adventures-${dirName}`.toLowerCase();
        setKnown(pypiName, pkg.name, pkg.path, pkg.language);
        break;
      }

      case "ruby": {
        // Convert dir name to gem name: "logic_gates" -> "coding_adventures_logic_gates"
        const gemName = `coding_adventures_${dirName}`.toLowerCase();
        setKnown(gemName, pkg.name, pkg.path, pkg.language);
        break;
      }

      case "go": {
        // For Go, read the module path from go.mod.
        const goModPath = nodePath.join(pkg.path, "go.mod");
        if (fs.existsSync(goModPath)) {
          const text = fs.readFileSync(goModPath, "utf-8");
          for (const line of text.split("\n")) {
            if (line.startsWith("module ")) {
              const modulePath = line
                .replace("module ", "")
                .trim()
                .toLowerCase();
              known.set(modulePath, pkg.name);
              knownLanguage.set(modulePath, pkg.language);
              break;
            }
          }
        }
        break;
      }

      case "typescript": {
        // Convert dir name to npm scoped name: "logic-gates" -> "@coding-adventures/logic-gates"
        const npmName = `@coding-adventures/${dirName}`.toLowerCase();
        setKnown(npmName, pkg.name, pkg.path, pkg.language);
        break;
      }

      case "rust":
      case "wasm": {
        // Rust crate names use the directory name directly (kebab-case).
        const crateName = dirName.toLowerCase();
        known.set(crateName, pkg.name);
        const cargoName = readCargoPackageName(pkg.path);
        if (cargoName) {
          known.set(cargoName, pkg.name);
        }
        break;
      }

      case "elixir": {
        // Elixir mix names replace hyphens with underscores:
        // "logic-gates" -> "coding_adventures_logic_gates"
        const appName =
          `coding_adventures_${dirName.replace(/-/g, "_")}`.toLowerCase();
        setKnown(appName, pkg.name, pkg.path, pkg.language);
        break;
      }

      case "lua": {
        // Lua rock names replace underscores with hyphens and add the prefix:
        // "logic_gates" -> "coding-adventures-logic-gates"
        const rockName =
          `coding-adventures-${dirName.replace(/_/g, "-")}`.toLowerCase();
        setKnown(rockName, pkg.name, pkg.path, pkg.language);
        break;
      }

      case "perl": {
        // Perl CPAN dist names use hyphens: "logic-gates" -> "coding-adventures-logic-gates"
        // This matches the Python convention exactly.
        const cpanName = `coding-adventures-${dirName}`.toLowerCase();
        setKnown(cpanName, pkg.name, pkg.path, pkg.language);
        break;
      }

      case "haskell": {
        // Haskell Cabal package names use hyphens.
        const cabalName = `coding-adventures-${dirName}`.toLowerCase();
        setKnown(cabalName, pkg.name, pkg.path, pkg.language);
        break;
      }
      case "csharp":
      case "fsharp":
      case "dotnet": {
        setKnown(dirName.toLowerCase(), pkg.name, pkg.path, pkg.language);
        break;
      }

      case "haskell": {
        // Haskell Cabal package names use hyphens.
        const cabalName = `coding-adventures-${dirName}`.toLowerCase();
        known.set(cabalName, pkg.name);
        break;
      }
      case "csharp":
      case "fsharp":
      case "dotnet": {
        known.set(dirName.toLowerCase(), pkg.name);
        break;
      }
    }
  }

  return known;
}

function parseDotnetDeps(
  pkg: Package,
  knownNames: Map<string, string>,
): string[] {
  let entries: string[];
  try {
    entries = fs.readdirSync(pkg.path);
  } catch {
    return [];
  }

  const projectFiles = entries
    .filter((entry) => entry.endsWith(".csproj") || entry.endsWith(".fsproj"))
    .map((entry) => nodePath.join(pkg.path, entry));

  const internalDeps: string[] = [];
  const re = /<ProjectReference\s+Include\s*=\s*"\.\.[\\/]+([^/\\"]+)[\\/][^"]*"/g;

  for (const projectFile of projectFiles) {
    const text = fs.readFileSync(projectFile, "utf-8");
    let match: RegExpExecArray | null;
    re.lastIndex = 0;
    while ((match = re.exec(text)) !== null) {
      const depDir = match[1].toLowerCase();
      if (depDir.includes("/") || depDir.includes("\\") || depDir === "..") {
        continue;
      }
      const pkgName = knownNames.get(depDir);
      if (pkgName) {
        internalDeps.push(pkgName);
      }
    }
  }

  return internalDeps;
}

// ===========================================================================
// Public API
// ===========================================================================

/**
 * Parse package metadata to discover dependencies and build a graph.
 *
 * The graph contains all discovered packages as nodes. Edges represent
 * "A depends on B" (A -> B means A needs B built first). External
 * dependencies (not found among the discovered packages) are silently
 * skipped.
 *
 * @param packages - List of discovered packages.
 * @returns A DirectedGraph with dependency edges.
 */
export function resolveDependencies(packages: Package[]): DirectedGraph {
  const graph = new DirectedGraph();

  // First, add all packages as nodes. Even packages with no dependencies
  // need to be in the graph so they appear in independentGroups().
  for (const pkg of packages) {
    graph.addNode(pkg.name);
  }

  // Build the name-mapping table.
  const knownNamesByScope = new Map<string, Map<string, string>>();
  for (const pkg of packages) {
    const scope = dependencyScope(pkg.language);
    if (!knownNamesByScope.has(scope)) {
      knownNamesByScope.set(scope, buildKnownNamesForScope(packages, scope));
    }
  }

  // Parse dependencies for each package.
  for (const pkg of packages) {
    const knownNames = knownNamesByScope.get(dependencyScope(pkg.language)) ?? new Map<string, string>();
    let deps: string[];

    switch (pkg.language) {
      case "python":
        deps = parsePythonDeps(pkg, knownNames);
        break;
      case "ruby":
        deps = parseRubyDeps(pkg, knownNames);
        break;
      case "go":
        deps = parseGoDeps(pkg, knownNames);
        break;
      case "typescript":
        deps = parseTypescriptDeps(pkg, knownNames);
        break;
      case "rust":
      case "wasm":
        deps = parseRustDeps(pkg, knownNames);
        break;
      case "elixir":
        deps = parseElixirDeps(pkg, knownNames);
        break;
      case "lua":
        deps = parseLuaDeps(pkg, knownNames);
        break;
      case "perl":
        deps = parsePerlDeps(pkg, knownNames);
        break;
      case "haskell":
        deps = parseHaskellDeps(pkg, knownNames);
        break;
      case "csharp":
      case "fsharp":
      case "dotnet":
        deps = parseDotnetDeps(pkg, knownNames);
        break;
      default:
        deps = [];
    }

    for (const depName of deps) {
      // Edge direction: dep -> pkg means "dep must be built before pkg".
      // This makes independentGroups() produce the correct build order:
      // nodes with zero in-degree (no dependencies) come first.
      graph.addEdge(depName, pkg.name);
    }
  }

  return graph;
}
