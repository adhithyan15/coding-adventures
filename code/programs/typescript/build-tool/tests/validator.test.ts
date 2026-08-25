import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  TRACKED_ARTIFACT_UNICODE_VERSION,
  type TrackedArtifactDiagnostic,
  type TrackedArtifactEntry,
  validateBuildContracts,
  validateCIFullBuildToolchains,
  validateTrackedArtifactSnapshot,
} from "../src/validator.js";
import {
  fullUppercase,
  nfc,
  nfkcCasefold,
} from "../src/tracked-artifact-unicode17.js";

const conformanceCases = new URL(
  "../../../../specs/fixtures/build-tool-v1/cases/",
  import.meta.url,
);

const trackedArtifactCases = [
  "validation-tracked-artifacts-clean.json",
  "validation-tracked-artifacts-forbidden.json",
  "validation-tracked-artifacts-aliases.json",
  "validation-tracked-artifacts-invalid.json",
  "validation-tracked-artifacts-unicode-boundaries.json",
] as const;

function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-validator-"));
}

function rmDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

function writeFile(filepath: string, content: string): void {
  fs.mkdirSync(path.dirname(filepath), { recursive: true });
  fs.writeFileSync(filepath, content, "utf-8");
}

describe("validateCIFullBuildToolchains", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("fails when forced full-build toolchains are not normalized", () => {
    writeFile(
      path.join(tmpDir, ".github", "workflows", "ci.yml"),
      `
jobs:
  detect:
    outputs:
      needs_python: \${{ steps.detect.outputs.needs_python }}
      needs_elixir: \${{ steps.detect.outputs.needs_elixir }}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
`,
    );

    const error = validateCIFullBuildToolchains(tmpDir, [
      { language: "elixir" },
      { language: "python" },
    ]);

    expect(error).toContain(".github/workflows/ci.yml");
    expect(error).toContain("elixir");
    expect(error).toContain("python");
  });

  it("allows normalized full-build toolchains", () => {
    writeFile(
      path.join(tmpDir, ".github", "workflows", "ci.yml"),
      `
jobs:
  detect:
    outputs:
      needs_python: \${{ steps.toolchains.outputs.needs_python }}
      needs_elixir: \${{ steps.toolchains.outputs.needs_elixir }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\\n' \\
            'needs_python=true' \\
            'needs_elixir=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
`,
    );

    expect(
      validateCIFullBuildToolchains(tmpDir, [
        { language: "elixir" },
        { language: "python" },
      ]),
    ).toBeNull();
  });
});

describe("validateBuildContracts", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("flags Lua isolated-build violations", () => {
    writeFile(
      path.join(tmpDir, "code", "packages", "lua", "problem_pkg", "BUILD"),
      `
luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
`,
    );

    const error = validateBuildContracts(tmpDir, [
      { language: "lua", path: path.join(tmpDir, "code/packages/lua/problem_pkg") },
    ]);

    expect(error).toContain("coding-adventures-branch-predictor");
    expect(error).toContain("state_machine before directed_graph");
  });

  it("flags guarded Lua installs without deps-mode none", () => {
    writeFile(
      path.join(tmpDir, "code", "packages", "lua", "guarded_pkg", "BUILD"),
      `
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
`,
    );

    const error = validateBuildContracts(tmpDir, [
      { language: "lua", path: path.join(tmpDir, "code/packages/lua/guarded_pkg") },
    ]);

    expect(error).toContain("--deps-mode=none or --no-manifest");
  });

  it("flags Windows Lua sibling drift", () => {
    const packagePath = path.join(
      tmpDir,
      "code",
      "packages",
      "lua",
      "arm1_gatelevel",
    );

    writeFile(
      path.join(packagePath, "BUILD"),
      `
(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
`,
    );
    writeFile(
      path.join(packagePath, "BUILD_windows"),
      `
(cd ..\\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
`,
    );

    const error = validateBuildContracts(tmpDir, [
      { language: "lua", path: packagePath },
    ]);

    expect(error).toContain("BUILD_windows is missing sibling installs present in BUILD");
    expect(error).toContain("../logic_gates");
    expect(error).toContain("../arithmetic");
    expect(error).toContain("--deps-mode=none or --no-manifest");
  });

  it("flags Perl Test2 bootstraps without --notest", () => {
    const packagePath = path.join(
      tmpDir,
      "code",
      "packages",
      "perl",
      "draw-instructions-svg",
    );

    writeFile(
      path.join(packagePath, "BUILD"),
      `
cpanm --quiet Test2::V0
prove -l -I../draw-instructions/lib -v t/
`,
    );

    const error = validateBuildContracts(tmpDir, [
      { language: "perl", path: packagePath },
    ]);

    expect(error).toContain("Test2::V0 without --notest");
  });

  it("validateCIFullBuildToolchains returns null when no ci.yml exists", () => {
    // Branch coverage for `if (!fs.existsSync(ciPath)) return null` (line 24).
    expect(validateCIFullBuildToolchains(tmpDir, [{ language: "python" }])).toBeNull();
  });

  it("validateCIFullBuildToolchains returns null when the marker comment is absent", () => {
    // Branch coverage for `if (!workflow.includes("Full build on main merge"))` (line 29).
    writeFile(
      path.join(tmpDir, ".github", "workflows", "ci.yml"),
      "jobs:\n  detect:\n    runs-on: ubuntu-latest\n",
    );
    expect(validateCIFullBuildToolchains(tmpDir, [{ language: "python" }])).toBeNull();
  });

  it("validateBuildContracts surfaces CI errors alongside Lua/Perl errors", () => {
    // Branch coverage for `if (ciError !== null) errors.push(ciError)` (line 77)
    // by crafting both a CI gap and a Perl issue in one call.
    writeFile(
      path.join(tmpDir, ".github", "workflows", "ci.yml"),
      `
jobs:
  detect:
    outputs: {}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force
`,
    );
    const perlPath = path.join(tmpDir, "code", "packages", "perl", "x");
    writeFile(path.join(perlPath, "BUILD"), "cpanm --quiet Test2::V0\nprove -l -v t/\n");
    const errors = validateBuildContracts(tmpDir, [
      { language: "python", path: path.join(tmpDir, "py") },
      { language: "perl", path: perlPath },
    ]);
    expect(errors).toContain("ci.yml");
    expect(errors).toContain("Test2::V0");
  });

  it("Lua validator tolerates empty BUILD files", () => {
    // Branch coverage for `if (lines.length === 0) continue` (line 120).
    const luaPath = path.join(tmpDir, "code", "packages", "lua", "empty_pkg");
    writeFile(path.join(luaPath, "BUILD"), "");
    expect(validateBuildContracts(tmpDir, [{ language: "lua", path: luaPath }])).toBeNull();
  });

  it("allows safe Lua isolated-build patterns", () => {
    const safePath = path.join(tmpDir, "code", "packages", "lua", "safe_pkg");

    writeFile(
      path.join(safePath, "BUILD"),
      `
luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
`,
    );
    writeFile(
      path.join(safePath, "BUILD_windows"),
      `
luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
`,
    );

    expect(
      validateBuildContracts(tmpDir, [
        { language: "lua", path: safePath },
      ]),
    ).toBeNull();
  });
});

describe("validateTrackedArtifactSnapshot", () => {
  for (const fixtureName of trackedArtifactCases) {
    it(`matches the shared ${fixtureName} fixture`, () => {
      const fixture = JSON.parse(
        fs.readFileSync(new URL(fixtureName, conformanceCases), "utf-8"),
      ) as {
        input: {
          options: {
            tracked_artifact_snapshot: {
              unicode_version: string;
              entries: TrackedArtifactEntry[];
            };
          };
        };
        expected: { diagnostics: TrackedArtifactDiagnostic[] };
      };
      const snapshot = fixture.input.options.tracked_artifact_snapshot;

      expect(
        validateTrackedArtifactSnapshot(
          snapshot.entries,
          snapshot.unicode_version,
        ),
      ).toEqual(fixture.expected.diagnostics);
    });
  }

  it("rejects Unicode-version drift before examining entries", () => {
    expect(TRACKED_ARTIFACT_UNICODE_VERSION).toBe("17.0.0");
    expect(() =>
      validateTrackedArtifactSnapshot(
        [{ ordinal: 1, path: "/hostile", entry_kind: "regular" }],
        "15.1.0",
      ),
    ).toThrow("tracked artifact Unicode version must be 17.0.0");
  });

  const unsafePaths = [
    ["", "EMPTY"],
    ["a".repeat(513), "TOO_LONG"],
    ["code/packages/e\u0301/file.ts", "NON_NFC"],
    ["/absolute/file.ts", "ABSOLUTE"],
    ["C:\\repo\\file.ts", "DRIVE_QUALIFIED"],
    ["code//file.ts", "EMPTY_SEGMENT"],
    ["code/trailing/", "EMPTY_SEGMENT"],
    ["code\\trailing\\", "EMPTY_SEGMENT"],
    ["code/<unsafe>/file.ts", "UNSAFE_CHARACTER"],
    ["code/../file.ts", "DOT_SEGMENT"],
    ["code/trailing./file.ts", "TRAILING_DOT_OR_SPACE"],
    ["code/CON.txt/file.ts", "RESERVED_BASENAME"],
  ] as const;

  for (const [unsafePath, expectedProblem] of unsafePaths) {
    it(`redacts the ${expectedProblem} path class`, () => {
      const diagnostic = validateTrackedArtifactSnapshot([
        { ordinal: 7, path: unsafePath, entry_kind: "regular" },
      ]);

      expect(diagnostic).toEqual([
        {
          code: "TRACKED_ARTIFACT_PATH_INVALID",
          severity: "error",
          path: "repository",
          details: {
            ordinal: 7,
            entry_kind: "regular",
            problem: expectedProblem,
          },
        },
      ]);
      if (unsafePath.length > 0) {
        expect(JSON.stringify(diagnostic)).not.toContain(unsafePath);
      }
    });
  }

  it("normalizes separators without using host path APIs", () => {
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 1, path: "code\\src\\file.ts", entry_kind: "regular" },
      ]),
    ).toEqual([]);
  });

  it("counts Unicode scalars rather than UTF-16 code units", () => {
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 1, path: "😀".repeat(512), entry_kind: "regular" },
      ]),
    ).toEqual([]);
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 1, path: "😀".repeat(513), entry_kind: "regular" },
      ])[0]?.details.problem,
    ).toBe("TOO_LONG");
  });

  it("uses only the pinned Unicode 17 normalization and casing substrate", () => {
    const todhriSource = String.fromCodePoint(0x105d2) + "\u0307";
    const todhriComposed = String.fromCodePoint(0x105c9);
    expect(nfc(todhriSource)).toBe(todhriComposed);
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 1, path: todhriSource, entry_kind: "regular" },
      ])[0]?.details.problem,
    ).toBe("NON_NFC");

    const outlined = [..."NODE_MODULES"]
      .map((character) =>
        character === "_"
          ? character
          : String.fromCodePoint(0x1ccd6 + character.codePointAt(0)! - 0x41),
      )
      .join("");
    expect(nfkcCasefold(outlined)).toBe("node_modules");
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 2, path: `code/${outlined}/file.ts`, entry_kind: "regular" },
      ])[0]?.code,
    ).toBe("TRACKED_ARTIFACT_FORBIDDEN");

    expect(fullUppercase("conın$")).toBe("CONIN$");
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 3, path: "code/conın$.txt/file.ts", entry_kind: "regular" },
      ])[0]?.details.problem,
    ).toBe("RESERVED_BASENAME");

    expect(nfc("q\u0300")).toBe("q\u0300");
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 4, path: "q\u0300/file.ts", entry_kind: "regular" },
      ]),
    ).toEqual([]);
  });

  it("sorts diagnostic paths by Unicode scalar value", () => {
    const diagnostics = validateTrackedArtifactSnapshot([
      {
        ordinal: 1,
        path: `${String.fromCodePoint(0x10000)}/node_modules/a`,
        entry_kind: "regular",
      },
      {
        ordinal: 2,
        path: `${String.fromCodePoint(0xe000)}/node_modules/b`,
        entry_kind: "regular",
      },
    ]);

    expect(diagnostics.map((diagnostic) => diagnostic.path)).toEqual([
      `${String.fromCodePoint(0xe000)}/node_modules/b`,
      `${String.fromCodePoint(0x10000)}/node_modules/a`,
    ]);
  });

  it("treats entry kind as inert metadata", () => {
    expect(
      validateTrackedArtifactSnapshot([
        { ordinal: 1, path: "node_modules/a", entry_kind: "regular" },
        { ordinal: 2, path: "node_modules/b", entry_kind: "symlink" },
        { ordinal: 3, path: "node_modules/c", entry_kind: "reparse" },
      ]).map((diagnostic) => diagnostic.details.entry_kind),
    ).toEqual(["regular", "symlink", "reparse"]);
  });
});
