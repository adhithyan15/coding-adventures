/**
 * Tests for plan.ts -- Build Plan Serialization and Deserialization
 *
 * These tests verify:
 *   - Round-trip: write then read produces the same plan.
 *   - Version rejection: mismatched schema_version throws.
 *   - Missing file: reading a nonexistent file throws.
 *   - Invalid JSON: corrupted file throws.
 *   - Edge cases: null affected_packages, empty arrays, etc.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  writePlan,
  readPlan,
  CURRENT_SCHEMA_VERSION,
  type BuildPlan,
  type PackageEntry,
} from "../src/plan.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

let tempDir: string;

beforeEach(() => {
  tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-plan-"));
});

afterEach(() => {
  fs.rmSync(tempDir, { recursive: true, force: true });
});

function planPath(name: string = "plan.json"): string {
  return path.join(tempDir, name);
}

/**
 * Create a minimal valid build plan for testing.
 */
function makeMinimalPlan(overrides?: Partial<BuildPlan>): BuildPlan {
  return {
    schema_version: CURRENT_SCHEMA_VERSION,
    diff_base: "origin/main",
    force: false,
    affected_packages: null,
    packages: [],
    dependency_edges: [],
    languages_needed: {},
    ...overrides,
  };
}

interface SharedPlanFixture {
  workspace: {
    files: Array<{ path: string; content_utf8: string }>;
  };
  expected: {
    result: { plan: BuildPlan };
  };
  limits: { wall_time_ms: number };
}

const packageRoot = fileURLToPath(new URL("..", import.meta.url));
const sharedFixtureRoot = fileURLToPath(
  new URL("../../../../specs/fixtures/build-tool-v1/cases/", import.meta.url),
);

function loadSharedPlanFixture(name: string): SharedPlanFixture {
  return JSON.parse(
    fs.readFileSync(path.join(sharedFixtureRoot, name), "utf-8"),
  ) as SharedPlanFixture;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("writePlan / readPlan round-trip", () => {
  it("round-trips a minimal plan", () => {
    const plan = makeMinimalPlan();
    const fp = planPath();
    writePlan(plan, fp);
    const loaded = readPlan(fp);
    expect(loaded).toEqual(plan);
  });

  it("round-trips a plan with packages", () => {
    const pkg: PackageEntry = {
      name: "python/logic-gates",
      rel_path: "code/packages/python/logic-gates",
      language: "python",
      build_commands: ["uv pip install -e .", "pytest"],
    };
    const plan = makeMinimalPlan({
      packages: [pkg],
      languages_needed: { python: true },
      affected_packages: ["python/logic-gates"],
    });
    const fp = planPath();
    writePlan(plan, fp);
    const loaded = readPlan(fp);
    expect(loaded).toEqual(plan);
  });

  it("round-trips a plan with Starlark metadata", () => {
    const pkg: PackageEntry = {
      name: "typescript/build-tool",
      rel_path: "code/programs/typescript/build-tool",
      language: "typescript",
      build_commands: ["npm install", "vitest run"],
      is_starlark: true,
      declared_srcs: ["src/*.ts", "tests/*.ts"],
      declared_deps: ["typescript/starlark-interpreter"],
    };
    const plan = makeMinimalPlan({
      packages: [pkg],
      dependency_edges: [["typescript/starlark-interpreter", "typescript/build-tool"]],
      languages_needed: { typescript: true },
    });
    const fp = planPath();
    writePlan(plan, fp);
    const loaded = readPlan(fp);
    expect(loaded).toEqual(plan);
  });

  it("round-trips a plan with force mode", () => {
    const plan = makeMinimalPlan({ force: true, affected_packages: null });
    const fp = planPath();
    writePlan(plan, fp);
    const loaded = readPlan(fp);
    expect(loaded.force).toBe(true);
    expect(loaded.affected_packages).toBeNull();
  });

  it("round-trips dependency edges", () => {
    const edges: [string, string][] = [
      ["python/boolean-algebra", "python/logic-gates"],
      ["python/logic-gates", "python/arithmetic"],
    ];
    const plan = makeMinimalPlan({ dependency_edges: edges });
    const fp = planPath();
    writePlan(plan, fp);
    const loaded = readPlan(fp);
    expect(loaded.dependency_edges).toEqual(edges);
  });

  it("round-trips multiple languages", () => {
    const langs = { python: true, ruby: true, go: true };
    const plan = makeMinimalPlan({ languages_needed: langs });
    const fp = planPath();
    writePlan(plan, fp);
    const loaded = readPlan(fp);
    expect(loaded.languages_needed).toEqual(langs);
  });
});

describe("readPlan validation", () => {
  it("rejects a plan with wrong schema version", () => {
    const plan = makeMinimalPlan({ schema_version: 999 });
    const fp = planPath();
    // Write it directly (bypassing writePlan which always uses CURRENT_SCHEMA_VERSION)
    fs.writeFileSync(fp, JSON.stringify(plan, null, 2), "utf-8");
    expect(() => readPlan(fp)).toThrow(/schema_version/);
  });

  it("rejects a plan with version 0", () => {
    const fp = planPath();
    fs.writeFileSync(fp, JSON.stringify({ schema_version: 0 }), "utf-8");
    expect(() => readPlan(fp)).toThrow(/schema_version/);
  });

  it("rejects a plan with missing version", () => {
    const fp = planPath();
    fs.writeFileSync(fp, JSON.stringify({ diff_base: "main" }), "utf-8");
    expect(() => readPlan(fp)).toThrow(/schema_version/);
  });

  it("throws on missing file", () => {
    expect(() => readPlan(planPath("nonexistent.json"))).toThrow();
  });

  it("throws on invalid JSON", () => {
    const fp = planPath();
    fs.writeFileSync(fp, "not valid json {{{", "utf-8");
    expect(() => readPlan(fp)).toThrow(/Invalid JSON/);
  });

  it("throws on non-object JSON (array)", () => {
    const fp = planPath();
    fs.writeFileSync(fp, "[]", "utf-8");
    expect(() => readPlan(fp)).toThrow(/expected a JSON object/);
  });

  it("throws on non-object JSON (string)", () => {
    const fp = planPath();
    fs.writeFileSync(fp, '"hello"', "utf-8");
    expect(() => readPlan(fp)).toThrow(/expected a JSON object/);
  });

  it("throws on non-object JSON (number)", () => {
    const fp = planPath();
    fs.writeFileSync(fp, "42", "utf-8");
    expect(() => readPlan(fp)).toThrow(/expected a JSON object/);
  });

  it("throws on null JSON", () => {
    const fp = planPath();
    fs.writeFileSync(fp, "null", "utf-8");
    expect(() => readPlan(fp)).toThrow(/expected a JSON object/);
  });
});

describe("CURRENT_SCHEMA_VERSION", () => {
  it("is 1", () => {
    expect(CURRENT_SCHEMA_VERSION).toBe(1);
  });
});

describe("writePlan file format", () => {
  it("writes pretty-printed JSON with trailing newline", () => {
    const plan = makeMinimalPlan();
    const fp = planPath();
    writePlan(plan, fp);
    const raw = fs.readFileSync(fp, "utf-8");
    // Should be indented (not minified).
    expect(raw).toContain("\n");
    // Should end with a newline.
    expect(raw.endsWith("\n")).toBe(true);
    // Should be valid JSON.
    expect(() => JSON.parse(raw)).not.toThrow();
  });
});

describe("real CLI plan emission", () => {
  it("consumes the shared portable package-path fixture", () => {
    const fixture = loadSharedPlanFixture("plan-portable-package-path.json");
    const root = path.join(tempDir, "repo");
    fs.mkdirSync(path.join(root, ".git"), { recursive: true });
    for (const file of fixture.workspace.files) {
      const destination = path.join(root, ...file.path.split("/"));
      fs.mkdirSync(path.dirname(destination), { recursive: true });
      fs.writeFileSync(destination, file.content_utf8, "utf-8");
    }

    const outputPath = path.join(tempDir, "emitted-plan.json");
    const tsxCli = path.join(
      packageRoot,
      "node_modules",
      "tsx",
      "dist",
      "cli.mjs",
    );
    const entrypoint = path.join(packageRoot, "src", "index.ts");
    const result = spawnSync(
      process.execPath,
      [
        tsxCli,
        entrypoint,
        "--root",
        root,
        "--force",
        "--emit-plan",
        "--plan-file",
        outputPath,
      ],
      { encoding: "utf-8", timeout: fixture.limits.wall_time_ms },
    );

    expect(result.error).toBeUndefined();
    expect(result.status).toBe(0);
    expect(readPlan(outputPath)).toEqual(fixture.expected.result.plan);
  });
});
