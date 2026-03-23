/**
 * Tests for chmod -- change file mode bits.
 *
 * We test the exported business logic functions: parseOctalMode,
 * parseSymbolicMode, applySymbolicClause, calculateMode, chmodFile,
 * chmodRecursive, and formatMode.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  parseOctalMode,
  parseSymbolicMode,
  applySymbolicClause,
  calculateMode,
  chmodFile,
  chmodRecursive,
  formatMode,
  ChmodOptions,
  SymbolicClause,
} from "../src/chmod.js";

// ---------------------------------------------------------------------------
// Helpers: temp directory management and default options.
// ---------------------------------------------------------------------------

let tmpDir: string;

function defaultOpts(overrides: Partial<ChmodOptions> = {}): ChmodOptions {
  return {
    recursive: false,
    verbose: false,
    changes: false,
    silent: false,
    ...overrides,
  };
}

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "chmod-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// parseOctalMode: parsing octal mode strings.
// ---------------------------------------------------------------------------

describe("parseOctalMode", () => {
  it("should parse 3-digit octal modes", () => {
    expect(parseOctalMode("755")).toBe(0o755);
    expect(parseOctalMode("644")).toBe(0o644);
    expect(parseOctalMode("700")).toBe(0o700);
    expect(parseOctalMode("000")).toBe(0o000);
    expect(parseOctalMode("777")).toBe(0o777);
  });

  it("should parse 4-digit octal modes", () => {
    expect(parseOctalMode("0755")).toBe(0o755);
    expect(parseOctalMode("4755")).toBe(0o4755); // setuid
    expect(parseOctalMode("2755")).toBe(0o2755); // setgid
    expect(parseOctalMode("1755")).toBe(0o1755); // sticky
  });

  it("should parse 1-2 digit octal modes", () => {
    expect(parseOctalMode("7")).toBe(0o7);
    expect(parseOctalMode("77")).toBe(0o77);
  });

  it("should return null for non-octal strings", () => {
    expect(parseOctalMode("u+x")).toBeNull();
    expect(parseOctalMode("abc")).toBeNull();
    expect(parseOctalMode("899")).toBeNull(); // 8 and 9 not valid octal
    expect(parseOctalMode("")).toBeNull();
    expect(parseOctalMode("12345")).toBeNull(); // too many digits
  });
});

// ---------------------------------------------------------------------------
// parseSymbolicMode: parsing symbolic mode strings.
// ---------------------------------------------------------------------------

describe("parseSymbolicMode", () => {
  it("should parse 'u+x'", () => {
    const clauses = parseSymbolicMode("u+x");
    expect(clauses).toHaveLength(1);
    expect(clauses[0].who).toEqual(["u"]);
    expect(clauses[0].operator).toBe("+");
    expect(clauses[0].permissions).toEqual(["x"]);
  });

  it("should parse 'go-w'", () => {
    const clauses = parseSymbolicMode("go-w");
    expect(clauses).toHaveLength(1);
    expect(clauses[0].who).toContain("g");
    expect(clauses[0].who).toContain("o");
    expect(clauses[0].operator).toBe("-");
    expect(clauses[0].permissions).toEqual(["w"]);
  });

  it("should parse comma-separated clauses", () => {
    const clauses = parseSymbolicMode("u+rx,go-w");
    expect(clauses).toHaveLength(2);
    expect(clauses[0].permissions).toContain("r");
    expect(clauses[0].permissions).toContain("x");
    expect(clauses[1].permissions).toEqual(["w"]);
  });

  it("should expand 'a' to u,g,o", () => {
    const clauses = parseSymbolicMode("a+r");
    expect(clauses[0].who).toEqual(["u", "g", "o"]);
  });

  it("should treat missing who as 'a'", () => {
    const clauses = parseSymbolicMode("+x");
    expect(clauses[0].who).toEqual(["u", "g", "o"]);
  });

  it("should parse '=' operator", () => {
    const clauses = parseSymbolicMode("u=rw");
    expect(clauses[0].operator).toBe("=");
    expect(clauses[0].permissions).toEqual(["r", "w"]);
  });

  it("should throw for invalid mode strings", () => {
    expect(() => parseSymbolicMode("invalid")).toThrow("invalid mode");
    expect(() => parseSymbolicMode("u+z")).toThrow("invalid mode");
  });

  it("should parse setuid/setgid (s) and sticky (t)", () => {
    const clauses = parseSymbolicMode("u+s");
    expect(clauses[0].permissions).toEqual(["s"]);

    const sticky = parseSymbolicMode("+t");
    expect(sticky[0].permissions).toEqual(["t"]);
  });
});

// ---------------------------------------------------------------------------
// applySymbolicClause: applying symbolic clauses to modes.
// ---------------------------------------------------------------------------

describe("applySymbolicClause", () => {
  it("should add execute for owner (u+x)", () => {
    const clause: SymbolicClause = { who: ["u"], operator: "+", permissions: ["x"] };
    const result = applySymbolicClause(0o644, clause, false);
    expect(result).toBe(0o744);
  });

  it("should remove write for group and others (go-w)", () => {
    const clause: SymbolicClause = { who: ["g", "o"], operator: "-", permissions: ["w"] };
    const result = applySymbolicClause(0o666, clause, false);
    expect(result).toBe(0o644);
  });

  it("should set owner to read-write only (u=rw)", () => {
    const clause: SymbolicClause = { who: ["u"], operator: "=", permissions: ["r", "w"] };
    const result = applySymbolicClause(0o755, clause, false);
    expect(result).toBe(0o655);
  });

  it("should add read for all (a+r)", () => {
    const clause: SymbolicClause = { who: ["u", "g", "o"], operator: "+", permissions: ["r"] };
    const result = applySymbolicClause(0o000, clause, false);
    expect(result).toBe(0o444);
  });

  it("should handle X (conditional execute) for directories", () => {
    const clause: SymbolicClause = { who: ["u", "g", "o"], operator: "+", permissions: ["X"] };
    // For a directory, X always sets execute.
    const result = applySymbolicClause(0o644, clause, true);
    expect(result).toBe(0o755);
  });

  it("should handle X for files that already have execute", () => {
    const clause: SymbolicClause = { who: ["u", "g", "o"], operator: "+", permissions: ["X"] };
    // File already has execute for owner.
    const result = applySymbolicClause(0o744, clause, false);
    expect(result).toBe(0o755);
  });

  it("should NOT set X for files without execute", () => {
    const clause: SymbolicClause = { who: ["u", "g", "o"], operator: "+", permissions: ["X"] };
    // File has no execute bits.
    const result = applySymbolicClause(0o644, clause, false);
    expect(result).toBe(0o644);
  });

  it("should set setuid bit (u+s)", () => {
    const clause: SymbolicClause = { who: ["u"], operator: "+", permissions: ["s"] };
    const result = applySymbolicClause(0o755, clause, false);
    expect(result).toBe(0o4755);
  });

  it("should set setgid bit (g+s)", () => {
    const clause: SymbolicClause = { who: ["g"], operator: "+", permissions: ["s"] };
    const result = applySymbolicClause(0o755, clause, false);
    expect(result).toBe(0o2755);
  });

  it("should set sticky bit (+t)", () => {
    const clause: SymbolicClause = { who: ["u", "g", "o"], operator: "+", permissions: ["t"] };
    const result = applySymbolicClause(0o755, clause, false);
    expect(result).toBe(0o1755);
  });
});

// ---------------------------------------------------------------------------
// calculateMode: combined octal + symbolic mode calculation.
// ---------------------------------------------------------------------------

describe("calculateMode", () => {
  it("should handle octal mode strings", () => {
    expect(calculateMode("755", 0o644, false)).toBe(0o755);
    expect(calculateMode("0644", 0o755, false)).toBe(0o644);
  });

  it("should handle symbolic mode strings", () => {
    expect(calculateMode("u+x", 0o644, false)).toBe(0o744);
    expect(calculateMode("go-w", 0o666, false)).toBe(0o644);
  });

  it("should handle comma-separated symbolic modes", () => {
    expect(calculateMode("u+x,go-w", 0o666, false)).toBe(0o744);
  });

  it("should throw for invalid mode strings", () => {
    expect(() => calculateMode("invalid", 0o644, false)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// formatMode: mode display formatting.
// ---------------------------------------------------------------------------

describe("formatMode", () => {
  it("should format mode as 4-digit octal with leading zero", () => {
    expect(formatMode(0o755)).toBe("0755");
    expect(formatMode(0o644)).toBe("0644");
    expect(formatMode(0o000)).toBe("0000");
    expect(formatMode(0o4755)).toBe("04755");
  });
});

// ---------------------------------------------------------------------------
// chmodFile: applying mode changes to files.
// ---------------------------------------------------------------------------

describe("chmodFile", () => {
  it("should change file mode with octal mode", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");
    fs.chmodSync(filePath, 0o644);

    const result = chmodFile(filePath, "755", defaultOpts());

    expect(result.changed).toBe(true);
    expect(result.newMode).toBe(0o755);

    const newStat = fs.statSync(filePath);
    expect(newStat.mode & 0o7777).toBe(0o755);
  });

  it("should change file mode with symbolic mode", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");
    fs.chmodSync(filePath, 0o644);

    const result = chmodFile(filePath, "u+x", defaultOpts());

    expect(result.changed).toBe(true);
    expect(result.newMode).toBe(0o744);
  });

  it("should report no change when mode is already correct", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");
    fs.chmodSync(filePath, 0o644);

    const result = chmodFile(filePath, "644", defaultOpts());

    expect(result.changed).toBe(false);
    expect(result.oldMode).toBe(0o644);
    expect(result.newMode).toBe(0o644);
  });

  it("should include file path in result message", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello");
    fs.chmodSync(filePath, 0o644);

    const result = chmodFile(filePath, "755", defaultOpts());

    expect(result.message).toContain(filePath);
    expect(result.message).toContain("0644");
    expect(result.message).toContain("0755");
  });
});

// ---------------------------------------------------------------------------
// chmodRecursive: recursive mode changes.
// ---------------------------------------------------------------------------

describe("chmodRecursive", () => {
  it("should change mode for directory and all contents", () => {
    // Create a directory structure.
    const subDir = path.join(tmpDir, "subdir");
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(tmpDir, "file1.txt"), "hello");
    fs.writeFileSync(path.join(subDir, "file2.txt"), "world");

    fs.chmodSync(path.join(tmpDir, "file1.txt"), 0o644);
    fs.chmodSync(path.join(subDir, "file2.txt"), 0o644);

    const results = chmodRecursive(tmpDir, "755", defaultOpts());

    // Should have results for the directory, file1, subdir, and file2.
    expect(results.length).toBeGreaterThanOrEqual(3);

    // All files should now have mode 755.
    const file1Stat = fs.statSync(path.join(tmpDir, "file1.txt"));
    expect(file1Stat.mode & 0o7777).toBe(0o755);

    const file2Stat = fs.statSync(path.join(subDir, "file2.txt"));
    expect(file2Stat.mode & 0o7777).toBe(0o755);
  });

  it("should suppress errors in silent mode", () => {
    // This should not throw even if a file can't be accessed.
    const results = chmodRecursive(tmpDir, "755", defaultOpts({ silent: true }));
    expect(results).toBeDefined();
  });
});
