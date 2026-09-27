import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { InvalidItfInputError, encodeItf, normalizeItf } from "../src/index.js";

interface FixtureCase {
  id: string;
  symbology: string;
  input: { text?: string; repeat?: { text: string; count: number } };
  expected: Record<string, unknown>;
}

const casesUrl = new URL(
  "../../../../specs/fixtures/barcode-symbologies-v1/cases.json",
  import.meta.url,
);
const cases = (
  JSON.parse(readFileSync(casesUrl, "utf8")) as { cases: FixtureCase[] }
).cases.filter((testCase) => testCase.symbology === "itf");

function materializeInput(testCase: FixtureCase): string {
  if (testCase.input.text !== undefined) return testCase.input.text;
  const repeat = testCase.input.repeat;
  if (!repeat) throw new Error(`missing input for ${testCase.id}`);
  return repeat.text.repeat(repeat.count);
}

function modules(data: string): string {
  return `1010${encodeItf(data).map((pair) => pair.binaryPattern).join("")}11101`;
}

function runLengths(bits: string): number[] {
  const runs: number[] = [];
  let previous: string | undefined;
  for (const bit of bits) {
    if (bit !== previous) {
      runs.push(1);
      previous = bit;
    }
    else runs[runs.length - 1] += 1;
  }
  return runs;
}

function sha256(value: string): string {
  return createHash("sha256").update(value, "utf8").digest("hex");
}

describe("barcode-symbologies-v1 ITF corpus", () => {
  for (const testCase of cases) {
    it(testCase.id, () => {
      const data = materializeInput(testCase);
      const expected = testCase.expected;
      if (typeof expected.error === "string") {
        try {
          normalizeItf(data);
          throw new Error("expected ITF validation failure");
        } catch (error) {
          expect(error).toBeInstanceOf(InvalidItfInputError);
          expect((error as InvalidItfInputError).errorId).toBe(expected.error);
        }
        return;
      }

      const normalized = normalizeItf(data);
      const encodedModules = modules(data);
      const runs = runLengths(encodedModules);
      if (typeof expected.normalized === "string") {
        expect(normalized).toBe(expected.normalized);
        expect(encodedModules).toBe(expected.modules);
        expect(runs).toEqual(expected.run_lengths);
      } else {
        expect(sha256(normalized)).toBe(expected.normalized_sha256);
        expect(encodedModules).toHaveLength(expected.module_count as number);
        expect(sha256(encodedModules)).toBe(expected.module_sha256);
        expect(runs).toHaveLength(expected.run_count as number);
        expect(sha256(JSON.stringify(runs))).toBe(expected.run_lengths_sha256);
      }
    });
  }
});
