import { describe, it, expect, assertProperty, Gen } from "../../test-bootstrap/dist/index.js";
import { analyzeSource } from "../dist/index.js";

describe("PBT Invariant Verification (VER05a)", () => {
  it("proves pure arithmetic expressions produce zero capabilities and zero CWEs", () => {
    const numGen = Gen.integer(-1000, 1000);
    const opGen = new Gen((rng) => ["+", "-", "*", "%"][rng.bounded(4)]);

    const exprGen = new Gen((rng) => {
      const a = numGen.sample(rng);
      const b = numGen.sample(rng);
      const op = opGen.sample(rng);
      return `export function calculate() { return ${a} ${op} ${b}; }`;
    });

    assertProperty(exprGen, (code) => {
      const res = analyzeSource(code, "pure.ts");
      expect(res.capabilities.length).toBe(0);
      expect(res.weaknesses.length).toBe(0);
      return true;
    }, { numRuns: 50 });
  });

  it("proves injecting Math.random() deterministically detects random:entropy", () => {
    const varNameGen = new Gen((rng) => {
      const letters = ["a", "b", "c", "x", "val", "seed"];
      return letters[rng.bounded(letters.length)] + rng.bounded(100);
    });

    assertProperty(varNameGen, (vName) => {
      const code = `
        export function getRandom() {
          const ${vName} = Math.random();
          return ${vName};
        }
      `;
      const res = analyzeSource(code, "random.ts");
      const caps = res.capabilities.map((c) => c.capability);
      expect(caps).toContain("random:entropy");
      return true;
    }, { numRuns: 30 });
  });

  it("proves injecting eval() deterministically detects CWE-94", () => {
    const stringLiteralGen = new Gen((rng) => `"${rng.bounded(1000)}"`);

    assertProperty(stringLiteralGen, (str) => {
      const code = `
        export function run() {
          eval(${str});
        }
      `;
      const res = analyzeSource(code, "eval.ts");
      const cwes = res.weaknesses.map((w) => w.cwe);
      expect(cwes).toContain("CWE-94");
      return true;
    }, { numRuns: 30 });
  });

  it("proves analysis is completely deterministic across repeated invocations", () => {
    const code = `
      import fs from "node:fs";
      export function read(path: string) {
        return fs.readFileSync(path, "utf-8");
      }
    `;

    const r1 = analyzeSource(code, "test.ts");
    const r2 = analyzeSource(code, "test.ts");

    expect(r1.capabilities.length).toBe(r2.capabilities.length);
    expect(r1.weaknesses.length).toBe(r2.weaknesses.length);
    expect(r1.capabilities[0].capability).toBe(r2.capabilities[0].capability);
    expect(r1.capabilities[0].line).toBe(r2.capabilities[0].line);
  });
});
