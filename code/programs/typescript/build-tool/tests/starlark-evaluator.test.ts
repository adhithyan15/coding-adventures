/**
 * Tests for starlark-evaluator.ts -- Starlark BUILD File Evaluator
 *
 * These tests verify that the Starlark evaluator correctly:
 *
 *   - Detects Starlark vs shell BUILD files (isStarlarkBuild)
 *   - Generates shell commands from target declarations (generateCommands)
 *   - Extracts structured targets from Starlark results (evaluateBuildFile)
 *
 * ## Test strategy
 *
 * Detection and command generation are pure functions -- they need no
 * filesystem or interpreter. We test those exhaustively with simple inputs.
 *
 * For evaluateBuildFile, we create temporary BUILD files on disk and
 * exercise the full evaluation pipeline. Since the StarlarkInterpreter
 * requires a compiler function (which may not be available in the test
 * environment), those tests are guarded by try/catch to gracefully
 * handle missing compiler dependencies.
 */

import { describe, it, expect } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  isStarlarkBuild,
  generateCommands,
  evaluateBuildFile,
  type Target,
} from "../src/starlark-evaluator.js";

// ---------------------------------------------------------------------------
// Helper: create a Target with defaults for fields we don't care about
// ---------------------------------------------------------------------------

function makeTarget(overrides: Partial<Target> = {}): Target {
  return {
    rule: "py_library",
    name: "test-pkg",
    srcs: [],
    deps: [],
    testRunner: "",
    entryPoint: "",
    ...overrides,
  };
}

// ===========================================================================
// Tests: isStarlarkBuild
// ===========================================================================

describe("isStarlarkBuild", () => {
  // -------------------------------------------------------------------------
  // Starlark patterns -- should return true
  // -------------------------------------------------------------------------

  it("detects load() statements", () => {
    const content = `load("code/rules/python.star", "py_library")\n\npy_library(name = "foo")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects load() with leading comments", () => {
    const content = `# This is a Starlark BUILD file\n# Another comment\nload("rules.star", "py_library")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects load() with leading blank lines", () => {
    const content = `\n\n\nload("rules.star", "go_library")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects def statements (function definitions)", () => {
    const content = `def my_custom_rule(name, srcs):\n    pass\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects py_library rule calls", () => {
    const content = `py_library(\n    name = "logic-gates",\n    srcs = glob(["src/**/*.py"]),\n)\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects py_binary rule calls", () => {
    const content = `py_binary(name = "build-tool")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects go_library rule calls", () => {
    const content = `go_library(name = "starlark-parser")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects go_binary rule calls", () => {
    const content = `go_binary(name = "build-tool")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects ruby_library rule calls", () => {
    const content = `ruby_library(name = "arithmetic")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects ts_library rule calls", () => {
    const content = `ts_library(name = "virtual-machine")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects rust_library rule calls", () => {
    const content = `rust_library(name = "lexer")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects elixir_library rule calls", () => {
    const content = `elixir_library(name = "parser")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects ts_binary rule calls", () => {
    const content = `ts_binary(name = "build-tool")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects rust_binary rule calls", () => {
    const content = `rust_binary(name = "compiler")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects elixir_binary rule calls", () => {
    const content = `elixir_binary(name = "server")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  it("detects ruby_binary rule calls", () => {
    const content = `ruby_binary(name = "cli")\n`;
    expect(isStarlarkBuild(content)).toBe(true);
  });

  // -------------------------------------------------------------------------
  // Shell patterns -- should return false
  // -------------------------------------------------------------------------

  it("classifies shell commands as non-Starlark", () => {
    const content = `npm install --silent\nnpx vitest run --coverage\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies pip install as non-Starlark", () => {
    const content = `uv pip install --system -e ".[dev]"\npython -m pytest\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies go commands as non-Starlark", () => {
    const content = `go build ./...\ngo test ./... -v -cover\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies bundle commands as non-Starlark", () => {
    const content = `bundle install --quiet\nbundle exec rake test\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies cargo commands as non-Starlark", () => {
    const content = `cargo build\ncargo test\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies mix commands as non-Starlark", () => {
    const content = `mix deps.get\nmix test --cover\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies empty content as non-Starlark", () => {
    expect(isStarlarkBuild("")).toBe(false);
  });

  it("classifies comment-only content as non-Starlark", () => {
    const content = `# Just a comment\n# Another comment\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies blank-line-only content as non-Starlark", () => {
    const content = `\n\n\n\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });

  it("classifies cd commands as non-Starlark", () => {
    const content = `cd ../dep && npm ci --quiet\nnpm ci --quiet\n`;
    expect(isStarlarkBuild(content)).toBe(false);
  });
});

// ===========================================================================
// Tests: generateCommands
// ===========================================================================

describe("generateCommands", () => {
  // -------------------------------------------------------------------------
  // Python rules
  // -------------------------------------------------------------------------

  it("generates pytest commands for py_library (default runner)", () => {
    const target = makeTarget({ rule: "py_library" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toContain("uv pip install");
    expect(cmds[1]).toContain("pytest");
  });

  it("generates unittest commands for py_library with unittest runner", () => {
    const target = makeTarget({ rule: "py_library", testRunner: "unittest" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toContain("uv pip install");
    expect(cmds[1]).toContain("unittest discover");
  });

  it("generates pytest commands for py_library with pytest runner", () => {
    const target = makeTarget({ rule: "py_library", testRunner: "pytest" });
    const cmds = generateCommands(target);

    expect(cmds[1]).toContain("pytest");
  });

  it("generates commands for py_binary", () => {
    const target = makeTarget({ rule: "py_binary" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toContain("uv pip install");
    expect(cmds[1]).toContain("pytest");
  });

  // -------------------------------------------------------------------------
  // Go rules
  // -------------------------------------------------------------------------

  it("generates commands for go_library", () => {
    const target = makeTarget({ rule: "go_library" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(3);
    expect(cmds[0]).toBe("go build ./...");
    expect(cmds[1]).toBe("go test ./... -v -cover");
    expect(cmds[2]).toBe("go vet ./...");
  });

  it("generates commands for go_binary", () => {
    const target = makeTarget({ rule: "go_binary" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(3);
    expect(cmds[0]).toBe("go build ./...");
  });

  // -------------------------------------------------------------------------
  // Ruby rules
  // -------------------------------------------------------------------------

  it("generates commands for ruby_library", () => {
    const target = makeTarget({ rule: "ruby_library" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("bundle install --quiet");
    expect(cmds[1]).toBe("bundle exec rake test");
  });

  it("generates commands for ruby_binary", () => {
    const target = makeTarget({ rule: "ruby_binary" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("bundle install --quiet");
  });

  // -------------------------------------------------------------------------
  // TypeScript rules
  // -------------------------------------------------------------------------

  it("generates commands for ts_library", () => {
    const target = makeTarget({ rule: "ts_library" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("npm install --silent");
    expect(cmds[1]).toBe("npx vitest run --coverage");
  });

  it("generates commands for ts_binary", () => {
    const target = makeTarget({ rule: "ts_binary" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("npm install --silent");
  });

  // -------------------------------------------------------------------------
  // Rust rules
  // -------------------------------------------------------------------------

  it("generates commands for rust_library", () => {
    const target = makeTarget({ rule: "rust_library" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("cargo build");
    expect(cmds[1]).toBe("cargo test");
  });

  it("generates commands for rust_binary", () => {
    const target = makeTarget({ rule: "rust_binary" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("cargo build");
  });

  // -------------------------------------------------------------------------
  // Elixir rules
  // -------------------------------------------------------------------------

  it("generates commands for elixir_library", () => {
    const target = makeTarget({ rule: "elixir_library" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("mix deps.get");
    expect(cmds[1]).toBe("mix test --cover");
  });

  it("generates commands for elixir_binary", () => {
    const target = makeTarget({ rule: "elixir_binary" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toBe("mix deps.get");
  });

  // -------------------------------------------------------------------------
  // Unknown rules
  // -------------------------------------------------------------------------

  it("generates diagnostic echo for unknown rules", () => {
    const target = makeTarget({ rule: "java_library" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(1);
    expect(cmds[0]).toContain("Unknown rule: java_library");
  });

  it("generates diagnostic echo for empty rule", () => {
    const target = makeTarget({ rule: "" });
    const cmds = generateCommands(target);

    expect(cmds).toHaveLength(1);
    expect(cmds[0]).toContain("Unknown rule:");
  });
});

// ===========================================================================
// Tests: Target extraction helpers (tested indirectly via types)
// ===========================================================================

describe("Target interface", () => {
  it("has the expected shape", () => {
    // This test verifies the Target type at compile time and runtime.
    const target: Target = {
      rule: "py_library",
      name: "my-package",
      srcs: ["src/**/*.py", "tests/**/*.py"],
      deps: ["python/arithmetic", "python/logic-gates"],
      testRunner: "pytest",
      entryPoint: "src/main.py",
    };

    expect(target.rule).toBe("py_library");
    expect(target.name).toBe("my-package");
    expect(target.srcs).toHaveLength(2);
    expect(target.deps).toHaveLength(2);
    expect(target.testRunner).toBe("pytest");
    expect(target.entryPoint).toBe("src/main.py");
  });

  it("supports empty optional fields", () => {
    const target: Target = {
      rule: "go_library",
      name: "parser",
      srcs: [],
      deps: [],
      testRunner: "",
      entryPoint: "",
    };

    expect(target.testRunner).toBe("");
    expect(target.entryPoint).toBe("");
    expect(target.srcs).toHaveLength(0);
    expect(target.deps).toHaveLength(0);
  });
});

// ===========================================================================
// Tests: generateCommands preserves Go standard patterns
// ===========================================================================

describe("generateCommands -- Go conventions", () => {
  it("uses ./... patterns (not parent directory references)", () => {
    // Lesson learned (2026-03-22): Go BUILD files must use ./... patterns.
    // The build tool cd's into the package dir before running commands.
    const target = makeTarget({ rule: "go_library" });
    const cmds = generateCommands(target);

    for (const cmd of cmds) {
      expect(cmd).not.toContain("cd ../");
      expect(cmd).toContain("./...");
    }
  });
});

// ===========================================================================
// Tests: generateCommands for all binary variants
// ===========================================================================

describe("generateCommands -- binary rules mirror library rules", () => {
  it("go_binary produces same commands as go_library", () => {
    const libCmds = generateCommands(makeTarget({ rule: "go_library" }));
    const binCmds = generateCommands(makeTarget({ rule: "go_binary" }));
    expect(binCmds).toEqual(libCmds);
  });

  it("ruby_binary produces same commands as ruby_library", () => {
    const libCmds = generateCommands(makeTarget({ rule: "ruby_library" }));
    const binCmds = generateCommands(makeTarget({ rule: "ruby_binary" }));
    expect(binCmds).toEqual(libCmds);
  });

  it("ts_binary produces same commands as ts_library", () => {
    const libCmds = generateCommands(makeTarget({ rule: "ts_library" }));
    const binCmds = generateCommands(makeTarget({ rule: "ts_binary" }));
    expect(binCmds).toEqual(libCmds);
  });

  it("rust_binary produces same commands as rust_library", () => {
    const libCmds = generateCommands(makeTarget({ rule: "rust_library" }));
    const binCmds = generateCommands(makeTarget({ rule: "rust_binary" }));
    expect(binCmds).toEqual(libCmds);
  });

  it("elixir_binary produces same commands as elixir_library", () => {
    const libCmds = generateCommands(makeTarget({ rule: "elixir_library" }));
    const binCmds = generateCommands(makeTarget({ rule: "elixir_binary" }));
    expect(binCmds).toEqual(libCmds);
  });
});

// ===========================================================================
// Tests: evaluateBuildFile + extractTargets (smoke + malformed inputs)
// ===========================================================================
//
// evaluateBuildFile relies on the starlark-interpreter package being
// installable.  In the build-tool's normal test environment the interpreter
// is reachable as a sibling file: dep; if not, every test in this block
// rejects with the same "Cannot find package" — guard each with try/catch
// so missing-interpreter setups still pass.

describe("evaluateBuildFile + extractTargets", () => {
  function makeTmpDir(): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-eval-"));
  }

  it("returns no targets when _targets is absent", () => {
    const tmp = makeTmpDir();
    try {
      const buildPath = path.join(tmp, "BUILD");
      fs.writeFileSync(buildPath, "x = 1\n");
      let result: { targets: Target[] };
      try {
        result = evaluateBuildFile(buildPath, tmp, tmp);
      } catch (err) {
        if (err instanceof Error && /Cannot find package|Cannot find module/.test(err.message)) return;
        throw err;
      }
      expect(result.targets).toEqual([]);
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("rejects when _targets is not a list", () => {
    const tmp = makeTmpDir();
    try {
      const buildPath = path.join(tmp, "BUILD");
      fs.writeFileSync(buildPath, '_targets = "oops"\n');
      try {
        evaluateBuildFile(buildPath, tmp, tmp);
      } catch (err) {
        if (err instanceof Error && /Cannot find package|Cannot find module/.test(err.message)) return;
        expect((err as Error).message).toMatch(/_targets is not a list/);
        return;
      }
      throw new Error("expected throw");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("extracts a target dict with missing/wrong-type fields safely", () => {
    const tmp = makeTmpDir();
    try {
      // srcs is intentionally a list with a non-string element; entry_point
      // is absent.  Both should fall back to "" / [] without throwing.
      const buildPath = path.join(tmp, "BUILD");
      fs.writeFileSync(buildPath, `
_targets = [
  {
    "rule": "py_library",
    "name": "foo",
    "srcs": ["src/foo.py", 42],
    "deps": [],
  },
]
`);
      let result: { targets: Target[] };
      try {
        result = evaluateBuildFile(buildPath, tmp, tmp);
      } catch (err) {
        if (err instanceof Error && /Cannot find package|Cannot find module/.test(err.message)) return;
        throw err;
      }
      expect(result.targets).toHaveLength(1);
      expect(result.targets[0].rule).toBe("py_library");
      expect(result.targets[0].name).toBe("foo");
      expect(result.targets[0].srcs).toEqual(["src/foo.py"]); // 42 dropped
      expect(result.targets[0].entryPoint).toBe("");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("rejects when a _targets element is not a dict", () => {
    const tmp = makeTmpDir();
    try {
      const buildPath = path.join(tmp, "BUILD");
      fs.writeFileSync(buildPath, '_targets = ["string-not-dict"]\n');
      try {
        evaluateBuildFile(buildPath, tmp, tmp);
      } catch (err) {
        if (err instanceof Error && /Cannot find package|Cannot find module/.test(err.message)) return;
        expect((err as Error).message).toMatch(/is not a dict/);
        return;
      }
      throw new Error("expected throw");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });

  it("surfaces interpreter errors with the BUILD-file path", () => {
    const tmp = makeTmpDir();
    try {
      const buildPath = path.join(tmp, "BUILD");
      fs.writeFileSync(buildPath, "def f(\n"); // syntax error
      try {
        evaluateBuildFile(buildPath, tmp, tmp);
      } catch (err) {
        if (err instanceof Error && /Cannot find package|Cannot find module/.test(err.message)) return;
        expect((err as Error).message).toContain("Evaluating BUILD file");
        return;
      }
      throw new Error("expected throw");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });
});
