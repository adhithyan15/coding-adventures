/**
 * scaffold-generator -- Generate CI-ready package scaffolding.
 *
 * === What This Program Does ===
 *
 * This tool generates correctly-structured, CI-ready package directories
 * for the coding-adventures monorepo. It supports the repo's current
 * scaffoldable language set: Python, Go, Ruby, TypeScript, Rust, Elixir,
 * Perl, Lua, Swift, Haskell, C#, and F#.
 *
 * === Why This Tool Exists ===
 *
 * The lessons.md file documents 12+ recurring categories of CI failures
 * caused by agents hand-crafting packages inconsistently:
 *
 *   - Missing BUILD files
 *   - TypeScript "main" pointing to dist/ instead of src/
 *   - Missing transitive dependency installs in BUILD files
 *   - Ruby require ordering (deps before own modules)
 *   - Rust workspace Cargo.toml not updated
 *   - Missing README.md or CHANGELOG.md
 *
 * This tool eliminates those failures. Run it, get a package that
 * compiles, lints, and passes tests. Then fill in the business logic.
 *
 * === How CLI Builder Powers This ===
 *
 * The entire CLI interface is defined in `scaffold-generator.json`. This
 * program never parses a single argument by hand. CLI Builder handles
 * all parsing, validation, and help generation.
 *
 * @module scaffold-generator
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { Parser, ParseErrors } from "@coding-adventures/cli-builder";
import type { ParseResult, HelpResult, VersionResult } from "@coding-adventures/cli-builder";

// =========================================================================
// Constants
// =========================================================================

/**
 * All languages supported by the scaffold generator.
 *
 * These match the directory names under `code/packages/<lang>/` and
 * `code/programs/<lang>/` in the monorepo.
 */
export const VALID_LANGUAGES = [
  "python",
  "go",
  "ruby",
  "typescript",
  "rust",
  "elixir",
  "perl",
  "lua",
  "swift",
  "haskell",
  "csharp",
  "fsharp",
] as const;

/**
 * A kebab-case name consists of lowercase letters and digits, with
 * segments separated by single hyphens. Examples:
 *
 *   - "logic-gates"  -- valid
 *   - "my-package"   -- valid
 *   - "MyPackage"    -- INVALID (uppercase)
 *   - "-leading"     -- INVALID (leading hyphen)
 *   - "trailing-"    -- INVALID (trailing hyphen)
 */
export const KEBAB_RE = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;

/**
 * Escape text before embedding it in XML element content or attributes.
 */
export function escapeXml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

// =========================================================================
// Name normalization
// =========================================================================
//
// The input package name is always kebab-case (e.g., "my-package"). Each
// language has different naming conventions. These functions convert between
// them.

/**
 * Convert "my-package" to "my_package" (Python, Ruby, Elixir convention).
 *
 * Snake case replaces each hyphen with an underscore. This is used for:
 *   - Python module names (import my_package)
 *   - Ruby gem names (coding_adventures_my_package)
 *   - Elixir app names (coding_adventures_my_package)
 *   - Ruby/Elixir directory names
 */
export function toSnakeCase(kebab: string): string {
  return kebab.replace(/-/g, "_");
}

/**
 * Convert "my-package" to "MyPackage" (Ruby module, Elixir module convention).
 *
 * Each hyphen-separated segment is capitalized and joined together.
 * For example:
 *   - "logic-gates"     -> "LogicGates"
 *   - "cli-builder"     -> "CliBuilder"
 *   - "state-machine"   -> "StateMachine"
 */
export function toCamelCase(kebab: string): string {
  return kebab
    .split("-")
    .map((part) => (part.length > 0 ? part[0].toUpperCase() + part.slice(1) : ""))
    .join("");
}

/**
 * Convert "my-package" to "mypackage" (Go package name convention).
 *
 * Go package names must be a single lowercase word. By convention,
 * multi-word package names simply concatenate the words without any
 * separator.
 */
export function toJoinedLower(kebab: string): string {
  return kebab.replace(/-/g, "");
}

/**
 * Return the directory name for a package in a given language.
 *
 * Ruby and Elixir use snake_case directories; all other languages
 * keep the original kebab-case name.
 *
 *   dirName("logic-gates", "ruby")   -> "logic_gates"
 *   dirName("logic-gates", "python") -> "logic-gates"
 */
export function dirName(kebab: string, lang: string): string {
  if (lang === "ruby" || lang === "elixir") {
    return toSnakeCase(kebab);
  }
  return kebab;
}

/**
 * Resolve the absolute directory path for a dependency.
 * Dependencies may live in either 'packages' or 'programs'.
 * 'packages' is preferred since it is the standard module location.
 */
export function resolveDepDir(repoRoot: string, lang: string, dep: string): string {
  const dName = dirName(dep, lang);
  const pkgDir = path.join(repoRoot, "code", "packages", lang, dName);
  const progDir = path.join(repoRoot, "code", "programs", lang, dName);
  if (fs.existsSync(pkgDir)) {
    return pkgDir;
  }
  if (fs.existsSync(progDir)) {
    return progDir;
  }
  return pkgDir; // Default to packages for clearer error messages
}

// =========================================================================
// Dependency resolution
// =========================================================================
//
// The scaffold generator reads existing packages' metadata to discover
// their dependencies, then computes the transitive closure and
// topological sort. This is the most critical feature -- missing
// transitive deps in BUILD files is the #1 CI failure category.

/**
 * Read direct local dependencies of a package by parsing its metadata
 * files. Returns dependency names in kebab-case.
 *
 * Each language stores dependency information differently:
 *   - Python: BUILD file with `-e ../dep` entries
 *   - Go: go.mod with `=> ../dep` replace directives
 *   - Ruby: Gemfile with `path: "../dep"` entries
 *   - TypeScript: package.json with `"file:../dep"` values
 *   - Rust: Cargo.toml with `path = "../dep"` entries
 *   - Elixir: mix.exs with `path: "../dep"` entries
 *   - C# / F#: .csproj / .fsproj with `<ProjectReference Include="../dep/...">`
 */
export function readDeps(pkgDir: string, lang: string): string[] {
  const readers: Record<string, (dir: string) => string[]> = {
    python: readPythonDeps,
    go: readGoDeps,
    ruby: readRubyDeps,
    typescript: readTypeScriptDeps,
    rust: readRustDeps,
    elixir: readElixirDeps,
    perl: readPerlDeps,
    lua: () => [],
    swift: () => [],
    haskell: readHaskellDeps,
    csharp: readDotnetDeps,
    fsharp: readDotnetDeps,
  };
  const reader = readers[lang];
  if (!reader) {
    return [];
  }
  return reader(pkgDir);
}

/**
 * Parse BUILD file for `-e ../` entries (Python).
 *
 * Python BUILD files contain `pip install -e ../dep-name` lines.
 * We extract the path after `../` as the dependency name.
 */
function readPythonDeps(pkgDir: string): string[] {
  const buildPath = path.join(pkgDir, "BUILD");
  if (!fs.existsSync(buildPath)) {
    return [];
  }
  const content = fs.readFileSync(buildPath, "utf-8");
  const deps: string[] = [];
  for (const line of content.split("\n")) {
    // Find ALL -e ../ entries on each line (new format puts them all on one line)
    let remaining = line;
    while (true) {
      let idx = remaining.indexOf("-e ../");
      if (idx < 0) {
        idx = remaining.indexOf('-e "../');
      }
      if (idx < 0) {
        break;
      }
      let rest = remaining.slice(idx);
      if (rest.startsWith('-e "../')) {
        rest = rest.slice(7); // skip `-e "../`
      } else {
        rest = rest.slice(6); // skip `-e ../`
      }
      let dep = "";
      for (const c of rest) {
        if (c === " " || c === '"' || c === "'" || c === "\n") {
          break;
        }
        dep += c;
      }
      if (dep && dep !== ".") {
        deps.push(dep);
      }
      remaining = remaining.slice(idx + 6);
    }
  }
  return deps;
}

/**
 * Parse go.mod replace directives for ../dep paths (Go).
 *
 * Go modules use `replace` directives to point to local sibling
 * packages: `replace github.com/.../dep => ../dep`.
 */
function readGoDeps(pkgDir: string): string[] {
  const modPath = path.join(pkgDir, "go.mod");
  if (!fs.existsSync(modPath)) {
    return [];
  }
  const content = fs.readFileSync(modPath, "utf-8");
  const deps: string[] = [];
  for (const line of content.split("\n")) {
    if (line.includes("=> ../")) {
      const idx = line.indexOf("=> ../");
      const rest = line.slice(idx + 6).trim();
      const dep = rest.split(/\s/)[0] || "";
      if (dep) {
        deps.push(dep);
      }
    }
  }
  return deps;
}

/**
 * Parse Gemfile for path: '../dep' entries (Ruby).
 *
 * Ruby Gemfiles declare local dependencies with path:
 *   gem "coding_adventures_logic_gates", path: "../logic_gates"
 *
 * The directory name uses snake_case, so we convert back to kebab-case.
 */
function readRubyDeps(pkgDir: string): string[] {
  const gemfilePath = path.join(pkgDir, "Gemfile");
  if (!fs.existsSync(gemfilePath)) {
    return [];
  }
  const content = fs.readFileSync(gemfilePath, "utf-8");
  const deps: string[] = [];
  for (const line of content.split("\n")) {
    if (line.includes("path:") && line.includes('"../')) {
      const idx = line.indexOf('"../');
      const rest = line.slice(idx + 4);
      let dep = "";
      for (const c of rest) {
        if (c === '"') {
          break;
        }
        dep += c;
      }
      // Convert snake_case directory name back to kebab-case
      dep = dep.replace(/_/g, "-");
      if (dep) {
        deps.push(dep);
      }
    }
  }
  return deps;
}

/**
 * Parse package.json dependencies with file:../ values (TypeScript).
 *
 * TypeScript packages declare local dependencies in package.json:
 *   "@coding-adventures/dep": "file:../dep"
 *
 * We extract the path after `file:../` as the dependency name.
 */
function readTypeScriptDeps(pkgDir: string): string[] {
  const pkgJsonPath = path.join(pkgDir, "package.json");
  if (!fs.existsSync(pkgJsonPath)) {
    return [];
  }
  let pkg: Record<string, unknown>;
  try {
    pkg = JSON.parse(fs.readFileSync(pkgJsonPath, "utf-8")) as Record<string, unknown>;
  } catch {
    return [];
  }
  const depsObj = (pkg["dependencies"] ?? {}) as Record<string, string>;
  const deps: string[] = [];
  for (const val of Object.values(depsObj)) {
    if (typeof val === "string" && val.startsWith("file:../")) {
      const dep = val.slice("file:../".length);
      if (dep) {
        deps.push(dep);
      }
    }
  }
  return deps;
}

/**
 * Parse Cargo.toml for path = '../dep' entries (Rust).
 *
 * Rust crates declare local dependencies in Cargo.toml:
 *   dep = { path = "../dep" }
 */
function readRustDeps(pkgDir: string): string[] {
  const cargoPath = path.join(pkgDir, "Cargo.toml");
  if (!fs.existsSync(cargoPath)) {
    return [];
  }
  const content = fs.readFileSync(cargoPath, "utf-8");
  const deps: string[] = [];
  for (const line of content.split("\n")) {
    if (line.includes('path = "../')) {
      const idx = line.indexOf('path = "../');
      const rest = line.slice(idx + 11);
      let dep = "";
      for (const c of rest) {
        if (c === '"') {
          break;
        }
        dep += c;
      }
      if (dep) {
        deps.push(dep);
      }
    }
  }
  return deps;
}

/**
 * Parse mix.exs for path: '../dep' entries (Elixir).
 *
 * Elixir packages declare local dependencies in mix.exs:
 *   {:coding_adventures_logic_gates, path: "../logic_gates"}
 *
 * The directory name uses snake_case, so we convert back to kebab-case.
 */
function readElixirDeps(pkgDir: string): string[] {
  const mixPath = path.join(pkgDir, "mix.exs");
  if (!fs.existsSync(mixPath)) {
    return [];
  }
  const content = fs.readFileSync(mixPath, "utf-8");
  const deps: string[] = [];
  for (const line of content.split("\n")) {
    if (line.includes('path: "../')) {
      const idx = line.indexOf('path: "../');
      const rest = line.slice(idx + 10);
      let dep = "";
      for (const c of rest) {
        if (c === '"') {
          break;
        }
        dep += c;
      }
      // Convert snake_case directory name back to kebab-case
      dep = dep.replace(/_/g, "-");
      if (dep) {
        deps.push(dep);
      }
    }
  }
  return deps;
}

function readPerlDeps(pkgDir: string): string[] {
  const cpanfilePath = path.join(pkgDir, "cpanfile");
  if (!fs.existsSync(cpanfilePath)) {
    return [];
  }
  const content = fs.readFileSync(cpanfilePath, "utf-8");
  const deps: string[] = [];
  const re = /requires\s+['"]coding-adventures-([^'"]+)['"]/;
  for (const line of content.split("\n")) {
    if (line.trim().startsWith("#")) continue;
    const m = line.match(re);
    if (m) {
      deps.push(m[1]);
    }
  }
  return deps;
}

export function readHaskellDeps(pkgDir: string): string[] {
  let files: string[];
  try {
    files = fs.readdirSync(pkgDir);
  } catch {
    return [];
  }
  const cabalFile = files.find((f) => f.endsWith(".cabal"));
  if (!cabalFile) {
    return [];
  }
  const content = fs.readFileSync(path.join(pkgDir, cabalFile), "utf-8");
  const deps: string[] = [];
  const re = /coding-adventures-([a-zA-Z0-9-]+)/;
  const selfName = path.basename(pkgDir);
  for (const line of content.split("\n")) {
    if (line.includes("name:") || line.includes("executable") || line.includes("library")) {
      continue;
    }
    const m = line.match(re);
    if (m && m[1] && m[1] !== selfName) {
      deps.push(m[1]);
    }
  }
  return deps;
}

export function readDotnetDeps(pkgDir: string): string[] {
  let files: string[];
  try {
    files = fs.readdirSync(pkgDir);
  } catch {
    return [];
  }
  const projectFile = files.find(
    (file) => file.endsWith(".csproj") || file.endsWith(".fsproj"),
  );
  if (!projectFile) {
    return [];
  }
  const content = fs.readFileSync(path.join(pkgDir, projectFile), "utf-8");
  const deps: string[] = [];
  const re =
    /<ProjectReference\s+Include\s*=\s*"\.\.[\\/]([^/\\"]+)[\\/][^"]+"/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(content)) !== null) {
    const depDir = match[1]?.trim();
    if (
      depDir &&
      depDir !== "." &&
      depDir !== ".." &&
      !depDir.includes("/") &&
      !depDir.includes("\\")
    ) {
      deps.push(depDir.replace(/_/g, "-"));
    }
  }
  return deps;
}

// =========================================================================
// Transitive closure (BFS)
// =========================================================================
//
// Given a list of direct dependencies, we expand them by reading each
// dependency's own dependencies, and their dependencies, and so on.
// This is a breadth-first search through the dependency graph.
//
// Why BFS? Because we need ALL transitive dependencies, not just the
// shortest path. BFS naturally explores every reachable node exactly once.

/**
 * Compute all transitive dependencies via BFS.
 *
 * Starting from the direct dependencies, we read each dependency's own
 * deps, adding any new ones to the queue. The result is sorted
 * alphabetically for deterministic output.
 *
 * @param directDeps - The direct dependencies (kebab-case names)
 * @param lang - The target language
 * @param baseDir - The directory containing all sibling packages
 * @returns Sorted list of all transitive dependency names
 */
export function transitiveClosure(
  directDeps: string[],
  lang: string,
  repoRoot: string,
): string[] {
  const visited = new Set<string>();
  const queue = [...directDeps];

  while (queue.length > 0) {
    const dep = queue.shift()!;
    if (visited.has(dep)) {
      continue;
    }
    visited.add(dep);
    const depDir = resolveDepDir(repoRoot, lang, dep);
    for (const dd of readDeps(depDir, lang)) {
      if (!visited.has(dd)) {
        queue.push(dd);
      }
    }
  }

  return [...visited].sort();
}

// =========================================================================
// Topological sort (Kahn's algorithm)
// =========================================================================
//
// After computing all transitive dependencies, we need to determine the
// order in which to install them. A dependency must be installed BEFORE
// anything that depends on it. This is a topological ordering.
//
// We use Kahn's algorithm:
//   1. Build a graph: for each dep, list what it depends on (within our set).
//   2. Compute in-degrees (how many deps each node has within the set).
//   3. Start with "leaves" -- nodes with in-degree 0.
//   4. Remove a leaf, decrement in-degrees of nodes that depended on it.
//   5. Repeat until all nodes are processed.
//
// If we can't process all nodes, there's a cycle.

/**
 * Return dependencies in leaf-first (install) order via Kahn's algorithm.
 *
 * @param allDeps - All transitive dependencies (from transitiveClosure)
 * @param lang - The target language
 * @param baseDir - The directory containing all sibling packages
 * @returns Dependencies in topological order (install leaves first)
 * @throws Error if a circular dependency is detected
 */
export function topologicalSort(
  allDeps: string[],
  lang: string,
  repoRoot: string,
): string[] {
  const depSet = new Set(allDeps);

  // Build graph: for each dep, list what it depends on (within our set)
  const graph: Record<string, string[]> = {};
  for (const dep of allDeps) {
    graph[dep] = [];
    const depDir = resolveDepDir(repoRoot, lang, dep);
    for (const dd of readDeps(depDir, lang)) {
      if (depSet.has(dd)) {
        graph[dep].push(dd);
      }
    }
  }

  // In-degree: how many deps does this node have within the set
  const inDegree: Record<string, number> = {};
  for (const dep of allDeps) {
    inDegree[dep] = graph[dep].length;
  }

  // Start with leaves (in-degree === 0)
  const queue = allDeps.filter((dep) => inDegree[dep] === 0).sort();
  const result: string[] = [];

  while (queue.length > 0) {
    const node = queue.shift()!;
    result.push(node);
    // Decrease in-degree for nodes that depend on this one
    for (const dep of allDeps) {
      if (graph[dep].includes(node)) {
        inDegree[dep] -= 1;
        if (inDegree[dep] === 0) {
          queue.push(dep);
          queue.sort();
        }
      }
    }
  }

  if (result.length !== allDeps.length) {
    throw new Error(
      `circular dependency detected: resolved ${result.length} of ${allDeps.length}`,
    );
  }

  return result;
}

// =========================================================================
// File generation
// =========================================================================
//
// Each language has a generator function that creates the package files
// from templates. The templates match the Go and Python reference
// implementations exactly.

/**
 * Write content to a file, creating parent directories as needed.
 */
function writeFile(filePath: string, content: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf-8");
}

/**
 * Get today's date in ISO format (YYYY-MM-DD).
 */
function todayISO(): string {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

// -------------------------------------------------------------------------
// Python generator
// -------------------------------------------------------------------------

/**
 * Generate Python package files.
 *
 * Creates:
 *   - pyproject.toml (hatchling build system, ruff linting, pytest config)
 *   - src/<snake>/__init__.py (package entry point)
 *   - tests/__init__.py (empty, makes tests a package)
 *   - tests/test_<snake>.py (version test)
 *   - BUILD (install deps in order, then test)
 */
export function generatePython(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  orderedDeps: string[],
): void {
  const snake = toSnakeCase(pkgName);

  const pyproject = `[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "coding-adventures-${pkgName}"
version = "0.1.0"
description = "${description}"
requires-python = ">=3.12"
license = "MIT"
authors = [{ name = "Adhithya Rajasekaran" }]
readme = "README.md"

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-cov>=5.0", "ruff>=0.4", "mypy>=1.10"]

[tool.hatch.build.targets.wheel]
packages = ["src/${snake}"]

[tool.ruff]
target-version = "py312"
line-length = 88

[tool.ruff.lint]
select = ["E", "W", "F", "I", "UP", "B", "SIM", "ANN"]

[tool.pytest.ini_options]
testpaths = ["tests"]
addopts = "--cov=${snake} --cov-report=term-missing --cov-fail-under=80"

[tool.coverage.run]
source = ["src/${snake}"]

[tool.coverage.report]
fail_under = 80
show_missing = true
`;

  const initPy = `"""${pkgName} \u2014 ${description}

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
${layerCtx}"""

__version__ = "0.1.0"
`;

  const testPy = `"""Tests for ${pkgName}."""

from ${snake} import __version__


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"
`;

  const installParts = ["python -m pip install"];
  for (const dep of orderedDeps) {
    installParts.push(`-e ../${dep}`);
  }
  installParts.push("-e .[dev]", "--quiet");
  const buildLines = [installParts.join(" ")];
  buildLines.push("python -m pytest tests/ -v");
  const build = buildLines.join("\n") + "\n";

  const buildWinLines = ["uv venv --quiet --clear"];
  if (orderedDeps.length > 0) {
    const depFlags = orderedDeps.map((dep) => `-e ../${dep}`).join(" ");
    buildWinLines.push(`uv pip install ${depFlags} --quiet`);
  }
  buildWinLines.push("uv pip install --no-deps -e .[dev] --quiet");
  buildWinLines.push("uv pip install pytest pytest-cov ruff mypy --quiet");
  buildWinLines.push("uv run --no-project python -m pytest tests/ -v");
  const buildWindows = buildWinLines.join("\n") + "\n";

  writeFile(path.join(targetDir, "pyproject.toml"), pyproject);
  writeFile(path.join(targetDir, "src", snake, "__init__.py"), initPy);
  writeFile(path.join(targetDir, "tests", "__init__.py"), "");
  writeFile(path.join(targetDir, "tests", `test_${snake}.py`), testPy);
  writeFile(path.join(targetDir, "BUILD"), build);
  writeFile(path.join(targetDir, "BUILD_windows"), buildWindows);
}

// -------------------------------------------------------------------------
// Go generator
// -------------------------------------------------------------------------

/**
 * Generate Go package files.
 *
 * Creates:
 *   - go.mod (module declaration with require/replace for deps)
 *   - <snake>.go (package source with doc comment)
 *   - <snake>_test.go (basic load test)
 *   - BUILD (go test command)
 */
export function generateGo(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  allDeps: string[],
): void {
  const goPkg = toJoinedLower(pkgName);
  const snake = toSnakeCase(pkgName);

  let goMod = `module github.com/adhithyan15/coding-adventures/code/packages/go/${pkgName}\n\ngo 1.26\n`;
  if (directDeps.length > 0) {
    goMod += "\nrequire (\n";
    for (const dep of directDeps) {
      goMod += `\tgithub.com/adhithyan15/coding-adventures/code/packages/go/${dep} v0.0.0\n`;
    }
    goMod += ")\n\nreplace (\n";
    for (const dep of allDeps) {
      goMod += `\tgithub.com/adhithyan15/coding-adventures/code/packages/go/${dep} => ../${dep}\n`;
    }
    goMod += ")\n";
  }

  const src = `// Package ${goPkg} provides ${description}.
//
// This package is part of the coding-adventures monorepo, a ground-up
// implementation of the computing stack from transistors to operating systems.
// ${layerCtx}
package ${goPkg}
`;

  const test = `package ${goPkg}

import "testing"

func TestPackageLoads(t *testing.T) {
\tt.Log("${pkgName} package loaded successfully")
}
`;

  writeFile(path.join(targetDir, "go.mod"), goMod);
  writeFile(path.join(targetDir, `${snake}.go`), src);
  writeFile(path.join(targetDir, `${snake}_test.go`), test);
  writeFile(path.join(targetDir, "BUILD"), "go test ./... -v -cover\n");
}

// -------------------------------------------------------------------------
// Ruby generator
// -------------------------------------------------------------------------

/**
 * Generate Ruby package files.
 *
 * Creates:
 *   - coding_adventures_<snake>.gemspec (gem specification)
 *   - Gemfile (bundler config with path deps)
 *   - Rakefile (test task configuration)
 *   - lib/coding_adventures_<snake>.rb (entry point with require ordering)
 *   - lib/coding_adventures/<snake>/version.rb (version constant)
 *   - test/test_<snake>.rb (minitest)
 *   - BUILD (bundle install + rake test)
 *
 * IMPORTANT: Ruby require ordering matters! Dependencies must be required
 * BEFORE own modules, or constant resolution will fail at runtime. This
 * is the #2 CI failure category for Ruby packages.
 */
export function generateRuby(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  allDeps: string[],
): void {
  const snake = toSnakeCase(pkgName);
  const camel = toCamelCase(pkgName);

  let gemspec = `# frozen_string_literal: true

require_relative "lib/coding_adventures/${snake}/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_${snake}"
  spec.version       = CodingAdventures::${camel}::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "${description}"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }

`;
  for (const dep of directDeps) {
    const depSnake = toSnakeCase(dep);
    gemspec += `  spec.add_dependency "coding_adventures_${depSnake}", "~> 0.1"\n`;
  }
  gemspec += '  spec.add_development_dependency "minitest", "~> 5.0"\n';
  gemspec += '  spec.add_development_dependency "rake", "~> 13.0"\nend\n';

  let gemfile =
    '# frozen_string_literal: true\n\nsource "https://rubygems.org"\ngemspec\n';
  if (allDeps.length > 0) {
    gemfile += "\n# All transitive path dependencies.\n";
    for (const dep of allDeps) {
      const depSnake = toSnakeCase(dep);
      gemfile += `gem "coding_adventures_${depSnake}", path: "../${depSnake}"\n`;
    }
  }

  const rakefile = `# frozen_string_literal: true

require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/test_*.rb"]
end

task default: :test
`;

  // Entry point: deps FIRST, then own modules (Ruby require ordering!)
  let entry = "# frozen_string_literal: true\n\n";
  if (directDeps.length > 0) {
    entry += "# IMPORTANT: Require dependencies FIRST, before own modules.\n";
    for (const dep of directDeps) {
      const depSnake = toSnakeCase(dep);
      entry += `require "coding_adventures_${depSnake}"\n`;
    }
    entry += "\n";
  }
  entry += `require_relative "coding_adventures/${snake}/version"\n\n`;
  entry += `module CodingAdventures\n  # ${description}\n  module ${camel}\n  end\nend\n`;

  const versionRb = `# frozen_string_literal: true

module CodingAdventures
  module ${camel}
    VERSION = "0.1.0"
  end
end
`;

  const testRb = `# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_${snake}"

class Test${camel} < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::${camel}::VERSION
  end
end
`;

  writeFile(
    path.join(targetDir, `coding_adventures_${snake}.gemspec`),
    gemspec,
  );
  writeFile(path.join(targetDir, "Gemfile"), gemfile);
  writeFile(path.join(targetDir, "Rakefile"), rakefile);
  writeFile(
    path.join(targetDir, "lib", `coding_adventures_${snake}.rb`),
    entry,
  );
  writeFile(
    path.join(targetDir, "lib", "coding_adventures", snake, "version.rb"),
    versionRb,
  );
  writeFile(path.join(targetDir, "test", `test_${snake}.rb`), testRb);
  writeFile(
    path.join(targetDir, "BUILD"),
    "bundle install --quiet\nbundle exec rake test\n",
  );
}

// -------------------------------------------------------------------------
// TypeScript generator
// -------------------------------------------------------------------------

/**
 * Generate TypeScript package files.
 *
 * Creates:
 *   - package.json (ESM, vitest, file: deps)
 *   - tsconfig.json (strict, ESNext module)
 *   - vitest.config.ts (v8 coverage with 80% threshold)
 *   - src/index.ts (entry point with version export)
 *   - tests/<name>.test.ts (version test)
 *   - BUILD (chain-install transitive deps, then test)
 *
 * CRITICAL fields in package.json:
 *   - "main": "src/index.ts"  (NOT dist/index.js -- vitest needs TS source)
 *   - "type": "module"        (ESM, not CommonJS)
 *   - "@vitest/coverage-v8"   (must be in devDependencies)
 */
export function generateTypeScript(
  targetDir: string,
  pkgName: string,
  pkgType: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  orderedDeps: string[],
): void {
  if (pkgType === "program") {
    generateTypeScriptProgram(
      targetDir,
      pkgName,
      description,
      layerCtx,
      directDeps,
      orderedDeps,
    );
  } else {
    generateTypeScriptLibrary(
      targetDir,
      pkgName,
      description,
      layerCtx,
      directDeps,
      orderedDeps,
    );
  }
}

export function generateTypeScriptLibrary(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  orderedDeps: string[],
): void {
  let packageJson = '{\n  "name": "@coding-adventures/' + pkgName + '",\n';
  packageJson += '  "version": "0.1.0",\n';
  packageJson += `  "description": "${description}",\n`;
  packageJson += '  "type": "module",\n';
  packageJson += '  "main": "src/index.ts",\n';
  packageJson += '  "scripts": {\n';
  packageJson += '    "build": "tsc",\n';
  packageJson += '    "test": "vitest run",\n';
  packageJson += '    "test:coverage": "vitest run --coverage"\n';
  packageJson += "  },\n";
  packageJson += '  "author": "Adhithya Rajasekaran",\n';
  packageJson += '  "license": "MIT",\n';
  packageJson += '  "dependencies": {\n';
  if (directDeps.length > 0) {
    const entries = directDeps.map(
      (dep) => `    "@coding-adventures/${dep}": "file:../${dep}"`,
    );
    packageJson += entries.join(",\n") + "\n";
  }
  packageJson += "  },\n";
  packageJson += '  "devDependencies": {\n';
  packageJson += '    "typescript": "^5.0.0",\n';
  packageJson += '    "vitest": "^3.0.0",\n';
  packageJson += '    "@vitest/coverage-v8": "^3.0.0"\n';
  packageJson += "  }\n";
  packageJson += "}\n";

  const tsconfig = `{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true
  },
  "include": ["src"]
}
`;

  const vitestConfig = `import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
`;

  const indexTs = `/**
 * @coding-adventures/${pkgName}
 *
 * ${description}
 *
 * This package is part of the coding-adventures monorepo.
 * ${layerCtx}
 */

export const VERSION = "0.1.0";
`;

  const testTs = `import { describe, it, expect } from "vitest";
import { VERSION } from "../src/index.js";

describe("${pkgName}", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });
});
`;

  const build = "npm ci --quiet\nnpx vitest run --coverage\n";

  writeFile(path.join(targetDir, "package.json"), packageJson);
  writeFile(path.join(targetDir, "tsconfig.json"), tsconfig);
  writeFile(path.join(targetDir, "vitest.config.ts"), vitestConfig);
  writeFile(path.join(targetDir, "src", "index.ts"), indexTs);
  writeFile(path.join(targetDir, "tests", `${pkgName}.test.ts`), testTs);
  writeFile(path.join(targetDir, "BUILD"), build);
}

export function generateTypeScriptProgram(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  orderedDeps: string[],
): void {
  // package.json dependencies
  let depsJSON = "";
  if (directDeps.length > 0) {
    const depEntries = directDeps.map(
      (dep) => `    "@coding-adventures/${dep}": "file:../${dep}"`
    );
    depsJSON = depEntries.join(",\n");
  }

  const packageJSON = `{
  "name": "@coding-adventures/${pkgName}",
  "private": true,
  "version": "0.1.0",
  "description": "${description}",
  "type": "module",
  "main": "electron/main.ts",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage",
    "test:e2e": "playwright test",
    "electron:dev": "electron . --enable-logging",
    "electron:build": "vite build && electron-builder"
  },
  "author": "Adhithya Rajasekaran",
  "license": "MIT",
  "dependencies": {
${depsJSON}
  },
  "devDependencies": {
    "@playwright/test": "^1.50.1",
    "@testing-library/react": "^16.0.0",
    "@types/node": "^22.13.5",
    "@types/react": "^19.0.10",
    "@types/react-dom": "^19.0.4",
    "@vitejs/plugin-react": "^4.3.4",
    "@vitest/coverage-v8": "^3.0.0",
    "electron": "^34.3.0",
    "electron-builder": "^25.1.8",
    "jsdom": "^26.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "typescript": "^5.0.0",
    "vite": "^6.4.1",
    "vitest": "^3.0.0"
  },
  "build": {
    "appId": "com.coding-adventures.${pkgName}",
    "productName": "${toCamelCase(pkgName)} App",
    "directories": {
      "output": "dist-electron"
    },
    "files": [
      "dist/**/*",
      "electron/main.ts"
    ],
    "mac": {
      "category": "public.app-category.productivity"
    }
  }
}
`;

  const viteConfig = `import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "./",
});
`;

  const vitestConfig = `import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    exclude: ["e2e/**", "node_modules/**", "playwright-report/**"],
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
`;

  const playwrightConfig = `import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
    video: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "npm run dev",
    port: 5173,
    reuseExistingServer: !process.env.CI,
  },
});
`;

  const indexHtml = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${toCamelCase(pkgName)} App</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
`;

  const mainTsx = `import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
`;

  const appTsx = `export function App() {
  return (
    <div style={{ fontFamily: "sans-serif", padding: "2rem", textAlign: "center" }}>
      <h1>${toCamelCase(pkgName)} Web/Desktop App</h1>
      <p>Scaffolded by coding-adventures.</p>
    </div>
  );
}
`;

  const electronMainTs = `import { app, BrowserWindow } from "electron";
import * as path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function createWindow() {
  const win = new BrowserWindow({
    width: 1000,
    height: 800,
    webPreferences: {
      nodeIntegration: true,
      contextIsolation: false,
    },
  });

  const isDev = !app.isPackaged;

  if (isDev) {
    win.loadURL("http://localhost:5173");
  } else {
    win.loadFile(path.join(__dirname, "../dist/index.html"));
  }
}

app.whenReady().then(() => {
  createWindow();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
`;

  const electronTsConfig = `{
  "compilerOptions": {
    "target": "es2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "outDir": "../dist-electron"
  },
  "include": [
    "main.ts"
  ]
}
`;

  const tsConfig = `{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src",
    "declaration": true,
    "jsx": "react-jsx"
  },
  "include": ["src"]
}
`;

  const e2eTest = `import { test, expect } from "@playwright/test";

test("loads the application", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("h1")).toBeVisible();
});
`;

  const appTestTsx = `import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { App } from "../App";

describe("App", () => {
  it("renders the heading", () => {
    render(<App />);
    expect(screen.getByText(/${toCamelCase(pkgName)} Web\\/Desktop App/i)).toBeTruthy();
  });
});
`;

  // =========================================================================
  // Scaffolding: GitIgnore
  // =========================================================================
  // The .gitignore file ensures that any build artifacts or local dependency
  // caches (such as .npm-cache) are not erroneously committed by the user.
  //
  // +-----------------+       +-------------------+       +-----------------+
  // |  node_modules/  |       |  Playwright E2E   |       | Electron Build  |
  // |  .npm-cache/    | ----> |  test-results/    | ----> | dist/           |
  // |                 |       |                   |       | release/        |
  // +-----------------+       +-------------------+       +-----------------+
  //
  const gitignore = `node_modules/
dist/
dist-electron/
release/
.npm-cache/
.DS_Store
test-results/
playwright-report/
blob-report/
`;

  const build = "npm ci --quiet\\nnpx vitest run --coverage\\n";

  fs.mkdirSync(path.join(targetDir, "src", "__tests__"), { recursive: true });
  fs.mkdirSync(path.join(targetDir, "electron"), { recursive: true });
  fs.mkdirSync(path.join(targetDir, "e2e"), { recursive: true });

  writeFile(path.join(targetDir, "package.json"), packageJSON);
  writeFile(path.join(targetDir, "tsconfig.json"), tsConfig);
  writeFile(path.join(targetDir, "vite.config.ts"), viteConfig);
  writeFile(path.join(targetDir, "vitest.config.ts"), vitestConfig);
  writeFile(path.join(targetDir, "playwright.config.ts"), playwrightConfig);
  writeFile(path.join(targetDir, "index.html"), indexHtml);
  writeFile(path.join(targetDir, "src", "main.tsx"), mainTsx);
  writeFile(path.join(targetDir, "src", "App.tsx"), appTsx);
  writeFile(path.join(targetDir, "src", "__tests__", "App.test.tsx"), appTestTsx);
  writeFile(path.join(targetDir, "electron", "main.ts"), electronMainTs);
  writeFile(path.join(targetDir, "electron", "tsconfig.json"), electronTsConfig);
  writeFile(path.join(targetDir, "e2e", "app.spec.ts"), e2eTest);
  writeFile(path.join(targetDir, ".gitignore"), gitignore);
  writeFile(path.join(targetDir, "BUILD"), build);
}

// -------------------------------------------------------------------------
// Rust generator
// -------------------------------------------------------------------------

/**
 * Generate Rust package files.
 *
 * Creates:
 *   - Cargo.toml (crate metadata with path deps)
 *   - src/lib.rs (library entry point with load test)
 *   - BUILD (cargo test command)
 *
 * NOTE: The workspace Cargo.toml must also be updated to include
 * the new crate. The scaffoldOne function handles that separately.
 */
export function generateRust(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
): void {
  let cargo = `[package]
name = "${pkgName}"
version = "0.1.0"
edition = "2021"
description = "${description}"

[dependencies]
`;
  for (const dep of directDeps) {
    cargo += `${dep} = { path = "../${dep}" }\n`;
  }

  const libRs = `//! # ${pkgName}
//!
//! ${description}
//!
//! This crate is part of the coding-adventures monorepo.
//! ${layerCtx}

#[cfg(test)]
mod tests {
    #[test]
    fn it_loads() {
        assert!(true, "${pkgName} crate loaded successfully");
    }
}
`;

  writeFile(path.join(targetDir, "Cargo.toml"), cargo);
  writeFile(path.join(targetDir, "src", "lib.rs"), libRs);
  writeFile(
    path.join(targetDir, "BUILD"),
    `cargo test -p ${pkgName} -- --nocapture\n`,
  );
}

// -------------------------------------------------------------------------
// Elixir generator
// -------------------------------------------------------------------------

/**
 * Generate Elixir package files.
 *
 * Creates:
 *   - mix.exs (Mix project config with path deps)
 *   - lib/coding_adventures/<snake>.ex (module definition)
 *   - test/<snake>_test.exs (ExUnit test)
 *   - test/test_helper.exs (ExUnit.start())
 *   - BUILD (chain-compile transitive deps, then test)
 */
export function generateElixir(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  orderedDeps: string[],
): void {
  const snake = toSnakeCase(pkgName);
  const camel = toCamelCase(pkgName);

  let depsStr = "";
  for (let i = 0; i < directDeps.length; i++) {
    const depSnake = toSnakeCase(directDeps[i]);
    const comma = i < directDeps.length - 1 ? "," : "";
    depsStr += `      {:coding_adventures_${depSnake}, path: "../${depSnake}"}${comma}\n`;
  }

  const mixExs = `defmodule CodingAdventures.${camel}.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_${snake},
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
${depsStr}    ]
  end
end
`;

  const libEx = `defmodule CodingAdventures.${camel} do
  @moduledoc """
  ${description}

  This module is part of the coding-adventures monorepo.
  ${layerCtx}
  """
end
`;

  const testExs = `defmodule CodingAdventures.${camel}Test do
  use ExUnit.Case

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.${camel})
  end
end
`;

  let build: string;
  if (orderedDeps.length > 0) {
    const parts = orderedDeps.map(
      (dep) =>
        `cd ../${toSnakeCase(dep)} && mix deps.get --quiet && mix compile --quiet`,
    );
    parts.push(`cd ../${snake} && mix deps.get --quiet && mix test --cover`);
    build = parts.join(" && \\\n") + "\n";
  } else {
    build = "mix deps.get --quiet && mix test --cover\n";
  }

  writeFile(path.join(targetDir, "mix.exs"), mixExs);
  writeFile(
    path.join(targetDir, "lib", "coding_adventures", `${snake}.ex`),
    libEx,
  );
  writeFile(path.join(targetDir, "test", `${snake}_test.exs`), testExs);
  writeFile(path.join(targetDir, "test", "test_helper.exs"), "ExUnit.start()\n");
  writeFile(path.join(targetDir, "BUILD"), build);
}

// -------------------------------------------------------------------------
// Perl
// -------------------------------------------------------------------------

/**
 * Generate Perl package scaffolding.
 *
 * Creates:
 *   - Makefile.PL (ExtUtils::MakeMaker build config)
 *   - cpanfile (declarative dependency spec)
 *   - lib/CodingAdventures/<Camel>.pm (module)
 *   - t/00-load.t (load test)
 *   - t/01-basic.t (basic placeholder test)
 *   - BUILD (chain-install deps then prove)
 */
export function generatePerl(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  orderedDeps: string[],
): void {
  const camel = toCamelCase(pkgName);

  const prereqPmEntries = directDeps
    .map((dep) => `        'CodingAdventures::${toCamelCase(dep)}' => 0,`)
    .join("\n");
  const prereqPmStr = prereqPmEntries
    ? `    PREREQ_PM        => {\n${prereqPmEntries}\n    },\n`
    : "";

  const makefilePl = `use strict;
use warnings;
use ExtUtils::MakeMaker;

WriteMakefile(
    NAME             => 'CodingAdventures::${camel}',
    VERSION_FROM     => 'lib/CodingAdventures/${camel}.pm',
    ABSTRACT         => '${description}',
    AUTHOR           => 'coding-adventures',
    LICENSE          => 'mit',
    MIN_PERL_VERSION => '5.026000',
${prereqPmStr}    TEST_REQUIRES    => {
        'Test2::V0' => 0,
    },
    META_MERGE       => {
        'meta-spec' => { version => 2 },
        resources   => {
            repository => {
                type => 'git',
                url  => 'https://github.com/adhithyan15/coding-adventures.git',
                web  => 'https://github.com/adhithyan15/coding-adventures',
            },
        },
    },
);
`;

  const cpanfileDeps = directDeps
    .map((dep) => `requires 'coding-adventures-${dep}';`)
    .join("\n");
  const cpanfile = `# Runtime dependencies
${cpanfileDeps ? cpanfileDeps + "\n" : ""}
# Test dependencies
on 'test' => sub {
    requires 'Test2::V0';
};
`;

  const depUses = directDeps
    .map((dep) => `use CodingAdventures::${toCamelCase(dep)};`)
    .join("\n");
  const modulePm = `package CodingAdventures::${camel};

# ============================================================================
# CodingAdventures::${camel} — ${description}
# ============================================================================
#
# This module is part of the coding-adventures project, an educational
# computing stack built from logic gates up through interpreters and
# compilers.
#
# ${layerCtx}
#
# Usage:
#
#   use CodingAdventures::${camel};
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

${depUses ? depUses + "\n\n" : ""}# TODO: Implement ${camel}

1;

__END__

=head1 NAME

CodingAdventures::${camel} - ${description}

=head1 SYNOPSIS

    use CodingAdventures::${camel};

=head1 DESCRIPTION

${description}

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
`;

  const loadT = `use strict;
use warnings;
use Test2::V0;

use_ok('CodingAdventures::${camel}');

# Verify the module exports a version number.
ok(CodingAdventures::${camel}->VERSION, 'has a VERSION');

done_testing;
`;

  const basicT = `use strict;
use warnings;
use Test2::V0;

use CodingAdventures::${camel};

# TODO: Replace this placeholder with real tests.
ok(1, '${camel} module loaded successfully');

done_testing;
`;

  const buildLines = orderedDeps.map(
    (dep) => `cd ../${dep} && cpanm --with-test --installdeps --quiet .\n`,
  );
  buildLines.push("cpanm --with-test --installdeps --quiet .\n");
  buildLines.push("prove -l -v t/\n");
  const build = buildLines.join("");

  writeFile(path.join(targetDir, "Makefile.PL"), makefilePl);
  writeFile(path.join(targetDir, "cpanfile"), cpanfile);
  writeFile(
    path.join(targetDir, "lib", "CodingAdventures", `${camel}.pm`),
    modulePm,
  );
  writeFile(path.join(targetDir, "t", "00-load.t"), loadT);
  writeFile(path.join(targetDir, "t", "01-basic.t"), basicT);
  writeFile(path.join(targetDir, "BUILD"), build);
}

// -------------------------------------------------------------------------
// Haskell generator
// -------------------------------------------------------------------------

export function generateHaskell(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
  orderedDeps: string[],
): void {
  const pkgNameHaskell = `coding-adventures-${pkgName}`;
  const moduleName = toCamelCase(pkgName);

  let cabal = `cabal-version: 3.0
name:          ${pkgNameHaskell}
version:       0.1.0
synopsis:      ${description}
license:       MIT
author:        Adhithya Rajasekaran
maintainer:    Adhithya Rajasekaran
build-type:    Simple

library
    exposed-modules:  ${moduleName}
    build-depends:    base >=4.14
`;
  for (const dep of orderedDeps) {
    cabal += `                      , coding-adventures-${dep}\n`;
  }
  cabal += `    hs-source-dirs:   src
    default-language: Haskell2010

test-suite spec
    type:             exitcode-stdio-1.0
    main-is:          Spec.hs
    build-depends:    base >=4.14
                    , ${pkgNameHaskell}
`;
  for (const dep of orderedDeps) {
    cabal += `                    , coding-adventures-${dep}\n`;
  }
  cabal += `    hs-source-dirs:   test
    default-language: Haskell2010
`;

  const libHs = `module ${moduleName} where

-- | ${description}
-- ${layerCtx}
someFunc :: IO ()
someFunc = putStrLn "someFunc"
`;

  const specHs = `import ${moduleName}

main :: IO ()
main = do
    putStrLn "Test suite not yet implemented."
`;

  let cabalProject = `packages: .\n`;
  for (const dep of orderedDeps) {
    cabalProject += `          ../${dep}\n`;
  }

  const build = `cabal test all\n`;

  writeFile(path.join(targetDir, `${pkgNameHaskell}.cabal`), cabal);
  writeFile(path.join(targetDir, "cabal.project"), cabalProject);
  writeFile(path.join(targetDir, "src", `${moduleName}.hs`), libHs);
  writeFile(path.join(targetDir, "test", "Spec.hs"), specHs);
  writeFile(path.join(targetDir, "BUILD"), build);
}

// -------------------------------------------------------------------------
// C# generator
// -------------------------------------------------------------------------

export function generateCSharp(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
): void {
  const camel = toCamelCase(pkgName);
  const projectName = `CodingAdventures.${camel}`;
  const testProjectName = `${projectName}.Tests`;

  const projectRefs = directDeps
    .map(
      (dep) =>
        `    <ProjectReference Include="../${dep}/CodingAdventures.${toCamelCase(dep)}.csproj" />`,
    )
    .join("\n");

  const csproj = `<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net9.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <GenerateDocumentationFile>true</GenerateDocumentationFile>
    <PackageId>${projectName}</PackageId>
    <Version>0.1.0</Version>
    <Authors>Adhithya Rajasekaran</Authors>
    <Description>${escapeXml(description)}</Description>
    <PackageLicenseExpression>MIT</PackageLicenseExpression>
  </PropertyGroup>
${projectRefs ? `  <ItemGroup>\n${projectRefs}\n  </ItemGroup>\n` : ""}</Project>
`;

  const graphCs = `namespace ${projectName};

/// <summary>
/// ${description}
/// ${layerCtx}
/// </summary>
public static class ${camel}
{
    public static string Ping() => "${pkgName}";
}
`;

  const testCsproj = `<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net9.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <IsPackable>false</IsPackable>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.12.0" />
    <PackageReference Include="xunit" Version="2.9.2" />
    <PackageReference Include="xunit.runner.visualstudio" Version="2.8.2" />
    <PackageReference Include="coverlet.collector" Version="6.0.2" />
    <PackageReference Include="coverlet.msbuild" Version="6.0.2" />
  </ItemGroup>

  <ItemGroup>
    <Using Include="Xunit" />
    <ProjectReference Include="../../${projectName}.csproj" />
  </ItemGroup>
</Project>
`;

  const testCs = `namespace ${testProjectName};

public sealed class ${camel}Tests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("${pkgName}", ${projectName}.${camel}.Ping());
    }
}
`;

  const build = `dotnet test tests/${testProjectName}/${testProjectName}.csproj --disable-build-servers /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line\n`;

  writeFile(path.join(targetDir, `${projectName}.csproj`), csproj);
  writeFile(path.join(targetDir, `${camel}.cs`), graphCs);
  writeFile(path.join(targetDir, "tests", testProjectName, `${testProjectName}.csproj`), testCsproj);
  writeFile(path.join(targetDir, "tests", testProjectName, `${camel}Tests.cs`), testCs);
  writeFile(path.join(targetDir, "BUILD"), build);
  writeFile(path.join(targetDir, "BUILD_windows"), build);
  writeFile(
    path.join(targetDir, "required_capabilities.json"),
    JSON.stringify(
      {
        $schema:
          "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
        version: 1,
        package: `csharp/${pkgName}`,
        capabilities: [],
        justification:
          "Pure in-memory library and tests. No filesystem, process, environment, or network access needed.",
      },
      null,
      2,
    ) + "\n",
  );
}

// -------------------------------------------------------------------------
// F# generator
// -------------------------------------------------------------------------

export function generateFSharp(
  targetDir: string,
  pkgName: string,
  description: string,
  layerCtx: string,
  directDeps: string[],
): void {
  const camel = toCamelCase(pkgName);
  const projectName = `CodingAdventures.${camel}`;
  const testProjectName = `${projectName}.Tests`;

  const projectRefs = directDeps
    .map(
      (dep) =>
        `    <ProjectReference Include="../${dep}/CodingAdventures.${toCamelCase(dep)}.fsproj" />`,
    )
    .join("\n");

  const fsproj = `<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net9.0</TargetFramework>
    <GenerateDocumentationFile>true</GenerateDocumentationFile>
    <PackageId>${projectName}</PackageId>
    <Version>0.1.0</Version>
    <Authors>Adhithya Rajasekaran</Authors>
    <Description>${escapeXml(description)}</Description>
    <PackageLicenseExpression>MIT</PackageLicenseExpression>
  </PropertyGroup>
${projectRefs ? `  <ItemGroup>\n${projectRefs}\n  </ItemGroup>\n` : ""}  <ItemGroup>
    <Compile Include="${camel}.fs" />
  </ItemGroup>
</Project>
`;

  const moduleFs = `namespace ${projectName}

/// ${description}
/// ${layerCtx}
module ${camel} =
    let ping () = "${pkgName}"
`;

  const testFsproj = `<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net9.0</TargetFramework>
    <IsPackable>false</IsPackable>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="17.12.0" />
    <PackageReference Include="xunit" Version="2.9.2" />
    <PackageReference Include="xunit.runner.visualstudio" Version="2.8.2" />
    <PackageReference Include="coverlet.collector" Version="6.0.2" />
    <PackageReference Include="coverlet.msbuild" Version="6.0.2" />
  </ItemGroup>

  <ItemGroup>
    <ProjectReference Include="../../${projectName}.fsproj" />
    <Compile Include="${camel}Tests.fs" />
  </ItemGroup>
</Project>
`;

  const testFs = `namespace ${testProjectName}

open Xunit
open ${projectName}

type ${camel}Tests() =
    [<Fact>]
    member _.\`\`Ping returns package name\`\`() =
        Assert.Equal("${pkgName}", ${camel}.ping())
`;

  const build = `dotnet test tests/${testProjectName}/${testProjectName}.fsproj --disable-build-servers /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line\n`;

  writeFile(path.join(targetDir, `${projectName}.fsproj`), fsproj);
  writeFile(path.join(targetDir, `${camel}.fs`), moduleFs);
  writeFile(path.join(targetDir, "tests", testProjectName, `${testProjectName}.fsproj`), testFsproj);
  writeFile(path.join(targetDir, "tests", testProjectName, `${camel}Tests.fs`), testFs);
  writeFile(path.join(targetDir, "BUILD"), build);
  writeFile(path.join(targetDir, "BUILD_windows"), build);
  writeFile(
    path.join(targetDir, "required_capabilities.json"),
    JSON.stringify(
      {
        $schema:
          "https://raw.githubusercontent.com/adhithyan15/coding-adventures/main/code/specs/schemas/required_capabilities.schema.json",
        version: 1,
        package: `fsharp/${pkgName}`,
        capabilities: [],
        justification:
          "Pure in-memory library and tests. No filesystem, process, environment, or network access needed.",
      },
      null,
      2,
    ) + "\n",
  );
}

// -------------------------------------------------------------------------
// Common files (README.md + CHANGELOG.md)
// -------------------------------------------------------------------------

/**
 * Generate README.md and CHANGELOG.md for any language.
 *
 * These files follow the same template regardless of language:
 *   - README has package name, description, layer info, deps list, and
 *     a "run tests" snippet.
 *   - CHANGELOG follows Keep a Changelog format with initial 0.1.0 entry.
 */
export function generateCommonFiles(
  targetDir: string,
  pkgName: string,
  description: string,
  layer: number,
  directDeps: string[],
): void {
  const today = todayISO();

  const changelog = `# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - ${today}

### Added

- Initial package scaffolding generated by scaffold-generator
`;

  let readme = `# ${pkgName}\n\n${description}\n`;
  if (layer > 0) {
    readme += `\n## Layer ${layer}\n\nThis package is part of Layer ${layer} of the coding-adventures computing stack.\n`;
  }
  if (directDeps.length > 0) {
    readme += "\n## Dependencies\n\n";
    for (const dep of directDeps) {
      readme += `- ${dep}\n`;
    }
  }
  readme += "\n## Development\n\n```bash\n# Run tests\nbash BUILD\n```\n";

  writeFile(path.join(targetDir, "README.md"), readme);
  writeFile(path.join(targetDir, "CHANGELOG.md"), changelog);
}

// -------------------------------------------------------------------------
// Rust workspace update
// -------------------------------------------------------------------------

/**
 * Add a crate to the workspace Cargo.toml members list.
 *
 * The workspace Cargo.toml at `code/packages/rust/Cargo.toml` has a
 * `members = [...]` array. This function inserts the new crate name
 * if it's not already there.
 *
 * @returns true on success, false if the workspace file is missing or
 *          doesn't have a members array.
 */
export function updateRustWorkspace(
  repoRoot: string,
  pkgName: string,
): boolean {
  const workspacePath = path.join(
    repoRoot,
    "code",
    "packages",
    "rust",
    "Cargo.toml",
  );
  if (!fs.existsSync(workspacePath)) {
    return false;
  }
  let content = fs.readFileSync(workspacePath, "utf-8");
  if (content.includes(`"${pkgName}"`)) {
    return true; // already listed
  }
  const idx = content.indexOf("members = [");
  if (idx < 0) {
    return false;
  }
  const closeIdx = content.indexOf("]", idx);
  if (closeIdx < 0) {
    return false;
  }
  const newEntry = `  "${pkgName}",\n`;
  content = content.slice(0, closeIdx) + newEntry + content.slice(closeIdx);
  fs.writeFileSync(workspacePath, content, "utf-8");
  return true;
}

// =========================================================================
// Main scaffold logic
// =========================================================================

/**
 * Walk up from the current working directory to find the git root.
 *
 * The git root is identified by the presence of a `.git` directory.
 * This allows the tool to work from any subdirectory of the monorepo.
 */
export function findRepoRoot(): string {
  let d = process.cwd();
  while (true) {
    if (fs.existsSync(path.join(d, ".git"))) {
      return d;
    }
    const parent = path.dirname(d);
    if (parent === d) {
      throw new Error("not inside a git repository");
    }
    d = parent;
  }
}

/**
 * Scaffold a package for a single language.
 *
 * This is the core orchestration function. It:
 *   1. Validates that the target directory doesn't already exist
 *   2. Verifies all direct dependencies exist on disk
 *   3. Computes transitive closure and topological sort
 *   4. Generates all language-specific files
 *   5. Generates common files (README.md, CHANGELOG.md)
 *   6. Handles language-specific post-generation tasks
 */
export function scaffoldOne(
  pkgName: string,
  pkgType: string,
  lang: string,
  directDeps: string[],
  layer: number,
  description: string,
  dryRun: boolean,
  repoRoot: string,
  output: (msg: string) => void = (msg) => process.stdout.write(msg + "\n"),
  errOutput: (msg: string) => void = (msg) => process.stderr.write(msg + "\n"),
): void {
  const baseCategory = pkgType === "library" ? "packages" : "programs";
  const baseDir = path.join(repoRoot, "code", baseCategory, lang);
  const dName = dirName(pkgName, lang);
  const targetDir = path.join(baseDir, dName);

  if (fs.existsSync(targetDir)) {
    throw new Error(`directory already exists: ${targetDir}`);
  }

  for (const dep of directDeps) {
    const depDir = resolveDepDir(repoRoot, lang, dep);
    if (!fs.existsSync(depDir)) {
      throw new Error(
        `dependency '${dep}' not found for ${lang} at ${depDir}`,
      );
    }
  }

  const allDeps = transitiveClosure(directDeps, lang, repoRoot);
  const orderedDeps = topologicalSort(allDeps, lang, repoRoot);

  const layerCtx = layer > 0 ? `Layer ${layer} in the computing stack.` : "";

  if (dryRun) {
    output(`[dry-run] Would create ${lang} package at: ${targetDir}`);
    output(`  Direct deps: ${JSON.stringify(directDeps)}`);
    output(`  All transitive deps: ${JSON.stringify(allDeps)}`);
    output(`  Install order: ${JSON.stringify(orderedDeps)}`);
    return;
  }

  fs.mkdirSync(targetDir, { recursive: true });

  const generators: Record<string, () => void> = {
    python: () =>
      generatePython(
        targetDir,
        pkgName,
        description,
        layerCtx,
        directDeps,
        orderedDeps,
      ),
    go: () =>
      generateGo(
        targetDir,
        pkgName,
        description,
        layerCtx,
        directDeps,
        allDeps,
      ),
    ruby: () =>
      generateRuby(
        targetDir,
        pkgName,
        description,
        layerCtx,
        directDeps,
        allDeps,
      ),
    typescript: () =>
      generateTypeScript(
        targetDir,
        pkgName,
        pkgType,
        description,
        layerCtx,
        directDeps,
        orderedDeps,
      ),
    rust: () =>
      generateRust(targetDir, pkgName, description, layerCtx, directDeps),
    elixir: () =>
      generateElixir(
        targetDir,
        pkgName,
        description,
        layerCtx,
        directDeps,
        orderedDeps,
      ),
    perl: () =>
      generatePerl(
        targetDir,
        pkgName,
        description,
        layerCtx,
        directDeps,
        orderedDeps,
      ),
    lua: () => {},
    swift: () => {},
    haskell: () =>
      generateHaskell(
        targetDir,
        pkgName,
        description,
        layerCtx,
        directDeps,
        orderedDeps,
      ),
    csharp: () =>
      generateCSharp(targetDir, pkgName, description, layerCtx, directDeps),
    fsharp: () =>
      generateFSharp(targetDir, pkgName, description, layerCtx, directDeps),
  };

  generators[lang]();
  generateCommonFiles(targetDir, pkgName, description, layer, directDeps);

  output(`Created ${lang} package at: ${targetDir}`);

  if (lang === "rust") {
    if (updateRustWorkspace(repoRoot, pkgName)) {
      output("  Updated code/packages/rust/Cargo.toml workspace members");
    } else {
      errOutput(
        `  WARNING: Manually add "${pkgName}" to code/packages/rust/Cargo.toml members`,
      );
    }
    output("  Run: cargo build --workspace (to verify)");
  } else if (lang === "typescript") {
    output(
      `  Run: cd ${targetDir} && npm install (to generate package-lock.json)`,
    );
  } else if (lang === "go") {
    output(`  Run: cd ${targetDir} && go mod tidy`);
  } else if (lang === "csharp" || lang === "fsharp") {
    output(`  Run: cd ${targetDir} && dotnet test`);
  }
}

// =========================================================================
// CLI entry point
// =========================================================================

/**
 * Main entry point: parse args via CLI Builder, then scaffold packages.
 *
 * The CLI interface is entirely driven by scaffold-generator.json.
 * This function only handles:
 *   1. Loading the spec and parsing argv
 *   2. Discriminating on the result type (help/version/parse)
 *   3. Validating inputs (kebab-case names, valid languages)
 *   4. Calling scaffoldOne for each target language
 */
export function main(argv: string[] = process.argv): void {
  const specFile = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "..",
    "..",
    "..",
    "scaffold-generator.json",
  );

  let result: ParseResult | HelpResult | VersionResult;
  try {
    result = new Parser(specFile, argv).parse();
  } catch (e: unknown) {
    if (e instanceof ParseErrors) {
      for (const error of e.errors) {
        process.stderr.write(`scaffold-generator: ${error.message}\n`);
      }
      process.exit(1);
    }
    throw e;
  }

  // Discriminate on result type using the same pattern as pwd.ts
  if ("text" in result) {
    process.stdout.write((result as HelpResult).text + "\n");
    process.exit(0);
  }
  if ("version" in result && !("flags" in result)) {
    process.stdout.write((result as VersionResult).version + "\n");
    process.exit(0);
  }

  const parsed = result as ParseResult;

  const pkgName = (parsed.arguments["package-name"] as string) || "";
  const pkgType = (parsed.flags["type"] as string) || "library";
  const langStr = (parsed.flags["language"] as string) || "all";
  const depsStr = (parsed.flags["depends-on"] as string) || "";
  const layerVal = parsed.flags["layer"] as number | undefined;
  const description = (parsed.flags["description"] as string) || "";
  const dryRun = !!parsed.flags["dry-run"];

  if (!KEBAB_RE.test(pkgName)) {
    process.stderr.write(
      `scaffold-generator: invalid package name '${pkgName}' (must be kebab-case)\n`,
    );
    process.exit(1);
  }

  let languages: string[];
  if (langStr === "all") {
    languages = [...VALID_LANGUAGES];
  } else {
    languages = [];
    for (const lang of langStr.split(",")) {
      const trimmed = lang.trim();
      if (!(VALID_LANGUAGES as readonly string[]).includes(trimmed)) {
        process.stderr.write(
          `scaffold-generator: unknown language '${trimmed}'\n`,
        );
        process.exit(1);
      }
      languages.push(trimmed);
    }
  }

  const directDeps = depsStr
    ? depsStr
        .split(",")
        .map((d) => d.trim())
        .filter((d) => d.length > 0)
    : [];

  for (const dep of directDeps) {
    if (!KEBAB_RE.test(dep)) {
      process.stderr.write(
        `scaffold-generator: invalid dependency name '${dep}'\n`,
      );
      process.exit(1);
    }
  }

  const repoRoot = findRepoRoot();
  const layer = layerVal ? Number(layerVal) : 0;

  let hadError = false;
  for (const lang of languages) {
    try {
      scaffoldOne(
        pkgName,
        pkgType,
        lang,
        directDeps,
        layer,
        description,
        dryRun,
        repoRoot,
      );
    } catch (e: unknown) {
      if (e instanceof Error) {
        process.stderr.write(`scaffold-generator [${lang}]: ${e.message}\n`);
      }
      hadError = true;
    }
  }

  if (hadError) {
    process.exit(1);
  }
}

// Run if invoked directly (not imported as a library)
const isMainModule =
  typeof process !== "undefined" &&
  process.argv[1] &&
  (() => {
    const entry = process.argv[1].replace(/\\/g, "/");
    return (
      entry.endsWith("scaffold-generator/src/index.ts") ||
      entry.endsWith("scaffold-generator/dist/index.js") ||
      entry.endsWith("/src/index.ts") ||
      entry.endsWith("/dist/index.js")
    );
  })();

if (isMainModule) {
  main();
}
