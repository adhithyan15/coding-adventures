/**
 * Native conformance tests for the process-free graph and diff-selection core.
 *
 * Fixture discovery intentionally happens here, outside production code. The
 * evaluator receives only already-materialized values and therefore cannot
 * gain checkout, Git, filesystem, process, environment, or network authority.
 */

import { readFileSync, readdirSync } from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  evaluateDiffSelection,
  evaluateGraph,
  type DiffSelectionInput,
  type PackageSpec,
} from "../src/graph-diff.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(HERE, "../../../../..");
const FIXTURE_ROOT = path.join(
  REPO_ROOT,
  "code/specs/fixtures/build-tool-v1",
);
const CASE_ROOT = path.join(FIXTURE_ROOT, "cases");

const GRAPH_IDS = [
  "graph/canonical-edge-order",
  "graph/chain",
  "graph/cycle",
  "graph/diamond",
  "graph/empty",
  "graph/isolated",
  "graph/multiple-components",
  "graph/partial-cycle-no-output",
] as const;

const DIFF_IDS = [
  "diff-selection/exact-build-fronts",
  "diff-selection/forced-package",
  "diff-selection/known-unmatched-near-build",
  "diff-selection/match-work-at-limit",
  "diff-selection/match-work-over-limit",
  "diff-selection/package-prefix",
  "diff-selection/repository-boundary-reverse-index",
  "diff-selection/shared-input-multiconsumer",
  "diff-selection/strict-glob-character-classes",
  "diff-selection/transitive-package-change",
  "diff-selection/unknown-path-all",
  "diff-selection/unknown-path-error",
] as const;

type Fixture = {
  id: string;
  input: {
    options: {
      packages: Array<{
        name: string;
        rel_path: string;
        source_mode: string;
        source_globs?: string[];
      }>;
      edges: string[][];
      forced_packages: string[];
      unknown_path_policy: string;
      boundary_sha256?: string;
    };
    changed_paths: string[];
  };
  expected: {
    outcome: "ok" | "error";
    result: Record<string, unknown>;
    diagnostics: Array<{ code: string }>;
  };
};

function loadJson(filename: string): unknown {
  return JSON.parse(readFileSync(filename, "utf8")) as unknown;
}

function fixtures(prefix: "graph" | "diff-selection"): Fixture[] {
  return readdirSync(CASE_ROOT)
    .filter((name) => name.startsWith(`${prefix}-`) && name.endsWith(".json"))
    .sort()
    .map((name) => loadJson(path.join(CASE_ROOT, name)) as Fixture);
}

function packageSpec(value: Fixture["input"]["options"]["packages"][number]): PackageSpec {
  return {
    name: value.name,
    relPath: value.rel_path,
    sourceMode: value.source_mode,
    sourceGlobs: value.source_globs ?? [],
  };
}

function diffInput(fixture: Fixture): DiffSelectionInput {
  const options = fixture.input.options;
  const boundarySha256 = options.boundary_sha256 ?? "";
  return {
    packages: options.packages.map(packageSpec),
    edges: options.edges,
    forcedPackages: options.forced_packages,
    unknownPathPolicy: options.unknown_path_policy,
    changedPaths: fixture.input.changed_paths,
    boundarySha256,
    boundary: boundarySha256
      ? loadJson(path.join(FIXTURE_ROOT, "repository-source-input-boundary.json"))
      : undefined,
  };
}

function assertFixture(
  fixture: Fixture,
  actual:
    | ReturnType<typeof evaluateGraph>
    | ReturnType<typeof evaluateDiffSelection>,
): void {
  if (actual.ok) {
    expect(fixture.expected.outcome, fixture.id).toBe("ok");
    expect(fixture.expected.diagnostics, fixture.id).toEqual([]);
    expect(actual.value, fixture.id).toEqual(fixture.expected.result);
  } else {
    expect(fixture.expected.outcome, fixture.id).toBe("error");
    expect(fixture.expected.result, fixture.id).toEqual({});
    expect(
      fixture.expected.diagnostics.map((diagnostic) => diagnostic.code),
      fixture.id,
    ).toEqual([actual.code]);
  }
}

function packagePrefix(name: string, relPath: string): PackageSpec {
  return { name, relPath, sourceMode: "package_prefix", sourceGlobs: [] };
}

function simpleDiff(packages: PackageSpec[]): DiffSelectionInput {
  return {
    packages,
    edges: [],
    forcedPackages: [],
    unknownPathPolicy: "error",
    changedPaths: [],
    boundarySha256: "",
  };
}

describe("process-free graph and diff selection", () => {
  it("dynamically consumes the exact eight graph fixtures", () => {
    const documents = fixtures("graph");
    expect(documents.map((document) => document.id).sort()).toEqual(GRAPH_IDS);

    for (const document of documents) {
      const options = document.input.options;
      assertFixture(
        document,
        evaluateGraph({ packages: options.packages, edges: options.edges }),
      );
    }
  });

  it("dynamically consumes the exact twelve diff-selection fixtures", () => {
    const documents = fixtures("diff-selection");
    expect(documents.map((document) => document.id).sort()).toEqual(DIFF_IDS);

    for (const document of documents) {
      assertFixture(document, evaluateDiffSelection(diffInput(document)));
    }
  });

  it("fans one shared repository input out to both registered consumers", () => {
    const document = fixtures("diff-selection").find(
      (fixture) => fixture.id === "diff-selection/shared-input-multiconsumer",
    );
    expect(document).toBeDefined();
    const actual = evaluateDiffSelection(diffInput(document!));
    expect(actual).toEqual({
      ok: true,
      value: {
        changed_packages: ["swift/conduit", "swift/sha256-native"],
        affected_packages: ["swift/app", "swift/conduit", "swift/sha256-native"],
        prerequisite_packages: ["swift/base"],
      },
    });
  });

  it("returns stable graph failures without partial levels", () => {
    expect(
      evaluateGraph({
        packages: ["fixture/a", "fixture/b", "fixture/free"],
        edges: [
          ["fixture/a", "fixture/b"],
          ["fixture/b", "fixture/a"],
        ],
      }),
    ).toEqual({ ok: false, code: "GRAPH_CYCLE" });
    expect(
      evaluateGraph({ packages: ["fixture/a", "fixture/a"], edges: [] }),
    ).toEqual({ ok: false, code: "GRAPH_PACKAGE_DUPLICATE" });
  });

  it("rejects non-adjacent root ancestry and Unicode fold collisions", () => {
    expect(
      evaluateDiffSelection(
        simpleDiff([
          packagePrefix("fixture/root", "a"),
          packagePrefix("fixture/punctuation", "a!"),
          packagePrefix("fixture/descendant", "a/x"),
        ]),
      ),
    ).toEqual({ ok: false, code: "DIFF_PATH_INVALID" });

    expect(
      evaluateDiffSelection(
        simpleDiff([
          packagePrefix("fixture/eszett", "code/ß"),
          packagePrefix("fixture/double-s", "code/ss"),
        ]),
      ),
    ).toEqual({ ok: false, code: "DIFF_PATH_INVALID" });
  });

  it("rejects shared and cyclic decoded boundary object graphs before digesting", () => {
    const leaf = { payload: "x".repeat(32) };
    let shared: unknown = leaf;
    for (let level = 0; level < 12; level += 1) shared = [shared, shared];
    const aliasedBoundary = {
      schema_version: 1,
      language_source_input_registry_sha256: "0".repeat(64),
      boundaries: shared,
    };
    const cyclicBoundary: Record<string, unknown> = {};
    cyclicBoundary.self = cyclicBoundary;

    const base = simpleDiff([packagePrefix("fixture/a", "code/a")]);
    expect(
      evaluateDiffSelection({
        ...base,
        boundarySha256: "0".repeat(64),
        boundary: aliasedBoundary,
      }),
    ).toEqual({ ok: false, code: "DIFF_BOUNDARY_INVALID" });
    expect(
      evaluateDiffSelection({
        ...base,
        boundarySha256: "0".repeat(64),
        boundary: cyclicBoundary,
      }),
    ).toEqual({ ok: false, code: "DIFF_BOUNDARY_INVALID" });
  });

  it("does not expose forced or local partial selection on an unknown path", () => {
    expect(
      evaluateDiffSelection({
        packages: [
          packagePrefix("fixture/a", "code/a"),
          packagePrefix("fixture/b", "code/b"),
        ],
        edges: [["fixture/a", "fixture/b"]],
        forcedPackages: ["fixture/a"],
        unknownPathPolicy: "error",
        changedPaths: ["code/b/file.ts", "docs/unknown.md"],
        boundarySha256: "",
      }),
    ).toEqual({ ok: false, code: "DIFF_UNKNOWN_PATH" });
  });

  it("retains the hardened single-scalar question-mark glob", () => {
    expect(
      evaluateDiffSelection({
        packages: [
          {
            name: "fixture/question",
            relPath: "code/question",
            sourceMode: "strict_globs",
            sourceGlobs: ["src/?.ts"],
          },
        ],
        edges: [],
        forcedPackages: [],
        unknownPathPolicy: "error",
        changedPaths: ["code/question/src/😀.ts"],
        boundarySha256: "",
      }),
    ).toEqual({
      ok: true,
      value: {
        changed_packages: ["fixture/question"],
        affected_packages: ["fixture/question"],
        prerequisite_packages: [],
      },
    });
  });

  it("rejects decoded boundary accessors without invoking them", () => {
    let invoked = 0;
    const boundary: Record<string, unknown> = {
      schema_version: 1,
      language_source_input_registry_sha256: "0".repeat(64),
    };
    Object.defineProperty(boundary, "boundaries", {
      enumerable: true,
      get() {
        invoked += 1;
        return [];
      },
    });
    const base = simpleDiff([packagePrefix("fixture/a", "code/a")]);
    expect(
      evaluateDiffSelection({
        ...base,
        boundarySha256: "0".repeat(64),
        boundary,
      }),
    ).toEqual({ ok: false, code: "DIFF_BOUNDARY_INVALID" });
    expect(invoked).toBe(0);
  });

  it("snapshots the caller's boundary property exactly once", () => {
    const base = simpleDiff([packagePrefix("fixture/a", "code/a")]);
    let reads = 0;
    const input = { ...base } as DiffSelectionInput;
    Object.defineProperty(input, "boundarySha256", {
      enumerable: true,
      value: "0".repeat(64),
    });
    Object.defineProperty(input, "boundary", {
      enumerable: true,
      get() {
        reads += 1;
        return {};
      },
    });
    expect(evaluateDiffSelection(input)).toEqual({
      ok: false,
      code: "DIFF_BOUNDARY_DIGEST_MISMATCH",
    });
    expect(reads).toBe(1);
  });
});
