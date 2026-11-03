/**
 * The shared rng corpus (spec rng.md, "Conformance corpus").
 *
 * code/specs/fixtures/rng-v1/cases.json pins exact outputs every language's
 * rng package must reproduce; its results come from the independent
 * reference code/scripts/rng_conformance.py. Integers in the corpus are
 * decimal strings, so they reach `bigint` without passing through a lossy
 * JavaScript `number`.
 */

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { LCG, PCG32, Xorshift64 } from "../src/index.js";

interface Call {
  op: string;
  min?: string;
  max?: string;
  result?: string;
  result_bits?: string;
  error?: string;
}
interface Case {
  id: string;
  generator: "lcg" | "xorshift64" | "pcg32";
  seed: string;
  calls: Call[];
}

const corpus: { cases: Case[] } = JSON.parse(
  readFileSync(new URL("../../../../specs/fixtures/rng-v1/cases.json", import.meta.url), "utf8"),
);

const make = (name: Case["generator"], seed: bigint) =>
  name === "lcg" ? new LCG(seed) : name === "xorshift64" ? new Xorshift64(seed) : new PCG32(seed);

const floatBits = (x: number): string => {
  const view = new DataView(new ArrayBuffer(8));
  view.setFloat64(0, x);
  return view.getBigUint64(0).toString(16).padStart(16, "0");
};

describe("rng-v1 corpus", () => {
  it("has every case", () => {
    expect(corpus.cases.length).toBeGreaterThanOrEqual(27);
  });

  for (const c of corpus.cases) {
    it(c.id, () => {
      const g = make(c.generator, BigInt(c.seed));
      c.calls.forEach((call, index) => {
        const at = `${c.id} call ${index} (${call.op})`;
        switch (call.op) {
          case "next_u32":
            expect(String(g.nextU32()), at).toBe(call.result);
            break;
          case "next_u64":
            expect(String(g.nextU64()), at).toBe(call.result);
            break;
          case "next_float":
            expect(floatBits(g.nextFloat()), at).toBe(call.result_bits);
            break;
          case "next_int_in_range": {
            const min = BigInt(call.min!);
            const max = BigInt(call.max!);
            if (call.error) {
              expect(() => g.nextBigIntInRange(min, max), at).toThrow(RangeError);
            } else {
              expect(String(g.nextBigIntInRange(min, max)), at).toBe(call.result);
            }
            break;
          }
          default:
            throw new Error(`${at}: unknown op`);
        }
      });
    });
  }
});

describe("number bounds", () => {
  it("agree with the bigint form for safe integers", () => {
    const a = new PCG32(5n);
    const b = new PCG32(5n);
    for (let i = 0; i < 200; i++) {
      expect(BigInt(a.nextIntInRange(-1000, 2 ** 40))).toBe(b.nextBigIntInRange(-1000n, 2n ** 40n));
    }
  });

  it("reject unsafe or reversed bounds before drawing", () => {
    const g = new LCG(9n);
    const fresh = new LCG(9n);
    expect(() => g.nextIntInRange(0, 2 ** 60)).toThrow(RangeError);
    expect(() => g.nextIntInRange(0.5, 2)).toThrow(RangeError);
    expect(() => g.nextIntInRange(2, 1)).toThrow(RangeError);
    expect(() => g.nextBigIntInRange(0n, 1n << 63n)).toThrow(RangeError);
    expect(() => g.nextBigIntInRange(-(1n << 63n) - 1n, 0n)).toThrow(RangeError);
    expect(g.nextU32()).toBe(fresh.nextU32());
  });

  it("reach beyond 2^32 for wide ranges", () => {
    const g = new Xorshift64(3n);
    const values = Array.from({ length: 64 }, () => g.nextIntInRange(0, 2 ** 40));
    expect(values.some((v) => v >= 2 ** 32)).toBe(true);
  });
});
