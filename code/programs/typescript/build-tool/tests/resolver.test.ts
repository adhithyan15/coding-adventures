/**
 * Tests for resolver.ts -- Dependency Resolution & Directed Graph
 *
 * These tests cover:
 * - DirectedGraph operations (add/query nodes and edges)
 * - Topological sort via independentGroups (Kahn's algorithm)
 * - Transitive closure and transitive dependents
 * - Affected nodes calculation
 * - Cycle detection
 * - Dependency parsing for all 6 languages
 * - Known names mapping
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  DirectedGraph,
  MetadataEncodingError,
  resolveDependencies,
  buildKnownNames,
} from "../src/resolver.js";
import { discoverPackages, type Package } from "../src/discovery.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-resolver-"));
}

function rmDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

function writeFile(filepath: string, content: string): void {
  fs.mkdirSync(path.dirname(filepath), { recursive: true });
  fs.writeFileSync(filepath, content, "utf-8");
}

function makePkg(
  name: string,
  pkgPath: string,
  language: string,
): Package {
  return { name, path: pkgPath, buildCommands: ["echo test"], language };
}

interface SharedResolutionFixture {
  workspace: {
    files: Array<{
      path: string;
      content_utf8?: string;
      content_base64?: string;
    }>;
  };
  expected: {
    outcome: "ok" | "error";
    result: { edges?: string[][] };
    diagnostics: Array<{
      code: string;
      path: string;
      package: string;
      details: { encoding: string };
    }>;
  };
  limits: { wall_time_ms: number };
}

const packageRoot = fileURLToPath(new URL("..", import.meta.url));
const sharedFixtureRoot = fileURLToPath(
  new URL("../../../../specs/fixtures/build-tool-v1/cases/", import.meta.url),
);

function loadSharedResolutionFixture(name: string): SharedResolutionFixture {
  return JSON.parse(
    fs.readFileSync(path.join(sharedFixtureRoot, name), "utf-8"),
  ) as SharedResolutionFixture;
}

function materializeSharedResolutionFixture(
  fixture: SharedResolutionFixture,
): { root: string; packages: Package[] } {
  const root = makeTempDir();
  fs.mkdirSync(path.join(root, ".git"));
  for (const file of fixture.workspace.files) {
    const destination = path.join(root, ...file.path.split("/"));
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    const content = file.content_base64 === undefined
      ? Buffer.from(file.content_utf8 ?? "", "utf-8")
      : Buffer.from(file.content_base64, "base64");
    fs.writeFileSync(destination, content);
  }
  return {
    root,
    packages: discoverPackages(path.join(root, "code")),
  };
}

function graphEdges(graph: DirectedGraph): string[][] {
  return graph.nodes()
    .flatMap((from) => graph.successors(from).map((to) => [from, to]))
    .sort((left, right) => left.join("\0").localeCompare(right.join("\0")));
}

// ---------------------------------------------------------------------------
// Tests: DirectedGraph basics
// ---------------------------------------------------------------------------

describe("DirectedGraph", () => {
  describe("node operations", () => {
    it("should add and check nodes", () => {
      const g = new DirectedGraph();
      g.addNode("A");
      expect(g.hasNode("A")).toBe(true);
      expect(g.hasNode("B")).toBe(false);
    });

    it("should list all nodes", () => {
      const g = new DirectedGraph();
      g.addNode("B");
      g.addNode("A");
      expect(g.nodes().sort()).toEqual(["A", "B"]);
    });

    it("should handle duplicate addNode calls", () => {
      const g = new DirectedGraph();
      g.addNode("A");
      g.addNode("A");
      expect(g.nodes()).toEqual(["A"]);
    });
  });

  describe("edge operations", () => {
    it("should add edges and auto-create nodes", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      expect(g.hasNode("A")).toBe(true);
      expect(g.hasNode("B")).toBe(true);
      expect(g.successors("A")).toEqual(["B"]);
      expect(g.predecessors("B")).toEqual(["A"]);
    });

    it("should track successors and predecessors", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("A", "C");
      expect(g.successors("A").sort()).toEqual(["B", "C"]);
      expect(g.predecessors("A")).toEqual([]);
    });
  });

  describe("successors / predecessors on unknown nodes", () => {
    // Exercises the `?? []` branches in successors() and predecessors()
    // when the requested node has never been seen by the graph.
    it("returns [] for an unknown node", () => {
      const g = new DirectedGraph();
      expect(g.successors("ghost")).toEqual([]);
      expect(g.predecessors("ghost")).toEqual([]);
    });
  });

  describe("transitiveClosure", () => {
    it("should find all reachable nodes", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("B", "C");
      g.addEdge("C", "D");
      expect(Array.from(g.transitiveClosure("A")).sort()).toEqual([
        "B",
        "C",
        "D",
      ]);
    });

    it("should return empty set for leaf node", () => {
      const g = new DirectedGraph();
      g.addNode("A");
      expect(g.transitiveClosure("A").size).toBe(0);
    });

    it("should return empty set for unknown node", () => {
      const g = new DirectedGraph();
      expect(g.transitiveClosure("X").size).toBe(0);
    });
  });

  describe("transitiveDependents", () => {
    it("should find all nodes that depend on given node", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("B", "C");
      // A -> B -> C: dependents of C are B and A
      expect(Array.from(g.transitiveDependents("C")).sort()).toEqual([
        "A",
        "B",
      ]);
    });

    it("should return empty set for root node", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      expect(g.transitiveDependents("A").size).toBe(0);
    });
  });

  describe("independentGroups", () => {
    it("should partition into correct levels", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("A", "C");
      g.addEdge("B", "D");
      g.addEdge("C", "D");

      const groups = g.independentGroups();
      expect(groups).toEqual([["A"], ["B", "C"], ["D"]]);
    });

    it("should handle single node", () => {
      const g = new DirectedGraph();
      g.addNode("A");
      expect(g.independentGroups()).toEqual([["A"]]);
    });

    it("should handle multiple independent nodes", () => {
      const g = new DirectedGraph();
      g.addNode("A");
      g.addNode("B");
      g.addNode("C");
      const groups = g.independentGroups();
      expect(groups).toEqual([["A", "B", "C"]]);
    });

    it("should detect cycles", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("B", "A");
      expect(() => g.independentGroups()).toThrow("cycle");
    });

    it("should handle a linear chain", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("B", "C");
      const groups = g.independentGroups();
      expect(groups).toEqual([["A"], ["B"], ["C"]]);
    });
  });

  describe("affectedNodes", () => {
    it("should include changed nodes and their dependents", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("B", "C");
      g.addEdge("A", "D");

      const affected = g.affectedNodes(new Set(["A"]));
      expect(Array.from(affected).sort()).toEqual(["A", "B", "C", "D"]);
    });

    it("should handle unknown nodes gracefully", () => {
      const g = new DirectedGraph();
      g.addNode("A");
      const affected = g.affectedNodes(new Set(["UNKNOWN"]));
      expect(affected.size).toBe(0);
    });

    it("should handle multiple changed nodes", () => {
      const g = new DirectedGraph();
      g.addEdge("A", "B");
      g.addEdge("C", "D");

      const affected = g.affectedNodes(new Set(["A", "C"]));
      expect(Array.from(affected).sort()).toEqual(["A", "B", "C", "D"]);
    });

    it("should handle node with no dependents", () => {
      const g = new DirectedGraph();
      g.addNode("A");
      const affected = g.affectedNodes(new Set(["A"]));
      expect(Array.from(affected)).toEqual(["A"]);
    });
  });
});

// ---------------------------------------------------------------------------
// Tests: buildKnownNames
// ---------------------------------------------------------------------------

describe("buildKnownNames", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should map Python packages", () => {
    const pkgPath = path.join(tmpDir, "logic-gates");
    fs.mkdirSync(pkgPath, { recursive: true });
    const pkg = makePkg("python/logic-gates", pkgPath, "python");

    const known = buildKnownNames([pkg]);
    expect(known.get("coding-adventures-logic-gates")).toBe(
      "python/logic-gates",
    );
  });

  it("should map Ruby packages", () => {
    const pkgPath = path.join(tmpDir, "logic_gates");
    fs.mkdirSync(pkgPath, { recursive: true });
    const pkg = makePkg("ruby/logic_gates", pkgPath, "ruby");

    const known = buildKnownNames([pkg]);
    expect(known.get("coding_adventures_logic_gates")).toBe(
      "ruby/logic_gates",
    );
  });

  it("should map Go packages from go.mod", () => {
    const pkgPath = path.join(tmpDir, "my-tool");
    writeFile(
      path.join(pkgPath, "go.mod"),
      "module github.com/user/repo/my-tool\n\ngo 1.21\n",
    );
    const pkg = makePkg("go/my-tool", pkgPath, "go");

    const known = buildKnownNames([pkg]);
    expect(known.get("github.com/user/repo/my-tool")).toBe("go/my-tool");
  });

  it("should map TypeScript packages", () => {
    const pkgPath = path.join(tmpDir, "logic-gates");
    fs.mkdirSync(pkgPath, { recursive: true });
    const pkg = makePkg("typescript/logic-gates", pkgPath, "typescript");

    const known = buildKnownNames([pkg]);
    expect(known.get("@coding-adventures/logic-gates")).toBe(
      "typescript/logic-gates",
    );
  });

  it("should map Rust packages", () => {
    const pkgPath = path.join(tmpDir, "logic-gates");
    fs.mkdirSync(pkgPath, { recursive: true });
    const pkg = makePkg("rust/logic-gates", pkgPath, "rust");

    const known = buildKnownNames([pkg]);
    expect(known.get("logic-gates")).toBe("rust/logic-gates");
  });

  it("should map WASM packages by Cargo package name without claiming bare Rust crate names", () => {
    const pkgPath = path.join(tmpDir, "avl-tree");
    writeFile(
      path.join(pkgPath, "Cargo.toml"),
      `[package]\nname = "avl-tree-wasm"\n`,
    );
    const pkg = makePkg("wasm/avl-tree", pkgPath, "wasm");

    const known = buildKnownNames([pkg]);
    expect(known.get("avl-tree")).toBeUndefined();
    expect(known.get("avl-tree-wasm")).toBe("wasm/avl-tree");
  });

  it("should map Elixir packages with hyphen-to-underscore conversion", () => {
    const pkgPath = path.join(tmpDir, "logic-gates");
    fs.mkdirSync(pkgPath, { recursive: true });
    const pkg = makePkg("elixir/logic-gates", pkgPath, "elixir");

    const known = buildKnownNames([pkg]);
    expect(known.get("coding_adventures_logic_gates")).toBe(
      "elixir/logic-gates",
    );
  });
});

// ---------------------------------------------------------------------------
// Tests: resolveDependencies
// ---------------------------------------------------------------------------

describe("resolveDependencies", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it.each([
    "resolution-lua-utf8.json",
    "resolution-lua-invalid-utf8.json",
  ])("consumes shared Lua UTF-8 fixture %s", (fixtureName) => {
    const fixture = loadSharedResolutionFixture(fixtureName);
    const materialized = materializeSharedResolutionFixture(fixture);
    try {
      if (fixture.expected.outcome === "ok") {
        expect(graphEdges(resolveDependencies(materialized.packages))).toEqual(
          (fixture.expected.result.edges ?? []).sort((left, right) =>
            left.join("\0").localeCompare(right.join("\0"))
          ),
        );
        return;
      }

      const diagnostic = fixture.expected.diagnostics[0];
      let caught: unknown;
      try {
        resolveDependencies(materialized.packages);
      } catch (error) {
        caught = error;
      }
      expect(caught).toBeInstanceOf(MetadataEncodingError);
      const encodingError = caught as MetadataEncodingError;
      expect(encodingError.code).toBe(diagnostic.code);
      expect(encodingError.package).toBe(diagnostic.package);
      expect(encodingError.manifest).toBe(diagnostic.path);
      expect(encodingError.encoding).toBe(diagnostic.details.encoding);
      expect(encodingError.message).toBe(
        `${diagnostic.code}: package=${diagnostic.package} manifest=${diagnostic.path} encoding=${diagnostic.details.encoding}`,
      );
      expect(encodingError.message).not.toContain(materialized.root);
    } finally {
      rmDir(materialized.root);
    }
  });

  it("accepts a valid literal replacement character in Lua metadata", () => {
    const pkgDir = path.join(tmpDir, "code", "packages", "lua", "pkg");
    writeFile(path.join(pkgDir, "BUILD"), "echo building\n");
    writeFile(
      path.join(pkgDir, "coding-adventures-pkg-0.1.0-1.rockspec"),
      "package = \"coding-adventures-pkg\"\n-- valid literal: \uFFFD\ndependencies = {\n  \"lua >= 5.4\",\n}\n",
    );

    const graph = resolveDependencies(discoverPackages(path.join(tmpDir, "code")));
    expect(graphEdges(graph)).toEqual([]);
  });

  it("real CLI returns exit 2 and the stable diagnostic for invalid UTF-8", () => {
    const fixture = loadSharedResolutionFixture(
      "resolution-lua-invalid-utf8.json",
    );
    const materialized = materializeSharedResolutionFixture(fixture);
    try {
      const tsxCli = path.join(packageRoot, "node_modules", "tsx", "dist", "cli.mjs");
      const entrypoint = path.join(packageRoot, "src", "index.ts");
      const result = spawnSync(
        process.execPath,
        [
          tsxCli,
          entrypoint,
          "--root",
          materialized.root,
          "--force",
          "--dry-run",
          "--language",
          "lua",
        ],
        {
          encoding: "utf-8",
          timeout: fixture.limits.wall_time_ms,
        },
      );
      const diagnostic = fixture.expected.diagnostics[0];
      expect(result.error).toBeUndefined();
      expect(result.status).toBe(2);
      expect(result.stderr).toBe(
        `${diagnostic.code}: package=${diagnostic.package} manifest=${diagnostic.path} encoding=${diagnostic.details.encoding}\n`,
      );
      expect(result.stderr).not.toContain(materialized.root);
    } finally {
      rmDir(materialized.root);
    }
  });

  it("should resolve Python dependencies from pyproject.toml", () => {
    const gatesDir = path.join(tmpDir, "logic-gates");
    const arithDir = path.join(tmpDir, "arithmetic");
    writeFile(path.join(gatesDir, "pyproject.toml"), `[project]\nname = "coding-adventures-logic-gates"\ndependencies = []\n`);
    writeFile(path.join(arithDir, "pyproject.toml"), `[project]\nname = "coding-adventures-arithmetic"\ndependencies = [\n    "coding-adventures-logic-gates>=0.1",\n]\n`);

    const gates = makePkg("python/logic-gates", gatesDir, "python");
    const arith = makePkg("python/arithmetic", arithDir, "python");

    const graph = resolveDependencies([gates, arith]);
    // Edge: logic-gates -> arithmetic (logic-gates must build first)
    expect(graph.successors("python/logic-gates")).toContain(
      "python/arithmetic",
    );
  });

  it("should resolve Ruby dependencies from .gemspec", () => {
    const gatesDir = path.join(tmpDir, "logic_gates");
    const arithDir = path.join(tmpDir, "arithmetic");
    fs.mkdirSync(gatesDir, { recursive: true });
    writeFile(path.join(arithDir, "arithmetic.gemspec"), `Gem::Specification.new do |spec|\n  spec.add_dependency "coding_adventures_logic_gates"\nend\n`);

    const gates = makePkg("ruby/logic_gates", gatesDir, "ruby");
    const arith = makePkg("ruby/arithmetic", arithDir, "ruby");

    const graph = resolveDependencies([gates, arith]);
    expect(graph.successors("ruby/logic_gates")).toContain("ruby/arithmetic");
  });

  it("should resolve Go dependencies from go.mod", () => {
    const gatesDir = path.join(tmpDir, "logic-gates");
    const arithDir = path.join(tmpDir, "arithmetic");
    writeFile(path.join(gatesDir, "go.mod"), "module github.com/user/repo/logic-gates\n\ngo 1.21\n");
    writeFile(path.join(arithDir, "go.mod"), "module github.com/user/repo/arithmetic\n\ngo 1.21\n\nrequire (\n\tgithub.com/user/repo/logic-gates v0.1.0\n)\n");

    const gates = makePkg("go/logic-gates", gatesDir, "go");
    const arith = makePkg("go/arithmetic", arithDir, "go");

    const graph = resolveDependencies([gates, arith]);
    expect(graph.successors("go/logic-gates")).toContain("go/arithmetic");
  });

  it("should resolve TypeScript dependencies from package.json", () => {
    const gatesDir = path.join(tmpDir, "logic-gates");
    const arithDir = path.join(tmpDir, "arithmetic");
    fs.mkdirSync(gatesDir, { recursive: true });
    writeFile(path.join(arithDir, "package.json"), JSON.stringify({
      name: "@coding-adventures/arithmetic",
      dependencies: {
        "@coding-adventures/logic-gates": "file:../logic-gates",
      },
    }, null, 2));

    const gates = makePkg("typescript/logic-gates", gatesDir, "typescript");
    const arith = makePkg("typescript/arithmetic", arithDir, "typescript");

    const graph = resolveDependencies([gates, arith]);
    expect(graph.successors("typescript/logic-gates")).toContain(
      "typescript/arithmetic",
    );
  });

  it("should resolve Rust dependencies from Cargo.toml", () => {
    const gatesDir = path.join(tmpDir, "logic-gates");
    const arithDir = path.join(tmpDir, "arithmetic");
    fs.mkdirSync(gatesDir, { recursive: true });
    writeFile(path.join(arithDir, "Cargo.toml"), `[package]\nname = "arithmetic"\n\n[dependencies]\nlogic-gates = { path = "../logic-gates" }\n`);

    const gates = makePkg("rust/logic-gates", gatesDir, "rust");
    const arith = makePkg("rust/arithmetic", arithDir, "rust");

    const graph = resolveDependencies([gates, arith]);
    expect(graph.successors("rust/logic-gates")).toContain("rust/arithmetic");
  });

  it("should resolve WASM dependencies to the Rust crate on shared basenames", () => {
    const wasmDir = path.join(tmpDir, "wasm-avl-tree");
    const rustDir = path.join(tmpDir, "rust-avl-tree");
    writeFile(
      path.join(wasmDir, "Cargo.toml"),
      `[package]\nname = "avl-tree-wasm"\n\n[dependencies]\navl-tree = { path = "../../rust/avl-tree" }\n`,
    );
    writeFile(path.join(rustDir, "Cargo.toml"), `[package]\nname = "avl-tree"\n`);

    const wasmPkg = makePkg("wasm/avl-tree", wasmDir, "wasm");
    const rustPkg = makePkg("rust/avl-tree", rustDir, "rust");

    const graph = resolveDependencies([wasmPkg, rustPkg]);
    expect(graph.successors("rust/avl-tree")).toContain("wasm/avl-tree");
    expect(graph.successors("wasm/avl-tree")).not.toContain("wasm/avl-tree");
  });

  it("should resolve .NET project references across C# and F#", () => {
    const csharpDir = path.join(tmpDir, "csharp-graph");
    const fsharpDir = path.join(tmpDir, "fsharp-helpers");
    writeFile(
      path.join(csharpDir, "CodingAdventures.Graph.csproj"),
      `<Project Sdk="Microsoft.NET.Sdk">\n  <ItemGroup>\n    <ProjectReference Include="../fsharp-helpers/CodingAdventures.Helpers.fsproj" />\n  </ItemGroup>\n</Project>\n`,
    );
    writeFile(
      path.join(fsharpDir, "CodingAdventures.Helpers.fsproj"),
      `<Project Sdk="Microsoft.NET.Sdk">\n</Project>\n`,
    );

    const graphPkg = makePkg("csharp/graph", csharpDir, "csharp");
    const helpersPkg = makePkg("fsharp/helpers", fsharpDir, "fsharp");

    const graph = resolveDependencies([graphPkg, helpersPkg]);
    expect(graph.successors("fsharp/helpers")).toContain("csharp/graph");
  });

  it("should prefer same-language .NET packages when basenames collide", () => {
    const csharpGraphDir = path.join(tmpDir, "csharp", "graph");
    const csharpBitsetDir = path.join(tmpDir, "csharp", "bitset");
    const fsharpBitsetDir = path.join(tmpDir, "fsharp", "bitset");
    writeFile(
      path.join(csharpGraphDir, "CodingAdventures.Graph.csproj"),
      `<Project Sdk="Microsoft.NET.Sdk">\n  <ItemGroup>\n    <ProjectReference Include="../bitset/CodingAdventures.Bitset.csproj" />\n  </ItemGroup>\n</Project>\n`,
    );
    writeFile(
      path.join(csharpBitsetDir, "CodingAdventures.Bitset.csproj"),
      `<Project Sdk="Microsoft.NET.Sdk">\n</Project>\n`,
    );
    writeFile(
      path.join(fsharpBitsetDir, "CodingAdventures.Bitset.fsproj"),
      `<Project Sdk="Microsoft.NET.Sdk">\n</Project>\n`,
    );

    const graphPkg = makePkg("csharp/graph", csharpGraphDir, "csharp");
    const csharpBitsetPkg = makePkg("csharp/bitset", csharpBitsetDir, "csharp");
    const fsharpBitsetPkg = makePkg("fsharp/bitset", fsharpBitsetDir, "fsharp");

    const graph = resolveDependencies([graphPkg, csharpBitsetPkg, fsharpBitsetPkg]);
    expect(graph.successors("csharp/bitset")).toContain("csharp/graph");
    expect(graph.successors("fsharp/bitset")).not.toContain("csharp/graph");
  });

  it("should resolve Elixir dependencies from mix.exs", () => {
    const gatesDir = path.join(tmpDir, "logic-gates");
    const arithDir = path.join(tmpDir, "arithmetic");
    fs.mkdirSync(gatesDir, { recursive: true });
    writeFile(path.join(arithDir, "mix.exs"), `defmodule Arithmetic.MixProject do\n  defp deps do\n    [\n      {:coding_adventures_logic_gates, path: "../logic-gates"}\n    ]\n  end\nend\n`);

    const gates = makePkg("elixir/logic-gates", gatesDir, "elixir");
    const arith = makePkg("elixir/arithmetic", arithDir, "elixir");

    const graph = resolveDependencies([gates, arith]);
    expect(graph.successors("elixir/logic-gates")).toContain(
      "elixir/arithmetic",
    );
  });

  it("should skip external dependencies", () => {
    const pkgDir = path.join(tmpDir, "my-pkg");
    writeFile(path.join(pkgDir, "pyproject.toml"), `[project]\nname = "coding-adventures-my-pkg"\ndependencies = [\n    "requests>=2.0",\n    "flask",\n]\n`);

    const pkg = makePkg("python/my-pkg", pkgDir, "python");
    const graph = resolveDependencies([pkg]);
    expect(graph.successors("python/my-pkg")).toEqual([]);
  });

  it("should handle packages with no metadata files", () => {
    const pkgDir = path.join(tmpDir, "no-metadata");
    fs.mkdirSync(pkgDir, { recursive: true });

    const pkg = makePkg("python/no-metadata", pkgDir, "python");
    const graph = resolveDependencies([pkg]);
    expect(graph.hasNode("python/no-metadata")).toBe(true);
    expect(graph.successors("python/no-metadata")).toEqual([]);
  });

  // ── Coverage for the "dep is external / not in known names" branch in
  //    each language-specific parser.  These tests don't add new dep edges;
  //    they just exercise the `if (pkgName) { ... }` false branch where the
  //    looked-up name isn't part of the workspace's known packages.

  it("Go parser ignores external require entries", () => {
    const pkgDir = path.join(tmpDir, "go-external");
    writeFile(path.join(pkgDir, "go.mod"), "module example.com/me\n\nrequire (\n  github.com/external/lib v1.2.3\n)\n");
    const pkg = makePkg("go/me", pkgDir, "go");
    const graph = resolveDependencies([pkg]);
    expect(graph.successors("go/me")).toEqual([]);
  });

  it("TypeScript parser ignores external package.json entries", () => {
    const pkgDir = path.join(tmpDir, "ts-external");
    writeFile(path.join(pkgDir, "package.json"), JSON.stringify({
      name: "@coding-adventures/ts-external",
      dependencies: { "lodash": "^4.0.0", "react": "^18.0.0" },
    }));
    const pkg = makePkg("typescript/ts-external", pkgDir, "typescript");
    const graph = resolveDependencies([pkg]);
    expect(graph.successors("typescript/ts-external")).toEqual([]);
  });

  it("Rust parser ignores external Cargo.toml deps without `path =`", () => {
    const pkgDir = path.join(tmpDir, "rs-external");
    writeFile(path.join(pkgDir, "Cargo.toml"), `[package]\nname = "rs-external"\n\n[dependencies]\nserde = "1"\nrand = { version = "0.8" }\n`);
    const pkg = makePkg("rust/rs-external", pkgDir, "rust");
    const graph = resolveDependencies([pkg]);
    expect(graph.successors("rust/rs-external")).toEqual([]);
  });

  it("Elixir parser ignores external atom dependencies", () => {
    const pkgDir = path.join(tmpDir, "ex-external");
    writeFile(path.join(pkgDir, "mix.exs"), `defmodule Foo do\n  def deps do\n    [{:jason, "~> 1.0"}]\n  end\nend\n`);
    const pkg = makePkg("elixir/ex-external", pkgDir, "elixir");
    const graph = resolveDependencies([pkg]);
    expect(graph.successors("elixir/ex-external")).toEqual([]);
  });

  it("Go parser handles missing go.mod gracefully", () => {
    const pkgDir = path.join(tmpDir, "go-no-mod");
    fs.mkdirSync(pkgDir, { recursive: true });
    const pkg = makePkg("go/no-mod", pkgDir, "go");
    expect(() => resolveDependencies([pkg])).not.toThrow();
  });
});
