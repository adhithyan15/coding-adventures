import { describe, it, expect, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  expandTemplate,
  jsonSafeString,
  generateFiles,
  scaffold,
} from "../src/scaffold/scaffold";

/**
 * Scaffold Generator Tests
 * ========================
 *
 * Three levels of testing:
 * 1. Template expansion — string replacement works correctly
 * 2. File generation — correct files with correct content are produced
 * 3. Disk writing — scaffold() creates the right directory structure
 */

describe("expandTemplate", () => {
  it("replaces single variable", () => {
    expect(expandTemplate("Hello, {{name}}!", { name: "World" })).toBe(
      "Hello, World!",
    );
  });

  it("replaces multiple occurrences of the same variable", () => {
    expect(
      expandTemplate("{{x}} + {{x}} = 2{{x}}", { x: "1" }),
    ).toBe("1 + 1 = 21");
  });

  it("replaces multiple different variables", () => {
    expect(
      expandTemplate("{{greeting}}, {{name}}!", {
        greeting: "Hi",
        name: "Alice",
      }),
    ).toBe("Hi, Alice!");
  });

  it("leaves unmatched placeholders unchanged", () => {
    expect(expandTemplate("{{known}} {{unknown}}", { known: "yes" })).toBe(
      "yes {{unknown}}",
    );
  });

  it("handles empty template", () => {
    expect(expandTemplate("", { name: "test" })).toBe("");
  });

  it("handles empty variables", () => {
    expect(expandTemplate("no vars here", {})).toBe("no vars here");
  });
});

describe("jsonSafeString", () => {
  it("escapes double quotes", () => {
    expect(jsonSafeString('hello "world"')).toBe('hello \\"world\\"');
  });

  it("escapes backslashes", () => {
    expect(jsonSafeString("path\\to\\file")).toBe("path\\\\to\\\\file");
  });

  it("escapes newlines", () => {
    expect(jsonSafeString("line1\nline2")).toBe("line1\\nline2");
  });

  it("escapes carriage returns and tabs", () => {
    expect(jsonSafeString("a\rb\tc")).toBe("a\\rb\\tc");
  });

  it("handles strings with no special characters", () => {
    expect(jsonSafeString("hello world")).toBe("hello world");
  });

  it("prevents JSON injection via description", () => {
    // An attacker might try to inject additional JSON fields
    const malicious = 'cool", "permissions": ["<all_urls>"]';
    const escaped = jsonSafeString(malicious);
    // The escaped string should be safely containable in JSON quotes
    const json = `{"description": "${escaped}"}`;
    const parsed = JSON.parse(json);
    expect(parsed.description).toBe(malicious);
    expect(parsed.permissions).toBeUndefined();
  });
});

describe("generateFiles", () => {
  it("generates the expected set of files", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "A test extension",
    });

    const paths = files.map((f) => f.path);

    // Core project files
    expect(paths).toContain("manifest.json");
    expect(paths).toContain("package.json");
    expect(paths).toContain("tsconfig.json");
    expect(paths).toContain("vite.config.ts");
    expect(paths).toContain("vitest.config.ts");
    expect(paths).toContain("BUILD");
    expect(paths).toContain("README.md");
    expect(paths).toContain("CHANGELOG.md");

    // Source files
    expect(paths).toContain("src/lib/browser-api.ts");
    expect(paths).toContain("src/popup/popup.html");
    expect(paths).toContain("src/popup/popup.ts");
    expect(paths).toContain("src/popup/popup.css");
    expect(paths).toContain("src/background/service-worker.ts");

    // Test files
    expect(paths).toContain("tests/popup.test.ts");
  });

  it("replaces name in generated content", () => {
    const files = generateFiles({
      name: "my-cool-ext",
      description: "Does cool things",
    });

    const manifest = files.find((f) => f.path === "manifest.json");
    expect(manifest).toBeDefined();
    expect(manifest!.content).toContain('"my-cool-ext"');
    expect(manifest!.content).not.toContain("{{name}}");
  });

  it("replaces description in generated content", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "My custom description",
    });

    const pkg = files.find((f) => f.path === "package.json");
    expect(pkg).toBeDefined();
    expect(pkg!.content).toContain("My custom description");
  });

  it("uses default gecko ID when not provided", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "Test",
    });

    const manifest = files.find((f) => f.path === "manifest.json");
    expect(manifest!.content).toContain("test-ext@coding-adventures");
  });

  it("uses custom gecko ID when provided", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "Test",
      geckoId: "custom-id@example.com",
    });

    const manifest = files.find((f) => f.path === "manifest.json");
    expect(manifest!.content).toContain("custom-id@example.com");
    expect(manifest!.content).not.toContain("test-ext@coding-adventures");
  });

  it("includes today's date in changelog", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "Test",
    });

    const changelog = files.find((f) => f.path === "CHANGELOG.md");
    const today = new Date().toISOString().split("T")[0];
    expect(changelog!.content).toContain(today);
  });

  it("generates valid JSON in manifest.json", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "Test",
    });

    const manifest = files.find((f) => f.path === "manifest.json");
    expect(() => JSON.parse(manifest!.content)).not.toThrow();
  });

  it("generates valid JSON in package.json", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "Test",
    });

    const pkg = files.find((f) => f.path === "package.json");
    expect(() => JSON.parse(pkg!.content)).not.toThrow();
  });

  it("manifest has manifest_version 3", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "Test",
    });

    const manifest = files.find((f) => f.path === "manifest.json");
    const parsed = JSON.parse(manifest!.content);
    expect(parsed.manifest_version).toBe(3);
  });

  it("package.json includes toolkit dependency", () => {
    const files = generateFiles({
      name: "test-ext",
      description: "Test",
    });

    const pkg = files.find((f) => f.path === "package.json");
    const parsed = JSON.parse(pkg!.content);
    expect(
      parsed.dependencies["@coding-adventures/browser-extension-toolkit"],
    ).toBeDefined();
  });
});

describe("scaffold", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "scaffold-test-"));
  });

  afterEach(() => {
    // Clean up temp directory
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("creates extension directory with all files", () => {
    const result = scaffold({
      name: "test-ext",
      description: "Test extension",
      outputDir: tmpDir,
    });

    expect(fs.existsSync(result)).toBe(true);
    expect(fs.existsSync(path.join(result, "manifest.json"))).toBe(true);
    expect(fs.existsSync(path.join(result, "package.json"))).toBe(true);
    expect(fs.existsSync(path.join(result, "src/popup/popup.html"))).toBe(true);
    expect(fs.existsSync(path.join(result, "src/popup/popup.ts"))).toBe(true);
    expect(fs.existsSync(path.join(result, "src/popup/popup.css"))).toBe(true);
    expect(
      fs.existsSync(path.join(result, "src/background/service-worker.ts")),
    ).toBe(true);
    expect(fs.existsSync(path.join(result, "tests/popup.test.ts"))).toBe(true);
  });

  it("returns the path to the created directory", () => {
    const result = scaffold({
      name: "my-ext",
      description: "Test",
      outputDir: tmpDir,
    });

    expect(result).toBe(path.join(tmpDir, "my-ext"));
  });

  it("throws if directory already exists", () => {
    // Create the directory first
    fs.mkdirSync(path.join(tmpDir, "existing-ext"));

    expect(() =>
      scaffold({
        name: "existing-ext",
        description: "Test",
        outputDir: tmpDir,
      }),
    ).toThrow("already exists");
  });

  it("created files have correct content", () => {
    const result = scaffold({
      name: "content-test",
      description: "Testing content",
      outputDir: tmpDir,
    });

    const manifest = JSON.parse(
      fs.readFileSync(path.join(result, "manifest.json"), "utf-8"),
    );
    expect(manifest.name).toBe("content-test");
    expect(manifest.manifest_version).toBe(3);

    const pkg = JSON.parse(
      fs.readFileSync(path.join(result, "package.json"), "utf-8"),
    );
    expect(pkg.name).toBe("content-test");
    expect(pkg.description).toBe("Testing content");
  });

  it("rejects names with path traversal characters", () => {
    expect(() =>
      scaffold({
        name: "../../../etc/malicious",
        description: "Test",
        outputDir: tmpDir,
      }),
    ).toThrow("Invalid extension name");
  });

  it("rejects names with dots", () => {
    expect(() =>
      scaffold({
        name: "..evil",
        description: "Test",
        outputDir: tmpDir,
      }),
    ).toThrow("Invalid extension name");
  });

  it("rejects names with spaces", () => {
    expect(() =>
      scaffold({
        name: "my extension",
        description: "Test",
        outputDir: tmpDir,
      }),
    ).toThrow("Invalid extension name");
  });

  it("accepts valid kebab-case names", () => {
    const result = scaffold({
      name: "valid-extension-name",
      description: "Test",
      outputDir: tmpDir,
    });
    expect(fs.existsSync(result)).toBe(true);
  });

  it("accepts names with underscores", () => {
    const result = scaffold({
      name: "valid_name_123",
      description: "Test",
      outputDir: tmpDir,
    });
    expect(fs.existsSync(result)).toBe(true);
  });
});

// Need to import beforeEach for the scaffold describe block
import { beforeEach } from "vitest";
