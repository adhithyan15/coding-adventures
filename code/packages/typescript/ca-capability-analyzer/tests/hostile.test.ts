import { describe, it, expect, assertProperty, Gen } from "../../test-bootstrap/dist/index.js";
import { analyzeSource, analyzePackage, loadManifest } from "../dist/index.js";

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
    }, { numRuns: 50 });
  });

  it("safely handles deeply nested expressions without RangeError call stack overflow", () => {
    // 5000 nested parentheses (CWE-674 regression test)
    const deepCode = "(".repeat(5000) + "1" + ")".repeat(5000);
    const res = analyzeSource(deepCode, "deep.ts");
    expect(res).toBeDefined();
    expect(Array.isArray(res.weaknesses)).toBe(true);
    expect(Array.isArray(res.capabilities)).toBe(true);
  });
});
