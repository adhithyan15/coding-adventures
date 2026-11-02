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
 * - **Rust**: Crate names in Cargo.toml use the directory name (kebab-case)
 *   or the declared `[package]` name. For example, `logic-gates` maps to
 *   `rust/logic-gates`. Path entries count in `[dependencies]`,
 *   `[build-dependencies]` and their `[target.<cfg>.…]` forms.
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

/** Stable diagnostic for metadata that is not valid UTF-8. */
export class MetadataEncodingError extends Error {
  readonly code: string;
  readonly package: string;
  readonly manifest: string;
  readonly encoding: string;

  constructor(
    code: string,
    packageName: string,
    manifest: string,
    encoding: string,
  ) {
    super(
      `${code}: package=${packageName} manifest=${manifest} encoding=${encoding}`,
    );
    this.name = "MetadataEncodingError";
    this.code = code;
    this.package = packageName;
    this.manifest = manifest;
    this.encoding = encoding;
  }
}

/**
 * A package whose own manifest resolves the package itself as a dependency.
 *
 * The conformance contract rejects this as `DEPENDENCY_SELF_EDGE`: a
 * self-edge is neither dropped nor drawn (a drawn self-loop would surface
 * later as a cycle, far from its cause). A Cargo manifest can do it with
 * nothing more than a path entry named after its own directory or its
 * `[package]` name. `manifest` is repository-relative, so the message never
 * discloses the checkout root.
 */
export class DependencySelfEdgeError extends Error {
  readonly code = "DEPENDENCY_SELF_EDGE";
  readonly package: string;
  readonly manifest: string;
  readonly dependency: string;

  constructor(packageName: string, manifest: string, dependency: string) {
    super(
      `DEPENDENCY_SELF_EDGE: package=${packageName} manifest=${manifest} dependency=${dependency}`,
    );
    this.name = "DependencySelfEdgeError";
    this.package = packageName;
    this.manifest = manifest;
    this.dependency = dependency;
  }
}

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
//
// Which Cargo tables are build-graph edges
// ----------------------------------------
//
// A graph edge means "rebuild and re-test the dependent when this changes",
// so it must cover everything Cargo compiles INTO the crate:
//
//     table                                 edge?  why
//     ------------------------------------  -----  ----------------------------
//     [dependencies]                        yes    linked into the library
//     [build-dependencies]                  yes    compiled into build.rs
//     [build_dependencies]                  yes    legacy spelling of the above
//     [target.<cfg>.<one of the above>]     yes    union over every cfg: CI
//                                                  builds on several hosts
//     [dev-dependencies] (every form)       no     tests only; Cargo allows a
//                                                  dev-dependency cycle
//     [patch], [replace], [workspace], ...  no     not a dependency of the crate
//
// Both TOML spellings of a path dependency are read:
//
//     [dependencies]
//     logic-gates = { path = "../logic-gates" }          <- inline table
//     gates = { package = "logic-gates", path = "..." }  <- renamed
//
//     [dependencies.logic-gates]                         <- sub-table
//     path = "../logic-gates"
//
// The line procedure is the Rust paragraph of
// code/specs/build-tool-conformance.md. Every build-tool front implements
// it and the shared fixture resolution-rust-field-aware pins it. It is a
// line reader, not a TOML parser, on purpose: "anything it does not describe
// contributes nothing", so a reader that understood more TOML than the
// contract would draw edges the other fronts do not.

/**
 * The table names whose path entries link into, or build, the crate.
 * Compared exactly: TOML keys are case-sensitive, so `[Dependencies]` is
 * some other table.
 */
const CARGO_LINK_TABLE_NAMES = new Set([
  "dependencies",
  "build-dependencies",
  "build_dependencies",
]);

/** TOML's bare-key alphabet: ASCII letters, digits, `_` and `-`. */
const TOML_BARE_KEY = /^[A-Za-z0-9_-]+$/;

/**
 * How the lines under the current header are read:
 *
 *   ignored  not a link table; its lines are skipped
 *   inline   [D] or [target.C.D]: one entry per line
 *   sub      [D.N] or [target.C.D.N]: the path/package keys of entry N
 */
type CargoLinkTable =
  | { kind: "ignored" }
  | { kind: "inline" }
  | { kind: "sub"; key: string };

/**
 * Trim only spaces and tabs (`String.prototype.trim` also eats NBSP, ...).
 *
 * An index scan from each end, not `replace(/[ \t]+$/, "")`: that regex
 * retries from every character of an interior run of spaces and fails at
 * each one, which is quadratic in a long run between two other characters.
 */
export function trimSpacesAndTabs(text: string): string {
  const isBlank = (character: string): boolean =>
    character === " " || character === "\t";
  let start = 0;
  let end = text.length;
  while (start < end && isBlank(text[start])) {
    start++;
  }
  while (end > start && isBlank(text[end - 1])) {
    end--;
  }
  return text.slice(start, end);
}

/** Lower-case only A-Z, as the contract's matching step says. */
function asciiLowercase(text: string): string {
  return text.replace(/[A-Z]/g, (letter) => letter.toLowerCase());
}

/**
 * Index just past the string that opens at `text[start]` (a `"` or `'`), or
 * -1 if it is not closed on this line (strings never span lines here). In a
 * basic "..." string a backslash makes the next character part of the
 * string, so `\"` does not end it; a literal '...' string has no escapes.
 */
function tomlStringEnd(text: string, start: number): number {
  const quote = text[start];
  for (let index = start + 1; index < text.length; index++) {
    if (quote === '"' && text[index] === "\\") {
      index++;
    } else if (text[index] === quote) {
      return index + 1;
    }
  }
  return -1;
}

function isTomlQuote(character: string | undefined): boolean {
  return character === '"' || character === "'";
}

/**
 * Remove a TOML comment: everything from the first `#` outside a string.
 * An unclosed string runs to the end of the line.
 */
function stripTomlLineComment(line: string): string {
  for (let index = 0; index < line.length; index++) {
    if (isTomlQuote(line[index])) {
      const end = tomlStringEnd(line, index);
      if (end < 0) {
        return line;
      }
      index = end - 1;
    } else if (line[index] === "#") {
      return line.slice(0, index);
    }
  }
  return line;
}

/** Index of the first `target` outside every string, or -1. */
function indexTomlOutsideStrings(text: string, target: string): number {
  for (let index = 0; index < text.length; index++) {
    if (isTomlQuote(text[index])) {
      const end = tomlStringEnd(text, index);
      if (end < 0) {
        return -1;
      }
      index = end - 1;
    } else if (text[index] === target) {
      return index;
    }
  }
  return -1;
}

/**
 * Counts the `{` and `[` outside strings that are not closed again, over
 * text fed to it in pieces. Feeding "ab" and then "cd" leaves exactly the
 * state feeding "abcd" would, because the scanner keeps what a rescan of the
 * joined text would have to rediscover: whether it is inside a string (and
 * which delimiter closes it), and whether the previous character was a
 * backslash escaping the next one in a basic string. So a string still open
 * at the end of one line continues after the joining space, as it does in
 * the joined value:
 *
 *     feed('{ x = "q\\')   depth 1, inside "...", escape pending
 *     feed(" ")             the space is escaped: still inside the string
 *     feed('", p = 1 }')    the string closes, `}` closes the table: 0
 *
 * Zero (or less) means an inline value is complete:
 *
 *     { path = "../a", features = [   -> 2
 *     { path = "../a" }               -> 0
 *
 * Carrying this state is what keeps the continuation join linear: the
 * reader used to rescan the whole growing value for every appended line,
 * which is quadratic in a long entry.
 */
class TomlBracketScanner {
  depth = 0;
  private quote = ""; // the delimiter of the string being scanned, or ""
  private escape = false; // the previous character was a backslash in "..."

  feed(text: string): void {
    for (let index = 0; index < text.length; index++) {
      const character = text[index];
      if (this.escape) {
        this.escape = false;
      } else if (this.quote !== "") {
        if (this.quote === '"' && character === "\\") {
          this.escape = true;
        } else if (character === this.quote) {
          this.quote = "";
        }
      } else if (isTomlQuote(character)) {
        this.quote = character;
      } else if (character === "{" || character === "[") {
        this.depth++;
      } else if (character === "}" || character === "]") {
        this.depth--;
      }
    }
  }
}

/**
 * Split at every separator outside strings. With `nested`, a separator
 * inside `[...]` or `{...}` does not split either, which keeps a
 * `features = ["a", "b"]` array in one field. An unclosed string runs to the
 * end of the last part.
 */
function splitTomlOutsideStrings(
  text: string,
  separator: string,
  nested: boolean,
): string[] {
  const parts: string[] = [];
  let start = 0;
  let depth = 0;
  for (let index = 0; index < text.length; index++) {
    const character = text[index];
    if (isTomlQuote(character)) {
      const end = tomlStringEnd(text, index);
      if (end < 0) {
        break;
      }
      index = end - 1;
    } else if (nested && (character === "[" || character === "{")) {
      depth++;
    } else if (nested && (character === "]" || character === "}")) {
      depth--;
    } else if (character === separator && depth === 0) {
      parts.push(text.slice(start, index));
      start = index + 1;
    }
  }
  parts.push(text.slice(start));
  return parts;
}

/**
 * Decode one key or header segment: a bare key, or exactly one complete
 * string whose value is the text between its quotes, verbatim (escapes are
 * not decoded). `'cfg(unix)'` -> `cfg(unix)`; `"a" "b"` and `a.b` -> null.
 */
function tomlKeySegment(raw: string): string | null {
  const segment = trimSpacesAndTabs(raw);
  if (segment === "") {
    return null;
  }
  if (isTomlQuote(segment[0])) {
    return tomlStringEnd(segment, 0) === segment.length
      ? segment.slice(1, -1)
      : null;
  }
  return TOML_BARE_KEY.test(segment) ? segment : null;
}

/**
 * The content of a value that is exactly one non-empty string: `"1"` is
 * one; `"a" "b"`, `1`, `""` and `["x"]` are not.
 */
function tomlSingleString(raw: string): string | null {
  const value = trimSpacesAndTabs(raw);
  if (!isTomlQuote(value[0]) || tomlStringEnd(value, 0) !== value.length) {
    return null;
  }
  const content = value.slice(1, -1);
  return content === "" ? null : content;
}

/**
 * Decide how to read the lines under a header (already comment-stripped
 * and trimmed, starting with `[` and ending with `]`). The header is split
 * into dotted-key segments at every `.` OUTSIDE a string and each segment is
 * decoded, so a cfg header is one segment however much punctuation it
 * quotes:
 *
 *     [target.'cfg(any(target_os = "linux", windows))'.dependencies]
 *      target  C (one quoted segment)                   D
 *
 *     [target."thumbv8m.main-none-eabihf".dependencies.pi]  -> sub-table pi
 *     [target.x86_64-unknown-linux-gnu.build-dependencies]  -> bare triple
 *     ["dependencies"]                                      -> [dependencies]
 *
 * Exactly [D], [D.N], [target.C.D] and [target.C.D.N] are read; arrays of
 * tables, malformed headers and every other segment list are ignored.
 */
function classifyCargoLinkHeader(line: string): CargoLinkTable {
  if (line.startsWith("[[")) {
    return { kind: "ignored" }; // array of tables, e.g. [[bin]]
  }
  let segments: string[] = [];
  for (const raw of splitTomlOutsideStrings(line.slice(1, -1), ".", false)) {
    const segment = tomlKeySegment(raw);
    if (segment === null) {
      return { kind: "ignored" };
    }
    segments.push(segment);
  }
  if (segments.length >= 3 && segments[0] === "target") {
    segments = segments.slice(2); // drop `target` and the cfg, whatever it is
  }
  if (!CARGO_LINK_TABLE_NAMES.has(segments[0])) {
    return { kind: "ignored" };
  }
  if (segments.length === 1) {
    return { kind: "inline" };
  }
  if (segments.length === 2) {
    return { kind: "sub", key: segments[1] };
  }
  return { kind: "ignored" };
}

/**
 * The crate a complete inline entry `key = value` names, if it is a path
 * dependency (the only kind that can point at a repository package):
 *
 *     logic-gates = { path = "../logic-gates" }                    -> logic-gates
 *     gates = { package = "logic-gates", path = "../logic-gates" } -> logic-gates
 *     serde = "1"                                -> null (not a table)
 *     x = { version = "1", features = ["path"] } -> null (no path field)
 *     foo.path = "../foo"                        -> null (dotted key)
 *
 * Fields are split at commas outside strings AND outside nested `[...]` or
 * `{...}`, so a `features` array cannot leak a fake field and a nested
 * table's `path` is not the entry's. A field key is decoded like an entry
 * key, so `"path"` and `'path'` are the key `path` (TOML, and so Cargo, read
 * them that way); the decoded key must then be exactly `path` or `package`
 * (Cargo ignores `Package`; escapes are not decoded). The first usable field
 * of each wins:
 *
 *     a = { "path" = "../a" }                 -> a
 *     x = { 'package' = "a", path = "../a" }  -> a
 *     a = { "Path" = "../a" }                 -> null (keys are case-sensitive)
 *     a = { path.x = "../a" }                 -> null (dotted field key)
 */
function cargoPathDependencyName(key: string, rawValue: string): string | null {
  const crateName = tomlKeySegment(key);
  if (crateName === null) {
    return null;
  }
  const value = trimSpacesAndTabs(rawValue);
  if (value.length < 2 || !value.startsWith("{") || !value.endsWith("}")) {
    return null;
  }
  let dependencyPath: string | null = null;
  let packageName: string | null = null;
  for (const field of splitTomlOutsideStrings(value.slice(1, -1), ",", true)) {
    const equals = indexTomlOutsideStrings(field, "=");
    if (equals < 0) {
      continue;
    }
    const fieldValue = tomlSingleString(field.slice(equals + 1));
    if (fieldValue === null) {
      continue;
    }
    // A dotted or malformed field key is no key at all (null).
    const fieldKey = tomlKeySegment(field.slice(0, equals));
    if (fieldKey === "path" && dependencyPath === null) {
      dependencyPath = fieldValue;
    } else if (fieldKey === "package" && packageName === null) {
      packageName = fieldValue;
    }
  }
  if (dependencyPath === null) {
    return null;
  }
  return asciiLowercase(packageName ?? crateName);
}

/**
 * Return the lower-case crate name of every path dependency in a manifest's
 * link tables, in manifest order with duplicates kept (the caller collapses
 * them).
 *
 * It is one pass over the lines with a three-state machine. Every header
 * line -- even a malformed one, or an array of tables -- ends the current
 * table, and so does the end of the file; a sub-table's entry is complete
 * only then, because its `path` and `package` keys may come in either order.
 *
 * One inline entry may cover several lines. TOML 1.0 lets an ARRAY inside
 * an inline table span lines, and the repository writes exactly that:
 *
 *     coding_adventures_vault_pm_cli = { path = "../vault-pm-cli", features = [
 *         "crash-injection",
 *     ] }
 *
 * So while the value still has an unclosed `{` or `[`, the next lines are
 * joined onto it with one space (and are not looked at as headers or
 * entries). If the file ends first, the entry contributes nothing. The
 * brackets are counted over the joined value as one text -- a string left
 * open at the end of one line continues after the joining space -- but the
 * joined value is never rescanned: a TomlBracketScanner carries its state
 * from line to line and the pieces are joined once, so the join is linear
 * in the entry's length.
 */
export function cargoLinkDependencyNames(manifest: string): string[] {
  // Step 1: lines. Split at LF, drop one trailing CR, ignore a leading BOM.
  const text = manifest.startsWith("﻿") ? manifest.slice(1) : manifest;
  const lines = text.split("\n");
  const clean = (raw: string): string =>
    trimSpacesAndTabs(
      stripTomlLineComment(raw.endsWith("\r") ? raw.slice(0, -1) : raw),
    );

  const names: string[] = [];
  let table: CargoLinkTable = { kind: "ignored" };
  let subPath: string | null = null;
  let subPackage: string | null = null;
  const finishTable = (): void => {
    if (table.kind === "sub" && subPath !== null) {
      names.push(asciiLowercase(subPackage ?? table.key));
    }
  };

  for (let index = 0; index < lines.length; index++) {
    const line = clean(lines[index]);

    // Step 2: headers. Every header ends the current table.
    if (line.startsWith("[") && line.endsWith("]")) {
      finishTable();
      table = classifyCargoLinkHeader(line);
      subPath = null;
      subPackage = null;
      continue;
    }

    const equals = indexTomlOutsideStrings(line, "=");
    if (equals < 0) {
      continue;
    }
    if (table.kind === "inline") {
      // Step 3: inline entries, continued while a bracket is open.
      // The pieces are collected and joined once, and the scanner carries
      // its state from piece to piece, so the join is linear.
      let value: string | null = trimSpacesAndTabs(line.slice(equals + 1));
      if (value.startsWith("{")) {
        const scanner = new TomlBracketScanner();
        scanner.feed(value);
        if (scanner.depth > 0) {
          const pieces = [value];
          let closed = false;
          while (!closed && index + 1 < lines.length) {
            index++;
            const piece = clean(lines[index]);
            scanner.feed(" ");
            scanner.feed(piece);
            pieces.push(piece);
            closed = scanner.depth <= 0;
          }
          // If the file ended inside the value, there is no entry.
          value = closed ? pieces.join(" ") : null;
        }
      }
      const crateName = value === null
        ? null
        : cargoPathDependencyName(line.slice(0, equals), value);
      if (crateName !== null) {
        names.push(crateName);
      }
    } else if (table.kind === "sub") {
      // Step 4: only the keys `path` and `package` matter, bare or quoted
      // (`"path" = ...` is the same key); first wins. `version`,
      // `features`, `optional` ... are ignored, and so is `Path`: TOML keys
      // are case-sensitive.
      const fieldValue = tomlSingleString(line.slice(equals + 1));
      if (fieldValue === null) {
        continue;
      }
      const fieldKey = tomlKeySegment(line.slice(0, equals));
      if (fieldKey === "path" && subPath === null) {
        subPath = fieldValue;
      } else if (fieldKey === "package" && subPackage === null) {
        subPackage = fieldValue;
      }
    }
  }
  finishTable();
  return names;
}

/**
 * Extract internal dependencies from a Rust Cargo.toml file.
 *
 * Step 5 of the contract: each candidate name from every link table is
 * looked up among the known Cargo names (the directory name and the
 * declared `[package]` name), and duplicates -- the same crate in
 * `[dependencies]` and `[build-dependencies]`, say -- collapse to one
 * dependency. A `package` rename is authoritative: the entry key of a
 * renamed entry is not looked up.
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
  // A Set, not Array.includes: each membership test is O(1), so a manifest
  // naming many crates costs time linear in its size. Insertion order is
  // manifest order.
  const internalDeps = new Set<string>();
  for (const crateName of cargoLinkDependencyNames(text)) {
    const pkgName = knownNames.get(crateName);
    if (pkgName) {
      internalDeps.add(pkgName);
    }
  }
  return [...internalDeps];
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

  const rockspecFile = entries.filter((e) => e.endsWith(".rockspec")).sort()[0];
  if (!rockspecFile) {
    return [];
  }

  const rockspecPath = nodePath.join(pkg.path, rockspecFile);
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(
      fs.readFileSync(rockspecPath),
    );
  } catch (error) {
    if (error instanceof TypeError) {
      throw new MetadataEncodingError(
        "METADATA_INVALID_UTF8",
        pkg.name,
        repositoryManifestPath(rockspecPath),
        "UTF-8",
      );
    }
    throw error;
  }
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

/** Return a stable repository-relative metadata path without host details. */
function repositoryManifestPath(manifestPath: string): string {
  const parts = manifestPath.replaceAll("\\", "/").split("/");
  let canonicalStart = -1;
  for (let index = 0; index < parts.length - 1; index += 1) {
    if (
      parts[index] === "code"
      && (parts[index + 1] === "packages" || parts[index + 1] === "programs")
    ) {
      canonicalStart = index;
    }
  }
  return canonicalStart >= 0
    ? parts.slice(canonicalStart).join("/")
    : nodePath.basename(manifestPath);
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
  return buildKnownNamesForLanguage(packages, "");
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

  // `[ \t]*`, not `\s*`: with the `m` flag a `\s*` after `^` runs across
  // every following blank line from each line start and backtracks, which
  // is quadratic in a run of blank lines. Names never span lines either.
  const match = fs
    .readFileSync(cargoTomlPath, "utf-8")
    .match(/^[ \t]*name[ \t]*=[ \t]*"([^"\n]+)"/m);
  return match ? match[1].trim().toLowerCase() : null;
}

function buildKnownNamesForLanguage(
  packages: Package[],
  language: string,
): Map<string, string> {
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
    if (language && !inDependencyScope(pkg.language, scope)) {
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

      case "rust": {
        // Rust crate names use the directory name directly (kebab-case).
        const crateName = dirName.toLowerCase();
        setKnown(crateName, pkg.name, pkg.path, pkg.language);
        const cargoName = readCargoPackageName(pkg.path);
        if (cargoName) {
          setKnown(cargoName, pkg.name, pkg.path, pkg.language);
        }
        break;
      }

      case "wasm": {
        // WASM wrappers should resolve via their explicit Cargo package names,
        // not by claiming the underlying Rust crate's bare directory name.
        const cargoName = readCargoPackageName(pkg.path);
        if (cargoName) {
          setKnown(cargoName, pkg.name, pkg.path, pkg.language);
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
 * @throws MetadataEncodingError if a rockspec is not valid UTF-8.
 * @throws DependencySelfEdgeError if a Cargo manifest resolves its own crate.
 */
export function resolveDependencies(packages: Package[]): DirectedGraph {
  const graph = new DirectedGraph();

  // First, add all packages as nodes. Even packages with no dependencies
  // need to be in the graph so they appear in independentGroups().
  for (const pkg of packages) {
    graph.addNode(pkg.name);
  }

  // Build the name-mapping table.
  const knownNamesByLanguage = new Map<string, Map<string, string>>();
  for (const pkg of packages) {
    if (!knownNamesByLanguage.has(pkg.language)) {
      knownNamesByLanguage.set(
        pkg.language,
        buildKnownNamesForLanguage(packages, pkg.language),
      );
    }
  }

  // Parse dependencies for each package.
  for (const pkg of packages) {
    const knownNames = knownNamesByLanguage.get(pkg.language) ?? new Map<string, string>();
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
        // Step 5 of the Rust contract: a candidate resolving to the crate
        // itself (by directory alias or [package] name) is rejected,
        // neither dropped nor drawn.
        if (deps.includes(pkg.name)) {
          throw new DependencySelfEdgeError(
            pkg.name,
            repositoryManifestPath(nodePath.join(pkg.path, "Cargo.toml")),
            pkg.name,
          );
        }
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
