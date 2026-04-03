import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { validateCIFullBuildToolchains } from "../src/validator.js";

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
