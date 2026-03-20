/**
 * # Manifest Tests
 *
 * Tests for manifest parsing, glob matching, and capability comparison.
 * These tests verify that the "declared vs. detected" logic works correctly.
 */

import { describe, it, expect } from "vitest";
import {
  parseManifest,
  simpleGlobMatch,
  capabilityMatchesDeclaration,
  compareCapabilities,
} from "../src/manifest.js";
import type { DetectedCapability } from "../src/analyzer.js";
import type { DeclaredCapability, CapabilityManifest } from "../src/manifest.js";

// ============================================================================
// Helper
// ============================================================================

/** Create a minimal DetectedCapability for testing */
function detected(
  category: string,
  action: string,
  target: string,
): DetectedCapability {
  return { category, action, target, file: "test.ts", line: 1, evidence: "test" };
}

/** Create a minimal DeclaredCapability for testing */
function declared(
  category: string,
  action: string,
  target: string,
): DeclaredCapability {
  return { category, action, target };
}

/** Create a minimal CapabilityManifest */
function manifest(caps: DeclaredCapability[]): CapabilityManifest {
  return { name: "test", version: "1.0.0", capabilities: caps };
}

// ============================================================================
// Manifest Parsing
// ============================================================================

describe("parseManifest", () => {
  it("parses valid manifest", () => {
    const m = parseManifest(JSON.stringify({
      name: "my-app",
      version: "1.0.0",
      capabilities: [
        { category: "fs", action: "read", target: "*" },
        { category: "net", action: "connect", target: "*" },
      ],
    }));
    expect(m.name).toBe("my-app");
    expect(m.version).toBe("1.0.0");
    expect(m.capabilities).toHaveLength(2);
    expect(m.capabilities[0].category).toBe("fs");
  });

  it("parses manifest with missing name/version", () => {
    const m = parseManifest(JSON.stringify({
      capabilities: [{ category: "fs", action: "read", target: "*" }],
    }));
    expect(m.name).toBe("");
    expect(m.version).toBe("");
    expect(m.capabilities).toHaveLength(1);
  });

  it("throws on invalid JSON", () => {
    expect(() => parseManifest("not json")).toThrow("Invalid JSON");
  });

  it("throws on non-object JSON", () => {
    expect(() => parseManifest("42")).toThrow("must be a JSON object");
  });

  it("throws on array JSON", () => {
    expect(() => parseManifest("[]")).toThrow("must be a JSON object");
  });

  it("throws on missing capabilities array", () => {
    expect(() => parseManifest(`{"name": "x"}`)).toThrow("must have a 'capabilities' array");
  });

  it("throws on non-object capability entry", () => {
    expect(() => parseManifest(`{"capabilities": ["bad"]}`)).toThrow("capabilities[0] must be an object");
  });

  it("throws on missing category", () => {
    expect(() =>
      parseManifest(`{"capabilities": [{"action": "read", "target": "*"}]}`)
    ).toThrow("capabilities[0].category must be a string");
  });

  it("throws on missing action", () => {
    expect(() =>
      parseManifest(`{"capabilities": [{"category": "fs", "target": "*"}]}`)
    ).toThrow("capabilities[0].action must be a string");
  });

  it("throws on missing target", () => {
    expect(() =>
      parseManifest(`{"capabilities": [{"category": "fs", "action": "read"}]}`)
    ).toThrow("capabilities[0].target must be a string");
  });
});

// ============================================================================
// Glob Matching
// ============================================================================

describe("simpleGlobMatch", () => {
  it("wildcard * matches anything", () => {
    expect(simpleGlobMatch("*", "anything")).toBe(true);
    expect(simpleGlobMatch("*", "")).toBe(true);
    expect(simpleGlobMatch("*", "/etc/passwd")).toBe(true);
  });

  it("exact match without glob", () => {
    expect(simpleGlobMatch("/etc/passwd", "/etc/passwd")).toBe(true);
    expect(simpleGlobMatch("/etc/passwd", "/etc/shadow")).toBe(false);
  });

  it("prefix glob /data/*", () => {
    expect(simpleGlobMatch("/data/*", "/data/file.csv")).toBe(true);
    expect(simpleGlobMatch("/data/*", "/data/sub/file.csv")).toBe(true);
    expect(simpleGlobMatch("/data/*", "/etc/file.csv")).toBe(false);
  });

  it("suffix glob *.txt", () => {
    expect(simpleGlobMatch("*.txt", "readme.txt")).toBe(true);
    expect(simpleGlobMatch("*.txt", "readme.md")).toBe(false);
    expect(simpleGlobMatch("*.txt", "/path/to/file.txt")).toBe(true);
  });

  it("middle glob /data/*.csv", () => {
    expect(simpleGlobMatch("/data/*.csv", "/data/sales.csv")).toBe(true);
    expect(simpleGlobMatch("/data/*.csv", "/data/sales.txt")).toBe(false);
  });

  it("escapes regex special characters", () => {
    // The dot in "file.txt" should not match any character
    expect(simpleGlobMatch("file.txt", "fileXtxt")).toBe(false);
    expect(simpleGlobMatch("file.txt", "file.txt")).toBe(true);
  });

  it("handles pattern with multiple *", () => {
    expect(simpleGlobMatch("/*/data/*", "/home/data/file")).toBe(true);
    expect(simpleGlobMatch("/*/data/*", "/tmp/data/other")).toBe(true);
  });
});

// ============================================================================
// Capability Matching
// ============================================================================

describe("capabilityMatchesDeclaration", () => {
  it("matches exact category:action:target", () => {
    expect(
      capabilityMatchesDeclaration(
        detected("fs", "read", "/etc/passwd"),
        declared("fs", "read", "/etc/passwd"),
      ),
    ).toBe(true);
  });

  it("matches with wildcard target", () => {
    expect(
      capabilityMatchesDeclaration(
        detected("fs", "read", "/etc/passwd"),
        declared("fs", "read", "*"),
      ),
    ).toBe(true);
  });

  it("matches with wildcard action", () => {
    expect(
      capabilityMatchesDeclaration(
        detected("fs", "write", "/tmp/x"),
        declared("fs", "*", "*"),
      ),
    ).toBe(true);
  });

  it("rejects category mismatch", () => {
    expect(
      capabilityMatchesDeclaration(
        detected("net", "connect", "*"),
        declared("fs", "read", "*"),
      ),
    ).toBe(false);
  });

  it("rejects action mismatch", () => {
    expect(
      capabilityMatchesDeclaration(
        detected("fs", "write", "/tmp/x"),
        declared("fs", "read", "*"),
      ),
    ).toBe(false);
  });

  it("matches with glob target pattern", () => {
    expect(
      capabilityMatchesDeclaration(
        detected("fs", "read", "/data/sales.csv"),
        declared("fs", "read", "/data/*.csv"),
      ),
    ).toBe(true);
  });

  it("rejects glob target that doesn't match", () => {
    expect(
      capabilityMatchesDeclaration(
        detected("fs", "read", "/etc/passwd"),
        declared("fs", "read", "/data/*"),
      ),
    ).toBe(false);
  });
});

// ============================================================================
// Capability Comparison
// ============================================================================

describe("compareCapabilities", () => {
  it("all capabilities matched", () => {
    const result = compareCapabilities(
      [
        detected("fs", "read", "/etc/hosts"),
        detected("net", "connect", "*"),
      ],
      manifest([
        declared("fs", "read", "*"),
        declared("net", "connect", "*"),
      ]),
    );
    expect(result.matched).toHaveLength(2);
    expect(result.undeclared).toHaveLength(0);
    expect(result.unused).toHaveLength(0);
  });

  it("detects undeclared capabilities", () => {
    const result = compareCapabilities(
      [
        detected("fs", "read", "/etc/hosts"),
        detected("proc", "exec", "*"),
      ],
      manifest([
        declared("fs", "read", "*"),
      ]),
    );
    expect(result.matched).toHaveLength(1);
    expect(result.undeclared).toHaveLength(1);
    expect(result.undeclared[0].category).toBe("proc");
    expect(result.unused).toHaveLength(0);
  });

  it("detects unused declarations", () => {
    const result = compareCapabilities(
      [detected("fs", "read", "/etc/hosts")],
      manifest([
        declared("fs", "read", "*"),
        declared("net", "connect", "*"),
      ]),
    );
    expect(result.matched).toHaveLength(1);
    expect(result.undeclared).toHaveLength(0);
    expect(result.unused).toHaveLength(1);
    expect(result.unused[0].category).toBe("net");
  });

  it("handles empty detected list", () => {
    const result = compareCapabilities(
      [],
      manifest([declared("fs", "read", "*")]),
    );
    expect(result.matched).toHaveLength(0);
    expect(result.undeclared).toHaveLength(0);
    expect(result.unused).toHaveLength(1);
  });

  it("handles empty manifest", () => {
    const result = compareCapabilities(
      [detected("fs", "read", "/etc/hosts")],
      manifest([]),
    );
    expect(result.matched).toHaveLength(0);
    expect(result.undeclared).toHaveLength(1);
    expect(result.unused).toHaveLength(0);
  });

  it("handles both empty", () => {
    const result = compareCapabilities([], manifest([]));
    expect(result.matched).toHaveLength(0);
    expect(result.undeclared).toHaveLength(0);
    expect(result.unused).toHaveLength(0);
  });

  it("one declaration covers multiple detected capabilities", () => {
    const result = compareCapabilities(
      [
        detected("fs", "read", "/data/a.csv"),
        detected("fs", "read", "/data/b.csv"),
        detected("fs", "read", "/data/c.csv"),
      ],
      manifest([declared("fs", "read", "/data/*")]),
    );
    expect(result.matched).toHaveLength(3);
    expect(result.undeclared).toHaveLength(0);
    expect(result.unused).toHaveLength(0);
  });

  it("partial matching — some matched, some not", () => {
    const result = compareCapabilities(
      [
        detected("fs", "read", "/data/a.csv"),
        detected("fs", "write", "/data/a.csv"),
        detected("net", "connect", "example.com"),
      ],
      manifest([
        declared("fs", "read", "/data/*"),
        declared("net", "connect", "*"),
      ]),
    );
    expect(result.matched).toHaveLength(2);
    expect(result.undeclared).toHaveLength(1);
    expect(result.undeclared[0].action).toBe("write");
    expect(result.unused).toHaveLength(0);
  });
});
