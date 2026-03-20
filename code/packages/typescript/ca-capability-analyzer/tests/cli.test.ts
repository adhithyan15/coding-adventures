/**
 * # CLI Tests
 *
 * Tests for the command-line interface. We test the exported `main()` function
 * directly, plus the individual subcommand functions via file-based integration
 * tests using temp files.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { main } from "../src/cli.js";

// Helper to create a temp directory with test files
function createTempProject(): { dir: string; cleanup: () => void } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ca-analyzer-test-"));

  // A file with capabilities
  fs.writeFileSync(
    path.join(dir, "cap.ts"),
    `import * as fs from "fs";\nfs.readFileSync("data.txt");\n`
  );

  // A pure file
  fs.writeFileSync(
    path.join(dir, "pure.ts"),
    `const x = 1 + 2;\nconst y = x * 3;\n`
  );

  // A file with banned constructs
  fs.writeFileSync(
    path.join(dir, "evil.ts"),
    `eval("1 + 2");\nnew Function("return 1");\n`
  );

  // A manifest
  fs.writeFileSync(
    path.join(dir, "manifest.json"),
    JSON.stringify({
      version: 1,
      package: "typescript/test-pkg",
      capabilities: [
        { category: "fs", action: "*", target: "*" },
      ],
      justification: "Test package",
    })
  );

  // An empty manifest (default deny)
  fs.writeFileSync(
    path.join(dir, "empty-manifest.json"),
    JSON.stringify({
      version: 1,
      package: "typescript/test-pkg",
      capabilities: [],
      justification: "No capabilities",
    })
  );

  return {
    dir,
    cleanup: () => fs.rmSync(dir, { recursive: true, force: true }),
  };
}

describe("CLI main()", () => {
  let consoleSpy: ReturnType<typeof vi.spyOn>;
  let consoleErrSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
    consoleErrSpy = vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    consoleSpy.mockRestore();
    consoleErrSpy.mockRestore();
  });

  it("shows help with no args", () => {
    const code = main([]);
    expect(code).toBe(0);
  });

  it("shows help with --help", () => {
    const code = main(["--help"]);
    expect(code).toBe(0);
  });

  it("returns 1 for unknown command", () => {
    const code = main(["foobar"]);
    expect(code).toBe(1);
  });
});

describe("CLI detect", () => {
  let project: ReturnType<typeof createTempProject>;
  let consoleSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    project = createTempProject();
    consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    project.cleanup();
    vi.restoreAllMocks();
  });

  it("detects capabilities in a file", () => {
    const code = main(["detect", path.join(project.dir, "cap.ts")]);
    expect(code).toBe(0);
    const output = consoleSpy.mock.calls.map((c) => c[0]).join("\n");
    expect(output).toContain("fs:");
  });

  it("reports nothing for pure file", () => {
    const code = main(["detect", path.join(project.dir, "pure.ts")]);
    expect(code).toBe(0);
    const output = consoleSpy.mock.calls.map((c) => c[0]).join("\n");
    expect(output).toContain("0 capabilities");
  });

  it("returns 1 with no file args", () => {
    const code = main(["detect"]);
    expect(code).toBe(1);
  });
});

describe("CLI banned", () => {
  let project: ReturnType<typeof createTempProject>;
  let consoleSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    project = createTempProject();
    consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    project.cleanup();
    vi.restoreAllMocks();
  });

  it("finds banned constructs", () => {
    const code = main(["banned", path.join(project.dir, "evil.ts")]);
    expect(code).toBe(1);
    const output = consoleSpy.mock.calls.map((c) => c[0]).join("\n");
    expect(output).toContain("FOUND");
  });

  it("passes on clean file", () => {
    const code = main(["banned", path.join(project.dir, "pure.ts")]);
    expect(code).toBe(0);
    const output = consoleSpy.mock.calls.map((c) => c[0]).join("\n");
    expect(output).toContain("No banned constructs");
  });

  it("returns 1 with no file args", () => {
    const code = main(["banned"]);
    expect(code).toBe(1);
  });
});

describe("CLI check", () => {
  let project: ReturnType<typeof createTempProject>;
  let consoleSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    project = createTempProject();
    consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    project.cleanup();
    vi.restoreAllMocks();
  });

  it("passes with matching manifest", () => {
    const code = main([
      "check",
      path.join(project.dir, "manifest.json"),
      path.join(project.dir, "cap.ts"),
    ]);
    expect(code).toBe(0);
  });

  it("fails with empty manifest (default deny)", () => {
    const code = main([
      "check",
      path.join(project.dir, "empty-manifest.json"),
      path.join(project.dir, "cap.ts"),
    ]);
    expect(code).toBe(1);
    const output = consoleSpy.mock.calls.map((c) => c[0]).join("\n");
    expect(output).toContain("UNDECLARED");
  });

  it("passes for pure file with empty manifest", () => {
    const code = main([
      "check",
      path.join(project.dir, "empty-manifest.json"),
      path.join(project.dir, "pure.ts"),
    ]);
    expect(code).toBe(0);
  });

  it("returns 1 with insufficient args", () => {
    const code = main(["check"]);
    expect(code).toBe(1);
  });

  it("returns 1 with only manifest arg", () => {
    const code = main(["check", path.join(project.dir, "manifest.json")]);
    expect(code).toBe(1);
  });

  it("returns 1 for missing manifest file", () => {
    const code = main([
      "check",
      path.join(project.dir, "nonexistent.json"),
      path.join(project.dir, "cap.ts"),
    ]);
    expect(code).toBe(1);
  });

  it("reports unused declarations", () => {
    const code = main([
      "check",
      path.join(project.dir, "manifest.json"),
      path.join(project.dir, "pure.ts"),
    ]);
    expect(code).toBe(0);
    const output = consoleSpy.mock.calls.map((c) => c[0]).join("\n");
    expect(output).toContain("Unused");
  });
});
