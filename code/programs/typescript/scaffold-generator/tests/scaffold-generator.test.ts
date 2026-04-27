/**
 * Tests for scaffold-generator.
 *
 * These tests verify the core logic of the scaffold generator:
 *   1. Name normalization (kebab -> snake, camel, joined-lower)
 *   2. Dependency reading for all supported languages
 *   3. Transitive closure (BFS)
 *   4. Topological sort (Kahn's algorithm)
 *   5. File generation (verify critical fields per language)
 *   6. Ruby require ordering (deps before own modules)
 *
 * The tests use a temporary directory to create mock package structures
 * and verify that generated files match the expected templates.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  toSnakeCase,
  toCamelCase,
  toJoinedLower,
  dirName,
  readDeps,
  transitiveClosure,
  topologicalSort,
  generatePython,
  generateGo,
  generateRuby,
  generateTypeScript,
  generateRust,
  generateElixir,
  generatePerl,
  generateHaskell,
  generateCSharp,
  generateFSharp,
  generateCommonFiles,
  updateRustWorkspace,
  findRepoRoot,
  scaffoldOne,
  escapeXml,
  VALID_LANGUAGES,
  KEBAB_RE,
} from "../src/index.js";

// =========================================================================
// Test helpers
// =========================================================================

/**
 * Create a temporary directory for test isolation.
 * Each test gets a fresh directory so there's no cross-contamination.
 */
function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "scaffold-test-"));
}

/**
 * Recursively remove a directory (cleanup after tests).
 */
function removeDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

/**
 * Write a file inside a temp directory, creating parent dirs as needed.
 */
function writeTestFile(base: string, relPath: string, content: string): void {
  const fullPath = path.join(base, relPath);
  fs.mkdirSync(path.dirname(fullPath), { recursive: true });
  fs.writeFileSync(fullPath, content, "utf-8");
}

// =========================================================================
// 1. Name normalization tests
// =========================================================================

describe("toSnakeCase", () => {
  it("converts single-word name", () => {
    expect(toSnakeCase("parser")).toBe("parser");
  });

  it("converts multi-word kebab-case to snake_case", () => {
    expect(toSnakeCase("logic-gates")).toBe("logic_gates");
  });

  it("converts three-word name", () => {
    expect(toSnakeCase("my-cool-package")).toBe("my_cool_package");
  });

  it("handles name with digits", () => {
    expect(toSnakeCase("v8-engine")).toBe("v8_engine");
  });
});

describe("toCamelCase", () => {
  it("converts single-word name", () => {
    expect(toCamelCase("parser")).toBe("Parser");
  });

  it("converts multi-word kebab-case to CamelCase", () => {
    expect(toCamelCase("logic-gates")).toBe("LogicGates");
  });

  it("converts three-word name", () => {
    expect(toCamelCase("my-cool-package")).toBe("MyCoolPackage");
  });

  it("handles name with digits", () => {
    expect(toCamelCase("v8-engine")).toBe("V8Engine");
  });
});

describe("toJoinedLower", () => {
  it("converts single-word name (no change)", () => {
    expect(toJoinedLower("parser")).toBe("parser");
  });

  it("removes hyphens from multi-word name", () => {
    expect(toJoinedLower("logic-gates")).toBe("logicgates");
  });

  it("removes all hyphens from three-word name", () => {
    expect(toJoinedLower("my-cool-package")).toBe("mycoolpackage");
  });
});

describe("dirName", () => {
  it("returns kebab-case for python", () => {
    expect(dirName("logic-gates", "python")).toBe("logic-gates");
  });

  it("returns kebab-case for go", () => {
    expect(dirName("logic-gates", "go")).toBe("logic-gates");
  });

  it("returns kebab-case for typescript", () => {
    expect(dirName("logic-gates", "typescript")).toBe("logic-gates");
  });

  it("returns kebab-case for rust", () => {
    expect(dirName("logic-gates", "rust")).toBe("logic-gates");
  });

  it("returns snake_case for ruby", () => {
    expect(dirName("logic-gates", "ruby")).toBe("logic_gates");
  });

  it("returns snake_case for elixir", () => {
    expect(dirName("logic-gates", "elixir")).toBe("logic_gates");
  });
});

// =========================================================================
// 2. KEBAB_RE validation tests
// =========================================================================

describe("KEBAB_RE", () => {
  it("accepts simple names", () => {
    expect(KEBAB_RE.test("parser")).toBe(true);
  });

  it("accepts kebab-case names", () => {
    expect(KEBAB_RE.test("logic-gates")).toBe(true);
  });

  it("accepts names with digits", () => {
    expect(KEBAB_RE.test("v8-engine")).toBe(true);
  });

  it("rejects uppercase", () => {
    expect(KEBAB_RE.test("MyPackage")).toBe(false);
  });

  it("rejects leading hyphen", () => {
    expect(KEBAB_RE.test("-leading")).toBe(false);
  });

  it("rejects trailing hyphen", () => {
    expect(KEBAB_RE.test("trailing-")).toBe(false);
  });

  it("rejects double hyphens", () => {
    expect(KEBAB_RE.test("double--hyphen")).toBe(false);
  });

  it("rejects empty string", () => {
    expect(KEBAB_RE.test("")).toBe(false);
  });
});

describe("escapeXml", () => {
  it("escapes XML metacharacters", () => {
    expect(escapeXml(`a&b<c>"d"'e'`)).toBe("a&amp;b&lt;c&gt;&quot;d&quot;&apos;e&apos;");
  });
});

// =========================================================================
// 3. Dependency reading tests
// =========================================================================

describe("readDeps", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  // --- Python ---

  it("reads Python deps from BUILD file", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "BUILD", [
      "python -m pip install -e ../logic-gates -e ../arithmetic -e .[dev] --quiet",
      "python -m pytest tests/ -v",
    ].join("\n"));

    const deps = readDeps(pkgDir, "python");
    expect(deps).toEqual(["logic-gates", "arithmetic"]);
  });

  it("returns empty for Python package with no BUILD", () => {
    const pkgDir = path.join(tmpDir, "no-build");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "python")).toEqual([]);
  });

  // --- Go ---

  it("reads Go deps from go.mod replace directives", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "go.mod", [
      "module github.com/adhithyan15/coding-adventures/code/packages/go/my-pkg",
      "",
      "go 1.26",
      "",
      "require (",
      "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/logic-gates v0.0.0",
      ")",
      "",
      "replace (",
      "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/logic-gates => ../logic-gates",
      ")",
    ].join("\n"));

    const deps = readDeps(pkgDir, "go");
    expect(deps).toEqual(["logic-gates"]);
  });

  it("returns empty for Go package with no go.mod", () => {
    const pkgDir = path.join(tmpDir, "no-mod");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "go")).toEqual([]);
  });

  // --- Ruby ---

  it("reads Ruby deps from Gemfile path entries", () => {
    const pkgDir = path.join(tmpDir, "my_pkg");
    writeTestFile(pkgDir, "Gemfile", [
      '# frozen_string_literal: true',
      'source "https://rubygems.org"',
      'gemspec',
      '',
      'gem "coding_adventures_logic_gates", path: "../logic_gates"',
    ].join("\n"));

    const deps = readDeps(pkgDir, "ruby");
    expect(deps).toEqual(["logic-gates"]);
  });

  it("returns empty for Ruby package with no Gemfile", () => {
    const pkgDir = path.join(tmpDir, "no-gemfile");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "ruby")).toEqual([]);
  });

  // --- TypeScript ---

  it("reads TypeScript deps from package.json file: entries", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "package.json", JSON.stringify({
      name: "@coding-adventures/my-pkg",
      dependencies: {
        "@coding-adventures/logic-gates": "file:../logic-gates",
        "@coding-adventures/arithmetic": "file:../arithmetic",
      },
    }));

    const deps = readDeps(pkgDir, "typescript");
    expect(deps).toEqual(["logic-gates", "arithmetic"]);
  });

  it("returns empty for TypeScript package with no package.json", () => {
    const pkgDir = path.join(tmpDir, "no-pkg");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "typescript")).toEqual([]);
  });

  it("returns empty for TypeScript package with malformed JSON", () => {
    const pkgDir = path.join(tmpDir, "bad-json");
    writeTestFile(pkgDir, "package.json", "not valid json{{{");
    expect(readDeps(pkgDir, "typescript")).toEqual([]);
  });

  // --- Rust ---

  it("reads Rust deps from Cargo.toml path entries", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "Cargo.toml", [
      '[package]',
      'name = "my-pkg"',
      '',
      '[dependencies]',
      'logic-gates = { path = "../logic-gates" }',
    ].join("\n"));

    const deps = readDeps(pkgDir, "rust");
    expect(deps).toEqual(["logic-gates"]);
  });

  it("returns empty for Rust package with no Cargo.toml", () => {
    const pkgDir = path.join(tmpDir, "no-cargo");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "rust")).toEqual([]);
  });

  // --- Elixir ---

  it("reads Elixir deps from mix.exs path entries", () => {
    const pkgDir = path.join(tmpDir, "my_pkg");
    writeTestFile(pkgDir, "mix.exs", [
      'defmodule CodingAdventures.MyPkg.MixProject do',
      '  defp deps do',
      '    [',
      '      {:coding_adventures_logic_gates, path: "../logic_gates"}',
      '    ]',
      '  end',
      'end',
    ].join("\n"));

    const deps = readDeps(pkgDir, "elixir");
    expect(deps).toEqual(["logic-gates"]);
  });

  it("returns empty for Elixir package with no mix.exs", () => {
    const pkgDir = path.join(tmpDir, "no-mix");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "elixir")).toEqual([]);
  });

  // --- Perl ---

  it("reads Perl deps from cpanfile", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "cpanfile", [
      "requires 'coding-adventures-logic-gates';",
      "requires 'coding-adventures-arithmetic';",
      "on 'test' => sub {",
      "    requires 'Test2::V0';",
      "};",
    ].join("\n"));

    const deps = readDeps(pkgDir, "perl");
    expect(deps).toEqual(["logic-gates", "arithmetic"]);
  });

  it("returns empty for Perl package with no cpanfile", () => {
    const pkgDir = path.join(tmpDir, "no-cpanfile");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "perl")).toEqual([]);
  });

  // --- C# / F# ---

  it("reads C# deps from ProjectReference entries", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "CodingAdventures.MyPkg.csproj", [
      '<Project Sdk="Microsoft.NET.Sdk">',
      "  <ItemGroup>",
      '    <ProjectReference Include="../logic-gates/CodingAdventures.LogicGates.csproj" />',
      '    <ProjectReference Include="../state_machine/CodingAdventures.StateMachine.csproj" />',
      "  </ItemGroup>",
      "</Project>",
    ].join("\n"));

    const deps = readDeps(pkgDir, "csharp");
    expect(deps).toEqual(["logic-gates", "state-machine"]);
  });

  it("reads F# deps from ProjectReference entries", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "CodingAdventures.MyPkg.fsproj", [
      '<Project Sdk="Microsoft.NET.Sdk">',
      "  <ItemGroup>",
      '    <ProjectReference Include="../graph/CodingAdventures.Graph.fsproj" />',
      "  </ItemGroup>",
      "</Project>",
    ].join("\n"));

    const deps = readDeps(pkgDir, "fsharp");
    expect(deps).toEqual(["graph"]);
  });

  it("rejects traversal-style .NET project references", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeTestFile(pkgDir, "CodingAdventures.MyPkg.csproj", [
      '<Project Sdk="Microsoft.NET.Sdk">',
      "  <ItemGroup>",
      '    <ProjectReference Include="../../evil/CodingAdventures.Evil.csproj" />',
      '    <ProjectReference Include="../safe-dep/CodingAdventures.SafeDep.csproj" />',
      "  </ItemGroup>",
      "</Project>",
    ].join("\n"));

    const deps = readDeps(pkgDir, "csharp");
    expect(deps).toEqual(["safe-dep"]);
  });

  it("returns empty for .NET package with no project file", () => {
    const pkgDir = path.join(tmpDir, "no-project");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "csharp")).toEqual([]);
    expect(readDeps(pkgDir, "fsharp")).toEqual([]);
  });

  // --- Unknown language ---

  it("returns empty for unknown language", () => {
    const pkgDir = path.join(tmpDir, "something");
    fs.mkdirSync(pkgDir, { recursive: true });
    expect(readDeps(pkgDir, "fortran")).toEqual([]);
  });
});

// =========================================================================
// 4. Transitive closure tests
// =========================================================================

describe("transitiveClosure", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("returns empty for no direct deps", () => {
    expect(transitiveClosure([], "python", tmpDir)).toEqual([]);
  });

  it("returns direct deps when they have no further deps", () => {
    // Create a package with no BUILD (so no deps)
    const depDir = path.join(tmpDir, "logic-gates");
    fs.mkdirSync(depDir, { recursive: true });

    const result = transitiveClosure(["logic-gates"], "python", tmpDir);
    expect(result).toEqual(["logic-gates"]);
  });

  it("follows transitive dependencies via BFS", () => {
    // resolveDepDir looks under repoRoot/code/packages/lang/ so we create the
    // packages there to match the production path resolution behaviour.
    const pkgBase = path.join(tmpDir, "code", "packages", "python");

    // C has no deps
    const cDir = path.join(pkgBase, "c-pkg");
    fs.mkdirSync(cDir, { recursive: true });
    writeTestFile(cDir, "BUILD", "python -m pip install -e .[dev] --quiet\n");

    // B depends on C
    const bDir = path.join(pkgBase, "b-pkg");
    fs.mkdirSync(bDir, { recursive: true });
    writeTestFile(bDir, "BUILD", "python -m pip install -e ../c-pkg -e .[dev] --quiet\n");

    // Ask for transitive closure starting from [b-pkg]; should include c-pkg
    const result = transitiveClosure(["b-pkg"], "python", tmpDir);
    expect(result).toEqual(["b-pkg", "c-pkg"]);
  });

  it("deduplicates deps found through multiple paths", () => {
    // resolveDepDir looks under repoRoot/code/packages/lang/ so we create the
    // packages there to match the production path resolution behaviour.
    const pkgBase = path.join(tmpDir, "code", "packages", "python");

    // base has no deps
    const baseDir = path.join(pkgBase, "base");
    fs.mkdirSync(baseDir, { recursive: true });
    writeTestFile(baseDir, "BUILD", "python -m pip install -e .[dev] --quiet\n");

    // left depends on base
    const leftDir = path.join(pkgBase, "left");
    fs.mkdirSync(leftDir, { recursive: true });
    writeTestFile(leftDir, "BUILD", "python -m pip install -e ../base -e .[dev] --quiet\n");

    // right depends on base
    const rightDir = path.join(pkgBase, "right");
    fs.mkdirSync(rightDir, { recursive: true });
    writeTestFile(rightDir, "BUILD", "python -m pip install -e ../base -e .[dev] --quiet\n");

    // Both left and right depend on base; should appear only once
    const result = transitiveClosure(["left", "right"], "python", tmpDir);
    expect(result).toEqual(["base", "left", "right"]);
  });
});

// =========================================================================
// 5. Topological sort tests
// =========================================================================

describe("topologicalSort", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("returns single dep as-is", () => {
    const depDir = path.join(tmpDir, "solo");
    fs.mkdirSync(depDir, { recursive: true });
    writeTestFile(depDir, "BUILD", "python -m pip install -e .[dev] --quiet\n");

    const result = topologicalSort(["solo"], "python", tmpDir);
    expect(result).toEqual(["solo"]);
  });

  it("orders leaves before dependents", () => {
    // leaf has no deps
    const leafDir = path.join(tmpDir, "leaf");
    fs.mkdirSync(leafDir, { recursive: true });
    writeTestFile(leafDir, "BUILD", "python -m pip install -e .[dev] --quiet\n");

    // mid depends on leaf
    const midDir = path.join(tmpDir, "mid");
    fs.mkdirSync(midDir, { recursive: true });
    writeTestFile(midDir, "BUILD", "python -m pip install -e ../leaf -e .[dev] --quiet\n");

    const result = topologicalSort(["leaf", "mid"], "python", tmpDir);
    expect(result).toEqual(["leaf", "mid"]);
  });

  it("handles diamond dependency graph", () => {
    // base has no deps
    const baseDir = path.join(tmpDir, "base");
    fs.mkdirSync(baseDir, { recursive: true });
    writeTestFile(baseDir, "BUILD", "python -m pip install -e .[dev] --quiet\n");

    // left depends on base
    const leftDir = path.join(tmpDir, "left");
    fs.mkdirSync(leftDir, { recursive: true });
    writeTestFile(leftDir, "BUILD", "python -m pip install -e ../base -e .[dev] --quiet\n");

    // right depends on base
    const rightDir = path.join(tmpDir, "right");
    fs.mkdirSync(rightDir, { recursive: true });
    writeTestFile(rightDir, "BUILD", "python -m pip install -e ../base -e .[dev] --quiet\n");

    const result = topologicalSort(
      ["base", "left", "right"],
      "python",
      tmpDir,
    );
    // base must come first; left and right can be in either order
    // but since we sort alphabetically, left comes before right
    expect(result).toEqual(["base", "left", "right"]);
  });

  it("detects circular dependencies", () => {
    // resolveDepDir looks under repoRoot/code/packages/lang/ so we create the
    // packages there to match the production path resolution behaviour.
    const pkgBase = path.join(tmpDir, "code", "packages", "python");

    // A depends on B, B depends on A
    const aDir = path.join(pkgBase, "a-pkg");
    fs.mkdirSync(aDir, { recursive: true });
    writeTestFile(aDir, "BUILD", "python -m pip install -e ../b-pkg -e .[dev] --quiet\n");

    const bDir = path.join(pkgBase, "b-pkg");
    fs.mkdirSync(bDir, { recursive: true });
    writeTestFile(bDir, "BUILD", "python -m pip install -e ../a-pkg -e .[dev] --quiet\n");

    expect(() =>
      topologicalSort(["a-pkg", "b-pkg"], "python", tmpDir),
    ).toThrow("circular dependency detected");
  });
});

// =========================================================================
// 6. File generation tests
// =========================================================================

describe("generatePython", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates pyproject.toml with correct name", () => {
    generatePython(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "pyproject.toml"),
      "utf-8",
    );
    expect(content).toContain('name = "coding-adventures-my-pkg"');
    expect(content).toContain('version = "0.1.0"');
    expect(content).toContain('packages = ["src/my_pkg"]');
  });

  it("creates __init__.py with version", () => {
    generatePython(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "src", "my_pkg", "__init__.py"),
      "utf-8",
    );
    expect(content).toContain('__version__ = "0.1.0"');
  });

  it("creates BUILD with deps in order", () => {
    generatePython(tmpDir, "my-pkg", "A test", "", ["logic-gates"], [
      "directed-graph",
      "logic-gates",
    ]);
    const content = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    expect(content).toContain("-e ../directed-graph");
    expect(content).toContain("-e ../logic-gates");
    // directed-graph should come before logic-gates
    const dgIdx = content.indexOf("directed-graph");
    const lgIdx = content.indexOf("logic-gates");
    expect(dgIdx).toBeLessThan(lgIdx);
  });
});

describe("generateGo", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates go.mod with module path", () => {
    generateGo(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(path.join(tmpDir, "go.mod"), "utf-8");
    expect(content).toContain(
      "module github.com/adhithyan15/coding-adventures/code/packages/go/my-pkg",
    );
    expect(content).toContain("go 1.26");
  });

  it("creates go.mod with require and replace for deps", () => {
    generateGo(tmpDir, "my-pkg", "A test", "", ["logic-gates"], [
      "directed-graph",
      "logic-gates",
    ]);
    const content = fs.readFileSync(path.join(tmpDir, "go.mod"), "utf-8");
    expect(content).toContain("require (");
    expect(content).toContain("logic-gates v0.0.0");
    expect(content).toContain("replace (");
    expect(content).toContain("=> ../directed-graph");
    expect(content).toContain("=> ../logic-gates");
  });

  it("uses joined-lower for package name", () => {
    generateGo(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "my_pkg.go"),
      "utf-8",
    );
    expect(content).toContain("package mypkg");
  });
});

describe("generateRuby", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates gemspec with correct name", () => {
    generateRuby(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "coding_adventures_my_pkg.gemspec"),
      "utf-8",
    );
    expect(content).toContain('spec.name          = "coding_adventures_my_pkg"');
    expect(content).toContain("CodingAdventures::MyPkg::VERSION");
  });

  it("requires dependencies BEFORE own modules in entry point", () => {
    generateRuby(tmpDir, "my-pkg", "A test", "", ["logic-gates"], [
      "logic-gates",
    ]);
    const content = fs.readFileSync(
      path.join(tmpDir, "lib", "coding_adventures_my_pkg.rb"),
      "utf-8",
    );
    // The require for the dep must come BEFORE require_relative
    const depIdx = content.indexOf('require "coding_adventures_logic_gates"');
    const ownIdx = content.indexOf('require_relative "coding_adventures/my_pkg/version"');
    expect(depIdx).toBeGreaterThanOrEqual(0);
    expect(ownIdx).toBeGreaterThanOrEqual(0);
    expect(depIdx).toBeLessThan(ownIdx);
  });

  it("includes IMPORTANT comment about require ordering", () => {
    generateRuby(tmpDir, "my-pkg", "A test", "", ["logic-gates"], [
      "logic-gates",
    ]);
    const content = fs.readFileSync(
      path.join(tmpDir, "lib", "coding_adventures_my_pkg.rb"),
      "utf-8",
    );
    expect(content).toContain("IMPORTANT: Require dependencies FIRST");
  });

  it("lists path deps in Gemfile for all transitive deps", () => {
    generateRuby(tmpDir, "my-pkg", "A test", "", ["logic-gates"], [
      "directed-graph",
      "logic-gates",
    ]);
    const content = fs.readFileSync(path.join(tmpDir, "Gemfile"), "utf-8");
    expect(content).toContain('path: "../directed_graph"');
    expect(content).toContain('path: "../logic_gates"');
  });
});

describe("generateTypeScript", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates package.json with main pointing to src/index.ts", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test package", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "package.json"),
      "utf-8",
    );
    expect(content).toContain('"main": "src/index.ts"');
  });

  it("creates package.json with type: module", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test package", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "package.json"),
      "utf-8",
    );
    expect(content).toContain('"type": "module"');
  });

  it("includes @vitest/coverage-v8 in devDependencies", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test package", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "package.json"),
      "utf-8",
    );
    expect(content).toContain('"@vitest/coverage-v8": "^3.0.0"');
  });

  it("creates package.json with file: deps for direct dependencies", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test", "", ["logic-gates"], [
      "logic-gates",
    ]);
    const content = fs.readFileSync(
      path.join(tmpDir, "package.json"),
      "utf-8",
    );
    expect(content).toContain(
      '"@coding-adventures/logic-gates": "file:../logic-gates"',
    );
  });

  it("creates BUILD that chain-installs transitive deps", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test", "", ["logic-gates"], [
      "directed-graph",
      "logic-gates",
    ]);
    const content = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    expect(content).toContain("npm ci --quiet");
    expect(content).toContain("npx vitest run --coverage");
  });

  it("creates simple BUILD when no deps", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test", "", [], []);
    const content = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    expect(content).toBe("npm ci --quiet\nnpx vitest run --coverage\n");
  });

  it("creates vitest.config.ts with v8 coverage", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "vitest.config.ts"),
      "utf-8",
    );
    expect(content).toContain('provider: "v8"');
    expect(content).toContain("lines: 80");
  });

  it("creates tsconfig.json", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "tsconfig.json"),
      "utf-8",
    );
    expect(content).toContain('"strict": true');
    expect(content).toContain('"declaration": true');
  });

  it("creates src/index.ts with version export", () => {
    generateTypeScript(tmpDir, "my-pkg", "library", "A test", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "src", "index.ts"),
      "utf-8",
    );
    expect(content).toContain('VERSION = "0.1.0"');
  });

  it("generates typescript program boilerplate", () => {
    generateTypeScript(tmpDir, "my-app", "program", "A test app", "", [], []);
    expect(fs.existsSync(path.join(tmpDir, "package.json"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "vite.config.ts"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "src", "main.tsx"))).toBe(true);
  });
});

describe("generateRust", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates Cargo.toml with correct name", () => {
    generateRust(tmpDir, "my-pkg", "A test package", "", []);
    const content = fs.readFileSync(
      path.join(tmpDir, "Cargo.toml"),
      "utf-8",
    );
    expect(content).toContain('name = "my-pkg"');
    expect(content).toContain('version = "0.1.0"');
  });

  it("creates Cargo.toml with path deps", () => {
    generateRust(tmpDir, "my-pkg", "A test", "", ["logic-gates"]);
    const content = fs.readFileSync(
      path.join(tmpDir, "Cargo.toml"),
      "utf-8",
    );
    expect(content).toContain('logic-gates = { path = "../logic-gates" }');
  });

  it("creates src/lib.rs with load test", () => {
    generateRust(tmpDir, "my-pkg", "A test package", "", []);
    const content = fs.readFileSync(
      path.join(tmpDir, "src", "lib.rs"),
      "utf-8",
    );
    expect(content).toContain("fn it_loads()");
  });

  it("creates BUILD with cargo test command", () => {
    generateRust(tmpDir, "my-pkg", "A test", "", []);
    const content = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    expect(content).toContain("cargo test -p my-pkg -- --nocapture");
  });
});

describe("generateElixir", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates mix.exs with correct app name", () => {
    generateElixir(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(path.join(tmpDir, "mix.exs"), "utf-8");
    expect(content).toContain("app: :coding_adventures_my_pkg");
    expect(content).toContain("CodingAdventures.MyPkg.MixProject");
  });

  it("creates mix.exs with path deps", () => {
    generateElixir(tmpDir, "my-pkg", "A test", "", ["logic-gates"], [
      "logic-gates",
    ]);
    const content = fs.readFileSync(path.join(tmpDir, "mix.exs"), "utf-8");
    expect(content).toContain(
      '{:coding_adventures_logic_gates, path: "../logic_gates"}',
    );
  });

  it("creates lib/coding_adventures/<snake>.ex", () => {
    generateElixir(tmpDir, "my-pkg", "A test", "", [], []);
    const content = fs.readFileSync(
      path.join(tmpDir, "lib", "coding_adventures", "my_pkg.ex"),
      "utf-8",
    );
    expect(content).toContain("defmodule CodingAdventures.MyPkg");
  });

  it("creates BUILD that chain-compiles transitive deps", () => {
    generateElixir(tmpDir, "my-pkg", "A test", "", ["logic-gates"], [
      "directed-graph",
      "logic-gates",
    ]);
    const content = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    expect(content).toContain("cd ../directed_graph && mix deps.get");
    expect(content).toContain("cd ../logic_gates && mix deps.get");
    expect(content).toContain("cd ../my_pkg && mix deps.get");
    expect(content).toContain("mix test --cover");
  });
});

// =========================================================================
// 6b. generatePerl tests
// =========================================================================

describe("generateHaskell", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates cabal file with correct name", () => {
    generateHaskell(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(path.join(tmpDir, "coding-adventures-my-pkg.cabal"), "utf-8");
    expect(content).toMatch(/name:\s+coding-adventures-my-pkg/);
    expect(content).toContain("test-suite spec");
  });
});

describe("generatePerl", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates Makefile.PL with package name", () => {
    generatePerl(tmpDir, "my-pkg", "A test package", "", [], []);
    const content = fs.readFileSync(path.join(tmpDir, "Makefile.PL"), "utf-8");
    expect(content).toContain("CodingAdventures::MyPkg");
  });

  it("creates cpanfile", () => {
    generatePerl(tmpDir, "my-pkg", "A test package", "", [], []);
    expect(fs.existsSync(path.join(tmpDir, "cpanfile"))).toBe(true);
  });

  it("creates module .pm file", () => {
    generatePerl(tmpDir, "my-pkg", "A test package", "", [], []);
    expect(
      fs.existsSync(path.join(tmpDir, "lib", "CodingAdventures", "MyPkg.pm")),
    ).toBe(true);
  });

  it("creates test files", () => {
    generatePerl(tmpDir, "my-pkg", "A test package", "", [], []);
    expect(fs.existsSync(path.join(tmpDir, "t", "00-load.t"))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, "t", "01-basic.t"))).toBe(true);
  });

  it("Makefile.PL includes direct dep", () => {
    generatePerl(tmpDir, "my-pkg", "A test", "", ["logic-gates"], ["logic-gates"]);
    const content = fs.readFileSync(path.join(tmpDir, "Makefile.PL"), "utf-8");
    expect(content).toContain("CodingAdventures::LogicGates");
  });

  it("cpanfile includes direct dep", () => {
    generatePerl(tmpDir, "my-pkg", "A test", "", ["logic-gates"], ["logic-gates"]);
    const content = fs.readFileSync(path.join(tmpDir, "cpanfile"), "utf-8");
    expect(content).toContain("coding-adventures-logic-gates");
  });

  it("BUILD chains transitive dep installs", () => {
    generatePerl(tmpDir, "my-pkg", "A test", "", ["logic-gates"], ["logic-gates"]);
    const content = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    expect(content).toContain("../logic-gates");
    expect(content).toContain("prove -l -v t/");
  });

  it("BUILD without deps has only self-install and prove", () => {
    generatePerl(tmpDir, "my-pkg", "A test", "", [], []);
    const content = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    const cdLines = content.split("\n").filter((l) => l.startsWith("cd ../"));
    expect(cdLines).toHaveLength(0);
    expect(content).toContain("prove -l -v t/");
  });
});

describe("generateCSharp", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates a library project and test project", () => {
    generateCSharp(tmpDir, "my-pkg", "A test package", "", []);

    const csproj = fs.readFileSync(
      path.join(tmpDir, "CodingAdventures.MyPkg.csproj"),
      "utf-8",
    );
    const testProject = fs.readFileSync(
      path.join(
        tmpDir,
        "tests",
        "CodingAdventures.MyPkg.Tests",
        "CodingAdventures.MyPkg.Tests.csproj",
      ),
      "utf-8",
    );

    expect(csproj).toContain("<TargetFramework>net9.0</TargetFramework>");
    expect(csproj).toContain("<PackageId>CodingAdventures.MyPkg</PackageId>");
    expect(testProject).toContain('PackageReference Include="xunit"');
    expect(testProject).toContain('PackageReference Include="coverlet.msbuild"');
    expect(testProject).toContain('ProjectReference Include="../../CodingAdventures.MyPkg.csproj"');
  });

  it("creates BUILD files and required capabilities metadata", () => {
    generateCSharp(tmpDir, "my-pkg", "A test package", "", []);

    const build = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    const buildWindows = fs.readFileSync(path.join(tmpDir, "BUILD_windows"), "utf-8");
    const capabilities = fs.readFileSync(
      path.join(tmpDir, "required_capabilities.json"),
      "utf-8",
    );

    expect(build).toContain("dotnet test tests/CodingAdventures.MyPkg.Tests/CodingAdventures.MyPkg.Tests.csproj");
    expect(build).toContain("/p:Threshold=80");
    expect(buildWindows).toBe(build);
    expect(capabilities).toContain('"package": "csharp/my-pkg"');
  });

  it("escapes XML in generated project metadata", () => {
    generateCSharp(
      tmpDir,
      "my-pkg",
      `A & B </Description><Target Name="Injected" />`,
      "",
      [],
    );

    const csproj = fs.readFileSync(
      path.join(tmpDir, "CodingAdventures.MyPkg.csproj"),
      "utf-8",
    );

    expect(csproj).toContain(
      "<Description>A &amp; B &lt;/Description&gt;&lt;Target Name=&quot;Injected&quot; /&gt;</Description>",
    );
    expect(csproj).not.toContain("<Target Name=\"Injected\"");
  });
});

describe("generateFSharp", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates a library project and test project", () => {
    generateFSharp(tmpDir, "my-pkg", "A test package", "", []);

    const fsproj = fs.readFileSync(
      path.join(tmpDir, "CodingAdventures.MyPkg.fsproj"),
      "utf-8",
    );
    const testProject = fs.readFileSync(
      path.join(
        tmpDir,
        "tests",
        "CodingAdventures.MyPkg.Tests",
        "CodingAdventures.MyPkg.Tests.fsproj",
      ),
      "utf-8",
    );

    expect(fsproj).toContain("<TargetFramework>net9.0</TargetFramework>");
    expect(fsproj).toContain('<Compile Include="MyPkg.fs" />');
    expect(testProject).toContain('PackageReference Include="xunit"');
    expect(testProject).toContain('PackageReference Include="coverlet.msbuild"');
    expect(testProject).toContain('ProjectReference Include="../../CodingAdventures.MyPkg.fsproj"');
  });

  it("creates BUILD files and required capabilities metadata", () => {
    generateFSharp(tmpDir, "my-pkg", "A test package", "", []);

    const build = fs.readFileSync(path.join(tmpDir, "BUILD"), "utf-8");
    const buildWindows = fs.readFileSync(path.join(tmpDir, "BUILD_windows"), "utf-8");
    const capabilities = fs.readFileSync(
      path.join(tmpDir, "required_capabilities.json"),
      "utf-8",
    );

    expect(build).toContain("dotnet test tests/CodingAdventures.MyPkg.Tests/CodingAdventures.MyPkg.Tests.fsproj");
    expect(build).toContain("/p:Threshold=80");
    expect(buildWindows).toBe(build);
    expect(capabilities).toContain('"package": "fsharp/my-pkg"');
  });

  it("escapes XML in generated project metadata", () => {
    generateFSharp(
      tmpDir,
      "my-pkg",
      `A & B </Description><Target Name="Injected" />`,
      "",
      [],
    );

    const fsproj = fs.readFileSync(
      path.join(tmpDir, "CodingAdventures.MyPkg.fsproj"),
      "utf-8",
    );

    expect(fsproj).toContain(
      "<Description>A &amp; B &lt;/Description&gt;&lt;Target Name=&quot;Injected&quot; /&gt;</Description>",
    );
    expect(fsproj).not.toContain("<Target Name=\"Injected\"");
  });
});

// =========================================================================
// 7. Common files tests
// =========================================================================

describe("generateCommonFiles", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates README.md with package name and description", () => {
    generateCommonFiles(tmpDir, "my-pkg", "A cool package", 0, []);
    const content = fs.readFileSync(path.join(tmpDir, "README.md"), "utf-8");
    expect(content).toContain("# my-pkg");
    expect(content).toContain("A cool package");
  });

  it("includes layer info when layer > 0", () => {
    generateCommonFiles(tmpDir, "my-pkg", "A test", 3, []);
    const content = fs.readFileSync(path.join(tmpDir, "README.md"), "utf-8");
    expect(content).toContain("## Layer 3");
    expect(content).toContain("Layer 3 of the coding-adventures computing stack");
  });

  it("omits layer info when layer is 0", () => {
    generateCommonFiles(tmpDir, "my-pkg", "A test", 0, []);
    const content = fs.readFileSync(path.join(tmpDir, "README.md"), "utf-8");
    expect(content).not.toContain("## Layer");
  });

  it("lists dependencies in README", () => {
    generateCommonFiles(tmpDir, "my-pkg", "A test", 0, [
      "logic-gates",
      "arithmetic",
    ]);
    const content = fs.readFileSync(path.join(tmpDir, "README.md"), "utf-8");
    expect(content).toContain("- logic-gates");
    expect(content).toContain("- arithmetic");
  });

  it("creates CHANGELOG.md with initial entry", () => {
    generateCommonFiles(tmpDir, "my-pkg", "A test", 0, []);
    const content = fs.readFileSync(
      path.join(tmpDir, "CHANGELOG.md"),
      "utf-8",
    );
    expect(content).toContain("## [0.1.0]");
    expect(content).toContain("scaffold-generator");
  });
});

// =========================================================================
// 8. Rust workspace update tests
// =========================================================================

describe("updateRustWorkspace", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("adds crate to workspace members", () => {
    const repoRoot = tmpDir;
    const cargoDir = path.join(repoRoot, "code", "packages", "rust");
    fs.mkdirSync(cargoDir, { recursive: true });
    fs.writeFileSync(
      path.join(cargoDir, "Cargo.toml"),
      '[workspace]\nmembers = [\n  "existing-crate",\n]\n',
    );

    const result = updateRustWorkspace(repoRoot, "my-pkg");
    expect(result).toBe(true);

    const content = fs.readFileSync(
      path.join(cargoDir, "Cargo.toml"),
      "utf-8",
    );
    expect(content).toContain('"my-pkg"');
  });

  it("returns true without modification if already listed", () => {
    const repoRoot = tmpDir;
    const cargoDir = path.join(repoRoot, "code", "packages", "rust");
    fs.mkdirSync(cargoDir, { recursive: true });
    fs.writeFileSync(
      path.join(cargoDir, "Cargo.toml"),
      '[workspace]\nmembers = [\n  "my-pkg",\n]\n',
    );

    const result = updateRustWorkspace(repoRoot, "my-pkg");
    expect(result).toBe(true);
  });

  it("returns false when Cargo.toml does not exist", () => {
    const result = updateRustWorkspace(tmpDir, "my-pkg");
    expect(result).toBe(false);
  });

  it("returns false when members array is missing", () => {
    const repoRoot = tmpDir;
    const cargoDir = path.join(repoRoot, "code", "packages", "rust");
    fs.mkdirSync(cargoDir, { recursive: true });
    fs.writeFileSync(
      path.join(cargoDir, "Cargo.toml"),
      "[workspace]\n",
    );

    const result = updateRustWorkspace(repoRoot, "my-pkg");
    expect(result).toBe(false);
  });
});

// =========================================================================
// 9. findRepoRoot tests
// =========================================================================

describe("findRepoRoot", () => {
  it("finds the repo root from the current directory", () => {
    // We're running inside a git repo, so this should work
    const root = findRepoRoot();
    expect(fs.existsSync(path.join(root, ".git"))).toBe(true);
  });
});

// =========================================================================
// 10. VALID_LANGUAGES constant test
// =========================================================================

describe("VALID_LANGUAGES", () => {
  it("contains all supported languages, including C# and F#", () => {
    expect(VALID_LANGUAGES).toHaveLength(12);
    expect(VALID_LANGUAGES).toContain("python");
    expect(VALID_LANGUAGES).toContain("go");
    expect(VALID_LANGUAGES).toContain("ruby");
    expect(VALID_LANGUAGES).toContain("typescript");
    expect(VALID_LANGUAGES).toContain("rust");
    expect(VALID_LANGUAGES).toContain("elixir");
    expect(VALID_LANGUAGES).toContain("perl");
    expect(VALID_LANGUAGES).toContain("lua");
    expect(VALID_LANGUAGES).toContain("swift");
    expect(VALID_LANGUAGES).toContain("haskell");
    expect(VALID_LANGUAGES).toContain("csharp");
    expect(VALID_LANGUAGES).toContain("fsharp");
  });
});

// =========================================================================
// 11. scaffoldOne integration test
// =========================================================================

describe("scaffoldOne", () => {
  let tmpDir: string;
  let repoRoot: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
    // Set up a fake repo structure
    repoRoot = tmpDir;
    fs.mkdirSync(path.join(repoRoot, ".git"), { recursive: true });
    for (const lang of VALID_LANGUAGES) {
      fs.mkdirSync(path.join(repoRoot, "code", "packages", lang), {
        recursive: true,
      });
      fs.mkdirSync(path.join(repoRoot, "code", "programs", lang), {
        recursive: true,
      });
    }
  });

  afterEach(() => {
    removeDir(tmpDir);
  });

  it("creates a complete Python package", () => {
    const messages: string[] = [];
    scaffoldOne(
      "test-pkg",
      "library",
      "python",
      [],
      5,
      "A test package",
      false,
      repoRoot,
      (msg) => messages.push(msg),
    );

    const pkgDir = path.join(repoRoot, "code", "packages", "python", "test-pkg");
    expect(fs.existsSync(path.join(pkgDir, "pyproject.toml"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "BUILD"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "README.md"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "CHANGELOG.md"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "src", "test_pkg", "__init__.py"))).toBe(true);
    expect(messages.some((m) => m.includes("Created python"))).toBe(true);
  });

  it("creates a complete TypeScript package", () => {
    const messages: string[] = [];
    scaffoldOne(
      "test-pkg",
      "library",
      "typescript",
      [],
      0,
      "A test package",
      false,
      repoRoot,
      (msg) => messages.push(msg),
    );

    const pkgDir = path.join(
      repoRoot,
      "code",
      "packages",
      "typescript",
      "test-pkg",
    );
    expect(fs.existsSync(path.join(pkgDir, "package.json"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "tsconfig.json"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "vitest.config.ts"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "src", "index.ts"))).toBe(true);
    expect(
      fs.existsSync(path.join(pkgDir, "tests", "test-pkg.test.ts")),
    ).toBe(true);
  });

  it("uses programs/ for program type", () => {
    scaffoldOne(
      "test-tool",
      "program",
      "go",
      [],
      0,
      "A test tool",
      false,
      repoRoot,
      () => {},
    );

    const pkgDir = path.join(repoRoot, "code", "programs", "go", "test-tool");
    expect(fs.existsSync(path.join(pkgDir, "go.mod"))).toBe(true);
  });

  it("throws when directory already exists", () => {
    const pkgDir = path.join(
      repoRoot,
      "code",
      "packages",
      "python",
      "existing",
    );
    fs.mkdirSync(pkgDir, { recursive: true });

    expect(() =>
      scaffoldOne("existing", "library", "python", [], 0, "test", false, repoRoot),
    ).toThrow("directory already exists");
  });

  it("throws when dependency not found", () => {
    expect(() =>
      scaffoldOne(
        "test-pkg",
        "library",
        "python",
        ["nonexistent-dep"],
        0,
        "test",
        false,
        repoRoot,
      ),
    ).toThrow("dependency 'nonexistent-dep' not found");
  });

  it("dry run does not create files", () => {
    const messages: string[] = [];
    scaffoldOne(
      "test-pkg",
      "library",
      "python",
      [],
      0,
      "A test",
      true,
      repoRoot,
      (msg) => messages.push(msg),
    );

    const pkgDir = path.join(
      repoRoot,
      "code",
      "packages",
      "python",
      "test-pkg",
    );
    expect(fs.existsSync(pkgDir)).toBe(false);
    expect(messages.some((m) => m.includes("[dry-run]"))).toBe(true);
  });

  it("creates Ruby package with snake_case directory", () => {
    scaffoldOne(
      "test-pkg",
      "library",
      "ruby",
      [],
      0,
      "A test",
      false,
      repoRoot,
      () => {},
    );

    const pkgDir = path.join(
      repoRoot,
      "code",
      "packages",
      "ruby",
      "test_pkg",
    );
    expect(fs.existsSync(pkgDir)).toBe(true);
    expect(
      fs.existsSync(
        path.join(pkgDir, "coding_adventures_test_pkg.gemspec"),
      ),
    ).toBe(true);
  });

  it("creates Elixir package with snake_case directory", () => {
    scaffoldOne(
      "test-pkg",
      "library",
      "elixir",
      [],
      0,
      "A test",
      false,
      repoRoot,
      () => {},
    );

    const pkgDir = path.join(
      repoRoot,
      "code",
      "packages",
      "elixir",
      "test_pkg",
    );
    expect(fs.existsSync(pkgDir)).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "mix.exs"))).toBe(true);
  });

  it("creates a complete C# package", () => {
    scaffoldOne(
      "test-pkg",
      "library",
      "csharp",
      [],
      0,
      "A test package",
      false,
      repoRoot,
      () => {},
    );

    const pkgDir = path.join(
      repoRoot,
      "code",
      "packages",
      "csharp",
      "test-pkg",
    );
    expect(fs.existsSync(path.join(pkgDir, "CodingAdventures.TestPkg.csproj"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "BUILD"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "BUILD_windows"))).toBe(true);
    expect(
      fs.existsSync(
        path.join(
          pkgDir,
          "tests",
          "CodingAdventures.TestPkg.Tests",
          "CodingAdventures.TestPkg.Tests.csproj",
        ),
      ),
    ).toBe(true);
  });

  it("creates a complete F# package", () => {
    scaffoldOne(
      "test-pkg",
      "library",
      "fsharp",
      [],
      0,
      "A test package",
      false,
      repoRoot,
      () => {},
    );

    const pkgDir = path.join(
      repoRoot,
      "code",
      "packages",
      "fsharp",
      "test-pkg",
    );
    expect(fs.existsSync(path.join(pkgDir, "CodingAdventures.TestPkg.fsproj"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "BUILD"))).toBe(true);
    expect(fs.existsSync(path.join(pkgDir, "BUILD_windows"))).toBe(true);
    expect(
      fs.existsSync(
        path.join(
          pkgDir,
          "tests",
          "CodingAdventures.TestPkg.Tests",
          "CodingAdventures.TestPkg.Tests.fsproj",
        ),
      ),
    ).toBe(true);
  });
});
