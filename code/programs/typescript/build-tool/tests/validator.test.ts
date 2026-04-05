import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  validateBuildContracts,
  validateCIFullBuildToolchains,
} from "../src/validator.js";

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
