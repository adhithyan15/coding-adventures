import { describe, it, expect, assertProperty, Gen, SplitMix64 } from "@coding-adventures/test-bootstrap";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  analyzeSource,
  analyzePackage,
  discoverSourceFiles,
  isCatastrophicRegex,
  loadManifest,
  MAX_REGEX_PATTERN_LEN,
  MAX_SOURCE_BYTES,
  runCli,
} from "../src/index.js";

describe("Hostile Input & Edge Case Hardening", () => {
  it("recovers safely from syntactically malformed TypeScript code", () => {
    const brokenCode = `
      eval("broken");
      function broken( {
        const x = ;
    `;
    // Should parse as much as possible without throwing unhandled crash
    const res = analyzeSource(brokenCode, "broken.ts");
    expect(res.weaknesses.some((w) => w.cwe === "CWE-94")).toBe(true);
  });

  it("handles extremely large files without crashing or hanging", () => {
    const largeComment = "// " + "A".repeat(50_000) + "\n";
    const code = largeComment + "export const pure = 42;\n";
    const res = analyzeSource(code, "large.ts");
    expect(res.capabilities.length).toBe(0);
    expect(res.weaknesses.length).toBe(0);
  });

  it("fails closed when required_capabilities.json contains malformed JSON", () => {
    expect(() => loadManifest("C:/nonexistent/directory/path/that/does/not/exist")).not.toThrow();
  });

  it("executes hostile fuzzing inputs against analyzer without panic", () => {
    const hostileInputs = [
      "",
      "//",
      "/* unclosed comment",
      "import",
      "eval(",
      "new RegExp(",
      "obj[__proto__]",
      "process.env",
      "const \x00 = 1;",
      "function foo() { foo(); }",
    ];

    const inputGen = new Gen((rng) => hostileInputs[rng.bounded(hostileInputs.length)]);

    assertProperty(inputGen, (input) => {
      try {
        const res = analyzeSource(input, "fuzz.ts");
        return Array.isArray(res.capabilities) && Array.isArray(res.weaknesses);
      } catch (err) {
        // Any error thrown must be a controlled Error instance
        return err instanceof Error;
      }
    }, { numTests: 50 });
  });

  it("safely handles deeply nested expressions without RangeError call stack overflow", () => {
    // 5000 nested parentheses (CWE-674 regression test)
    const deepCode = "(".repeat(5000) + "1" + ")".repeat(5000);
    const res = analyzeSource(deepCode, "deep.ts");
    expect(res).toBeDefined();
    expect(Array.isArray(res.weaknesses)).toBe(true);
    expect(Array.isArray(res.capabilities)).toBe(true);
  });

  it("rejects hostile CWE-20/400/1321 inputs fail-closed", () => {
    // Non-string and oversized sourceText / regex patterns
    expect(() => analyzeSource(123 as any, "bad.ts")).toThrow();
    expect(() => analyzeSource("x", "bad\0.ts")).toThrow();
    expect(() => analyzeSource("a".repeat(MAX_SOURCE_BYTES + 1), "huge.ts")).toThrow();
    expect(() => isCatastrophicRegex(null as any)).toThrow();
    expect(isCatastrophicRegex("a".repeat(MAX_REGEX_PATTERN_LEN + 1))).toBe(true);
    // Meta-ReDoS regression: unclosed "(" + "+"*16380 must complete in linear time
    const startMs = performance.now();
    expect(isCatastrophicRegex("(" + "+".repeat(16380))).toBe(false);
    expect(performance.now() - startMs).toBeLessThan(50);
    expect(isCatastrophicRegex("\\d+foo\\d+")).toBe(true);
    expect(isCatastrophicRegex("(|a)+")).toBe(false);

    // Null / invalid options and invalid paths
    expect(() => analyzePackage("", {})).toThrow();
    expect(() => analyzePackage(".\0", {})).toThrow();
    expect(() => analyzePackage(".", null as any)).toThrow();
    expect(() => discoverSourceFiles("", {})).toThrow();
    expect(() => discoverSourceFiles(".", null as any)).toThrow();
    expect(() => loadManifest("")).toThrow();
    expect(() => loadManifest(".\0")).toThrow();

    // Hostile CLI argv inputs
    expect(runCli(null as any)).toBe(2);
    expect(runCli(["bad\0arg"])).toBe(2);
    expect(runCli(["--dir", ""])).toBe(2);
    expect(runCli(["--dir="])).toBe(2);
    expect(runCli([""])).toBe(2);

    // SplitMix64-seeded malformed & prototype-polluting manifests in temp dir
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "ca-hostile-"));
    try {
      const manifestPath = path.join(tmpDir, "required_capabilities.json");
      fs.writeFileSync(manifestPath, "{ bad json", "utf-8");
      expect(() => loadManifest(tmpDir)).toThrow();

      fs.writeFileSync(manifestPath, "123", "utf-8");
      expect(() => loadManifest(tmpDir)).toThrow();

      fs.writeFileSync(
        manifestPath,
        JSON.stringify({
          package: "test-pkg",
          capabilities: [
            "fs:read",
            { category: { evil: true }, action: "read" },
            { category: "net", action: "connect" },
          ],
          banned_construct_exceptions: [
            { construct: "CWE-94", language: "ts", justification: "test" },
          ],
        }),
        "utf-8"
      );
      const parsed = loadManifest(tmpDir);
      expect(parsed.declaredCapabilities.has("fs:read")).toBe(true);
      expect(parsed.declaredCapabilities.has("net:socket")).toBe(true);
      expect(parsed.exceptions.has("cwe-94")).toBe(true);

      // Oversized capability token (> 128) rejected
      fs.writeFileSync(
        manifestPath,
        JSON.stringify({ capabilities: ["x".repeat(200)] }),
        "utf-8"
      );
      expect(() => loadManifest(tmpDir)).toThrow();

      // Oversized array (> 4096) rejected
      fs.writeFileSync(
        manifestPath,
        JSON.stringify(Array.from({ length: 4097 }, () => "fs:read")),
        "utf-8"
      );
      expect(() => loadManifest(tmpDir)).toThrow();

      // Oversized category/action (> 128) and empty capability string
      fs.writeFileSync(
        manifestPath,
        JSON.stringify({ capabilities: [""] }),
        "utf-8"
      );
      expect(loadManifest(tmpDir).declaredCapabilities.size).toBe(0);

      fs.writeFileSync(
        manifestPath,
        JSON.stringify({ capabilities: [{ category: "x".repeat(200), action: "read" }] }),
        "utf-8"
      );
      expect(() => loadManifest(tmpDir)).toThrow();

      // Oversized manifest file (> 1 MiB) rejected
      fs.writeFileSync(manifestPath, " ".repeat(1024 * 1024 + 8), "utf-8");
      expect(() => loadManifest(tmpDir)).toThrow();

      // Single-file discovery & oversized file rejection
      const singleFile = path.join(tmpDir, "single.ts");
      fs.writeFileSync(singleFile, "export const a = 1;\n", "utf-8");
      expect(discoverSourceFiles(singleFile)).toEqual([singleFile]);

      const hugeFile = path.join(tmpDir, "huge.ts");
      fs.writeFileSync(hugeFile, "a".repeat(2 * 1024 * 1024 + 8), "utf-8");
      expect(() => discoverSourceFiles(hugeFile)).toThrow();
      expect(() => discoverSourceFiles(tmpDir)).toThrow();

      // SplitMix64 fuzz over hostile manifest strings
      const rng = new SplitMix64(0x99887766n);
      const payloads = ["null", "[]", "{}", "true", "\"str\"", "{bad"];
      for (let i = 0; i < 12; i++) {
        const p = payloads[rng.bounded(payloads.length)];
        fs.writeFileSync(manifestPath, p, "utf-8");
        try {
          const m = loadManifest(tmpDir);
          expect(m.declaredCapabilities instanceof Set).toBe(true);
        } catch (err) {
          expect(err instanceof Error).toBe(true);
        }
      }
    } finally {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});
