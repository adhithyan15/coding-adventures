import { describe, it } from "node:test";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { expect } from "../dist/matchers.js";
import { parseFile } from "../dist/tfv.js";
import { valuesEqual } from "../dist/deep_equal.js";
import { SplitMix64 } from "../dist/prng.js";
import type { Record, Value } from "../dist/types.js";
import {
  FID_COUNTEREXAMPLE,
  FID_EXHAUSTED,
  FID_TIMEOUT,
  FID_PANIC,
  isRegisteredFailureId,
  genPrimitive,
  genBounded,
  genBytes,
  shrinkInt,
  shrinkList,
  shrinkBytes,
  simulateReplay,
  checkProperty,
  assertProperty,
  Gen,
  type PbtOptions,
} from "../dist/pbt.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function getField(record: Record, fieldName: string): Value {
  const val = record.fields.get(fieldName);
  if (!val) {
    throw new Error(`missing field ${fieldName} in record ${record.id}`);
  }
  return val;
}

function getText(record: Record, fieldName: string): string {
  const f = getField(record, fieldName);
  if (f.kind !== "text") {
    throw new Error(`field ${fieldName} must be text`);
  }
  return f.value;
}

function getInt(record: Record, fieldName: string): bigint {
  const f = getField(record, fieldName);
  if (f.kind !== "int") {
    throw new Error(`field ${fieldName} must be int`);
  }
  return BigInt(f.text);
}

function evalIntProperty(name: string, x: bigint): boolean {
  switch (name) {
    case "int_le_100":
      return x <= 100n;
    case "int_in_range_neg10_50":
      return x >= -10n && x <= 50n;
    case "refinement_x_le_50":
      return x <= 50n;
    default:
      throw new Error(`unknown int property: ${name}`);
  }
}

function evalListProperty(name: string, lst: bigint[]): boolean {
  switch (name) {
    case "list_no_consecutive_duplicates":
      for (let i = 0; i < lst.length - 1; i++) {
        if (lst[i] === lst[i + 1]) return false;
      }
      return true;
    case "list_sum_le_20": {
      const sum = lst.reduce((acc, v) => acc + v, 0n);
      return sum <= 20n;
    }
    default:
      throw new Error(`unknown list property: ${name}`);
  }
}

function evalBytesProperty(name: string, b: Uint8Array): boolean {
  switch (name) {
    case "bytes_len_lt_5":
      return b.length < 5;
    default:
      throw new Error(`unknown bytes property: ${name}`);
  }
}

describe("PBT Engine Conformance Suite (VER05 / pbt-v1)", () => {
  const corpusPath = path.resolve(
    __dirname,
    "../../../../specs/fixtures/pbt-v1/cases.tfv",
  );
  const corpusContent = fs.readFileSync(corpusPath, "utf-8");
  const records = parseFile(corpusContent);

  it("loads exactly 25 conformance cases from pbt-v1/cases.tfv", () => {
    expect(records.length).toBe(25);
  });

  for (const record of records) {
    it(`conformance case: ${record.id}`, () => {
      const op = getText(record, "op");
      switch (op) {
        case "gen_primitive": {
          const seed = getInt(record, "seed");
          const typeName = getText(record, "type");
          const count = Number(getInt(record, "count"));
          const got = genPrimitive(seed, typeName, count);
          const want = getField(record, "outputs");
          expect(valuesEqual({ kind: "list", items: got }, want)).toBe(true);
          break;
        }

        case "gen_bounded": {
          const seed = getInt(record, "seed");
          const minVal = getInt(record, "min");
          const maxVal = getInt(record, "max");
          const count = Number(getInt(record, "count"));
          const got = genBounded(seed, minVal, maxVal, count);
          const want = getField(record, "outputs");
          expect(valuesEqual({ kind: "list", items: got }, want)).toBe(true);
          break;
        }

        case "gen_bytes": {
          const seed = getInt(record, "seed");
          const minLen = Number(getInt(record, "min_len"));
          const maxLen = Number(getInt(record, "max_len"));
          const count = Number(getInt(record, "count"));
          const got = genBytes(seed, minLen, maxLen, count);
          const want = getField(record, "outputs");
          expect(valuesEqual({ kind: "list", items: got }, want)).toBe(true);
          break;
        }

        case "shrink_int":
        case "shrink_refinement": {
          const initial = getInt(record, "initial");
          const target = getInt(record, "target");
          const minVal = getInt(record, "min");
          const maxVal = getInt(record, "max");
          const propName = getText(record, "property");
          const expectedMinimal = getInt(record, "expected_minimal");
          const expectedSteps = Number(getInt(record, "expected_steps"));

          const [gotMinimal, gotSteps] = shrinkInt(
            initial,
            target,
            minVal,
            maxVal,
            (x) => evalIntProperty(propName, x),
            1000,
          );

          expect(gotMinimal).toBe(expectedMinimal);
          expect(gotSteps).toBe(expectedSteps);
          break;
        }

        case "shrink_list": {
          const initialField = getField(record, "initial");
          if (initialField.kind !== "list") throw new Error("initial must be list");
          const initial = initialField.items.map((it) => {
            if (it.kind !== "int") throw new Error("items must be int");
            return BigInt(it.text);
          });
          const elemTarget = getInt(record, "elem_target");
          const propName = getText(record, "property");
          const expectedMinimalField = getField(record, "expected_minimal");
          if (expectedMinimalField.kind !== "list") throw new Error("expected must be list");
          const expectedMinimal = expectedMinimalField.items.map((it) => {
            if (it.kind !== "int") throw new Error("items must be int");
            return BigInt(it.text);
          });
          const expectedSteps = Number(getInt(record, "expected_steps"));

          const [gotMinimal, gotSteps] = shrinkList(
            initial,
            elemTarget,
            (slice) => evalListProperty(propName, slice),
            1000,
          );

          expect(gotMinimal.length).toBe(expectedMinimal.length);
          for (let i = 0; i < gotMinimal.length; i++) {
            expect(gotMinimal[i]).toBe(expectedMinimal[i]);
          }
          expect(gotSteps).toBe(expectedSteps);
          break;
        }

        case "shrink_bytes": {
          const initialField = getField(record, "initial");
          if (initialField.kind !== "bytes") throw new Error("initial must be bytes");
          const propName = getText(record, "property");
          const expectedMinimalField = getField(record, "expected_minimal");
          if (expectedMinimalField.kind !== "bytes") throw new Error("expected must be bytes");
          const expectedSteps = Number(getInt(record, "expected_steps"));

          const [gotMinimal, gotSteps] = shrinkBytes(
            initialField.value,
            (b) => evalBytesProperty(propName, b),
            1000,
          );

          expect(valuesEqual({ kind: "bytes", value: gotMinimal }, expectedMinimalField)).toBe(true);
          expect(gotSteps).toBe(expectedSteps);
          break;
        }

        case "replay_consistency": {
          const seed = getInt(record, "seed");
          const propName = getText(record, "property");
          const expectedInit = getInt(record, "initial_fail");
          const expectedMinimal = getInt(record, "expected_minimal");

          const [gotInit, gotMin] = simulateReplay(seed, (x) =>
            evalIntProperty(propName, x),
          );

          expect(gotInit).toBe(expectedInit);
          expect(gotMin).toBe(expectedMinimal);
          break;
        }

        case "filter_exhausted": {
          const maxRetries = Number(getInt(record, "max_retries"));
          const exhaustedGen = Gen.integer(1, 10).filter(() => false, maxRetries);
          const opts: PbtOptions = {
            numTests: 10,
            maxRetries,
            maxShrinkSteps: 100,
            isCi: false,
          };
          const verdict = checkProperty(
            12345n,
            opts,
            (r) => exhaustedGen.sample(r),
            () => true,
          );

          const wantVerdict = getText(record, "verdict");
          const wantFid = getText(record, "failure_id");

          expect(verdict.status).toBe(wantVerdict);
          if (verdict.status === "fail") {
            expect(verdict.failureId).toBe(wantFid);
          }
          break;
        }

        case "failure_id_valid": {
          const candidate = getText(record, "input");
          const got = isRegisteredFailureId(candidate);
          const validField = getField(record, "valid");
          if (validField.kind !== "bool") throw new Error("valid must be bool");
          expect(got).toBe(validField.value);
          break;
        }

        case "corpus_reject": {
          const rawText = getText(record, "text");
          let rejected = false;
          try {
            parseFile(rawText);
          } catch {
            rejected = true;
          }
          expect(rejected).toBe(true);
          break;
        }

        default:
          throw new Error(`unrecognized op: ${op}`);
      }
    });
  }
});

describe("PBT High-Level Generator and Assertions API", () => {
  it("assertProperty passes for true invariant", () => {
    assertProperty(
      Gen.integer(1, 100),
      (n) => n >= 1n && n <= 100n,
      { numTests: 50, seed: 12345n },
    );
  });

  it("assertProperty throws FID_COUNTEREXAMPLE on failure with minimal witness", () => {
    let caught: any;
    try {
      assertProperty(
        Gen.integer(0, 500),
        (n) => n <= 50n,
        { numTests: 50, seed: 42n },
      );
    } catch (err) {
      caught = err;
    }
    expect(caught).toBeDefined();
    expect(caught.failureId).toBe(FID_COUNTEREXAMPLE);
  });

  it("Gen combinators: map, filter, list, oneOf", () => {
    const listGen = Gen.list(2, 5, Gen.integer(1, 10));
    assertProperty(
      listGen,
      (arr) => {
        expect(arr.length).toBeGreaterThanOrEqual(2);
        expect(arr.length).toBeLessThanOrEqual(5);
        for (const item of arr) {
          expect(item).toBeGreaterThanOrEqual(1n);
          expect(item).toBeLessThanOrEqual(10n);
        }
      },
      { numTests: 30, seed: 777n },
    );

    const mappedGen = Gen.bool().map((b) => (b ? "yes" : "no"));
    assertProperty(
      mappedGen,
      (s) => s === "yes" || s === "no",
      { numTests: 20, seed: 888n },
    );

    const oneOfGen = Gen.oneOf(Gen.integer(1, 5), Gen.integer(10, 15));
    assertProperty(
      oneOfGen,
      (n) => (n >= 1n && n <= 5n) || (n >= 10n && n <= 15n),
      { numTests: 30, seed: 999n },
    );
  });

  it("handles panic, CI mode digest, and generator errors", () => {
    // Panic in property
    const panicVerdict = checkProperty(
      1n,
      { numTests: 10 },
      () => 42,
      () => {
        throw new Error("unexpected explosion");
      },
    );
    expect(panicVerdict.status).toBe("fail");
    if (panicVerdict.status === "fail") {
      expect(panicVerdict.failureId).toBe(FID_PANIC);
      expect(panicVerdict.message).toContain("unexpected explosion");
    }

    // CI mode digest emission
    const ciVerdict = checkProperty(
      42n,
      { numTests: 10, isCi: true },
      () => 100,
      () => false,
      (val) => [val, 5],
    );
    expect(ciVerdict.status).toBe("fail");
    if (ciVerdict.status === "fail") {
      expect(ciVerdict.failureId).toBe(FID_COUNTEREXAMPLE);
      expect(ciVerdict.message).toContain("repro-fnv:");
    }

    // Generator error
    const genErrorVerdict = checkProperty(
      1n,
      { numTests: 10 },
      () => {
        throw new Error("sampling failure");
      },
      () => true,
    );
    expect(genErrorVerdict.status).toBe("fail");
    if (genErrorVerdict.status === "fail") {
      expect(genErrorVerdict.failureId).toBe(FID_EXHAUSTED);
      expect(genErrorVerdict.message).toContain("sampling failure");
    }
  });

  it("validates generator boundary checks and error conditions", () => {
    expect(() => Gen.oneOf()).toThrow();
    expect(() => Gen.list(5, 2, Gen.bool())).toThrow();
    expect(() => genBytes(1n, 5, 2, 1)).toThrow();
    expect(() => genPrimitive(1n, "unknown_type", 1)).toThrow();

    // shrinkInt and shrinkList throw if initial does not falsify property
    expect(() =>
      shrinkInt(10n, 0n, 0n, 100n, () => true),
    ).toThrow();
    expect(() =>
      shrinkList([1n, 2n], 0n, () => true),
    ).toThrow();

    // isPbtFailureId checks
    expect(isRegisteredFailureId(FID_COUNTEREXAMPLE)).toBe(true);
    expect(isRegisteredFailureId(FID_EXHAUSTED)).toBe(true);
    expect(isRegisteredFailureId(FID_TIMEOUT)).toBe(true);
    expect(isRegisteredFailureId(FID_PANIC)).toBe(true);
    expect(isRegisteredFailureId("random-failure-id")).toBe(false);

    // Large span bounded integers
    const rng = new SplitMix64(42n);
    const largeSpanVal = Gen.integer(0n, 1n << 64n).sample(rng);
    expect(largeSpanVal).toBeGreaterThanOrEqual(0n);

    const hugeSpanVal = Gen.integer(0n, 1n << 70n).sample(rng);
    expect(hugeSpanVal).toBeGreaterThanOrEqual(0n);

    expect(() => Gen.integer(100n, 10n).sample(rng)).toThrow();
  });
});
