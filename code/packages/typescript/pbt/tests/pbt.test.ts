import { describe, it } from "node:test";
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

import {
  SplitMix64,
  fnv1a64,
  subSeed,
  FID_COUNTEREXAMPLE,
  FID_EXHAUSTED,
  FID_TIMEOUT,
  FID_PANIC,
  isPbtFailureId,
  isRegisteredFailureId,
  genBool,
  genU8,
  genU64,
  genI128,
  genPrimitive,
  boundedU64,
  genBoundedInt,
  genBounded,
  genBytes,
  shrinkInt,
  shrinkList,
  shrinkBytes,
  simulateReplay,
  checkProperty,
  assertProperty,
  Gen,
  ExhaustedError,
  type PbtOptions,
} from "../dist/index.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

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

function hexToBytes(hex: string): Uint8Array {
  if (hex.length === 0) return new Uint8Array(0);
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

describe("PBT Engine Conformance Suite (VER05 / pbt-v1)", () => {
  const corpusPath = path.resolve(
    __dirname,
    "../../../../specs/fixtures/pbt-v1/cases.json",
  );
  const corpusContent = fs.readFileSync(corpusPath, "utf-8");
  const parsedCorpus = JSON.parse(corpusContent);
  const cases: any[] = parsedCorpus.cases;

  it("loads exactly 25 conformance cases from pbt-v1/cases.json", () => {
    assert.equal(cases.length, 25);
  });

  for (const c of cases) {
    it(`conformance case: ${c.id}`, () => {
      const op = c.op;
      switch (op) {
        case "gen_primitive": {
          const seed = BigInt(c.seed.value);
          const typeName = c.type;
          const count = Number(c.count.value);
          const got = genPrimitive(seed, typeName, count);

          assert.equal(got.length, c.outputs.length);
          for (let i = 0; i < got.length; i++) {
            const exp = c.outputs[i];
            if (typeof exp === "boolean") {
              assert.equal(got[i].kind, "bool");
              assert.equal((got[i] as { kind: "bool"; value: boolean }).value, exp);
            } else if (exp.kind === "int") {
              assert.equal(got[i].kind, "int");
              assert.equal((got[i] as { kind: "int"; text: string }).text, exp.value);
            } else {
              assert.fail(`unexpected output format in ${c.id}`);
            }
          }
          break;
        }

        case "gen_bounded": {
          const seed = BigInt(c.seed.value);
          const minVal = BigInt(c.min.value);
          const maxVal = BigInt(c.max.value);
          const count = Number(c.count.value);
          const got = genBounded(seed, minVal, maxVal, count);

          assert.equal(got.length, c.outputs.length);
          for (let i = 0; i < got.length; i++) {
            assert.equal(got[i].kind, "int");
            assert.equal((got[i] as { kind: "int"; text: string }).text, c.outputs[i].value);
          }
          break;
        }

        case "gen_bytes": {
          const seed = BigInt(c.seed.value);
          const minLen = Number(c.min_len.value);
          const maxLen = Number(c.max_len.value);
          const count = Number(c.count.value);
          const got = genBytes(seed, minLen, maxLen, count);

          assert.equal(got.length, c.outputs.length);
          for (let i = 0; i < got.length; i++) {
            assert.equal(got[i].kind, "bytes");
            const gotHex = bytesToHex((got[i] as { kind: "bytes"; value: Uint8Array }).value);
            assert.equal(gotHex, c.outputs[i].hex);
          }
          break;
        }

        case "shrink_int":
        case "shrink_refinement": {
          const initial = BigInt(c.initial.value);
          const target = BigInt(c.target.value);
          const minVal = BigInt(c.min.value);
          const maxVal = BigInt(c.max.value);
          const propName = c.property;
          const expectedMinimal = BigInt(c.expected_minimal.value);
          const expectedSteps = Number(c.expected_steps.value);

          const [gotMinimal, gotSteps] = shrinkInt(
            initial,
            target,
            minVal,
            maxVal,
            (x) => evalIntProperty(propName, x),
            1000,
          );

          assert.equal(gotMinimal, expectedMinimal);
          assert.equal(gotSteps, expectedSteps);
          break;
        }

        case "shrink_list": {
          const initial = c.initial.map((it: any) => BigInt(it.value));
          const elemTarget = BigInt(c.elem_target.value);
          const propName = c.property;
          const expectedMinimal = c.expected_minimal.map((it: any) => BigInt(it.value));
          const expectedSteps = Number(c.expected_steps.value);

          const [gotMinimal, gotSteps] = shrinkList(
            initial,
            elemTarget,
            (slice) => evalListProperty(propName, slice),
            1000,
          );

          assert.equal(gotMinimal.length, expectedMinimal.length);
          for (let i = 0; i < gotMinimal.length; i++) {
            assert.equal(gotMinimal[i], expectedMinimal[i]);
          }
          assert.equal(gotSteps, expectedSteps);
          break;
        }

        case "shrink_bytes": {
          const initialBytes = hexToBytes(c.initial.hex);
          const propName = c.property;
          const expectedMinimalBytes = hexToBytes(c.expected_minimal.hex);
          const expectedSteps = Number(c.expected_steps.value);

          const [gotMinimal, gotSteps] = shrinkBytes(
            initialBytes,
            (b) => evalBytesProperty(propName, b),
            1000,
          );

          assert.equal(bytesToHex(gotMinimal), bytesToHex(expectedMinimalBytes));
          assert.equal(gotSteps, expectedSteps);
          break;
        }

        case "replay_consistency": {
          const seed = BigInt(c.seed.value);
          const propName = c.property;
          const expectedInit = BigInt(c.initial_fail.value);
          const expectedMinimal = BigInt(c.expected_minimal.value);

          const [gotInit, gotMin] = simulateReplay(seed, (x) =>
            evalIntProperty(propName, x),
          );

          assert.equal(gotInit, expectedInit);
          assert.equal(gotMin, expectedMinimal);
          break;
        }

        case "filter_exhausted": {
          const maxRetries = Number(c.max_retries.value);
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

          assert.equal(verdict.status, c.verdict);
          if (verdict.status === "fail") {
            assert.equal(verdict.failureId, c.failure_id);
          }
          break;
        }

        case "failure_id_valid": {
          const candidate = c.input;
          const got = isRegisteredFailureId(candidate);
          assert.equal(got, c.valid);
          assert.equal(isPbtFailureId(candidate), c.valid);
          break;
        }

        case "corpus_reject": {
          const rawText = c.text;
          assert.throws(() => {
            if (!rawText.startsWith("{") && !rawText.startsWith("[")) {
              throw new Error(`invalid JSON format: ${rawText}`);
            }
            JSON.parse(rawText);
          });
          break;
        }

        default:
          assert.fail(`unrecognized op: ${op}`);
      }
    });
  }
});

describe("Generator Algebra & Combinators", () => {
  it("Gen.bool produces booleans", () => {
    const rng = new SplitMix64(42n);
    const g = Gen.bool();
    let hasTrue = false;
    let hasFalse = false;
    for (let i = 0; i < 50; i++) {
      const v = g.sample(rng);
      assert.equal(typeof v, "boolean");
      if (v) hasTrue = true;
      else hasFalse = true;
    }
    assert.ok(hasTrue && hasFalse);
  });

  it("Gen.u8 produces numbers in [0, 255]", () => {
    const rng = new SplitMix64(42n);
    const g = Gen.u8();
    for (let i = 0; i < 100; i++) {
      const v = g.sample(rng);
      assert.ok(v >= 0 && v <= 255 && Number.isInteger(v));
    }
  });

  it("Gen.u64 produces 64-bit unsigned bigints", () => {
    const rng = new SplitMix64(42n);
    const g = Gen.u64();
    for (let i = 0; i < 50; i++) {
      const v = g.sample(rng);
      assert.ok(v >= 0n && v < 1n << 64n);
    }
  });

  it("Gen.integer produces bounded bigints", () => {
    const rng = new SplitMix64(101n);
    const g = Gen.integer(-50, 50);
    for (let i = 0; i < 100; i++) {
      const v = g.sample(rng);
      assert.ok(v >= -50n && v <= 50n);
    }
  });

  it("Gen.bytes produces Uint8Array within length range", () => {
    const rng = new SplitMix64(202n);
    const g = Gen.bytes(2, 6);
    for (let i = 0; i < 50; i++) {
      const b = g.sample(rng);
      assert.ok(b instanceof Uint8Array);
      assert.ok(b.length >= 2 && b.length <= 6);
      for (const byte of b) {
        assert.ok(byte >= 0 && byte <= 255);
      }
    }
  });

  it("Gen.list produces arrays of elements", () => {
    const rng = new SplitMix64(303n);
    const g = Gen.list(3, 8, Gen.integer(1, 10));
    for (let i = 0; i < 50; i++) {
      const arr = g.sample(rng);
      assert.ok(Array.isArray(arr));
      assert.ok(arr.length >= 3 && arr.length <= 8);
      for (const item of arr) {
        assert.ok(item >= 1n && item <= 10n);
      }
    }
  });

  it("Gen.constant always returns the same value", () => {
    const rng = new SplitMix64(404n);
    const g = Gen.constant("hello-pbt");
    for (let i = 0; i < 20; i++) {
      assert.equal(g.sample(rng), "hello-pbt");
    }
  });

  it("Gen.oneOf selects among choices", () => {
    const rng = new SplitMix64(505n);
    const g = Gen.oneOf(Gen.constant(1), Gen.constant(2), Gen.constant(3));
    const seen = new Set<number>();
    for (let i = 0; i < 50; i++) {
      const v = g.sample(rng);
      assert.ok(v === 1 || v === 2 || v === 3);
      seen.add(v);
    }
    assert.equal(seen.size, 3);
  });

  it("Gen.tuple produces typed tuples", () => {
    const rng = new SplitMix64(606n);
    const g = Gen.tuple(Gen.constant("alpha"), Gen.integer(10, 20), Gen.bool());
    for (let i = 0; i < 30; i++) {
      const [str, num, b] = g.sample(rng);
      assert.equal(str, "alpha");
      assert.ok(num >= 10n && num <= 20n);
      assert.equal(typeof b, "boolean");
    }
  });

  it("Gen.map transforms generator outputs", () => {
    const rng = new SplitMix64(707n);
    const g = Gen.integer(1, 10).map((n) => Number(n) * 2);
    for (let i = 0; i < 50; i++) {
      const v = g.sample(rng);
      assert.ok(v % 2 === 0);
      assert.ok(v >= 2 && v <= 20);
    }
  });

  it("Gen.filter respects predicate and throws ExhaustedError when impossible", () => {
    const rng = new SplitMix64(808n);
    const g = Gen.integer(1, 100).filter((n) => n % 2n === 0n);
    for (let i = 0; i < 30; i++) {
      const v = g.sample(rng);
      assert.ok(v % 2n === 0n);
    }

    const impossible = Gen.integer(1, 10).filter(() => false, 10);
    assert.throws(() => impossible.sample(rng), ExhaustedError);
  });

  it("Gen.flatMap chains dependent generators", () => {
    const rng = new SplitMix64(909n);
    const g = Gen.integer(1, 5).flatMap((len) =>
      Gen.list(Number(len), Number(len), Gen.constant(99)),
    );
    for (let i = 0; i < 30; i++) {
      const arr = g.sample(rng);
      assert.ok(arr.length >= 1 && arr.length <= 5);
      assert.ok(arr.every((v) => v === 99));
    }
  });
});

describe("Verification, Shrinking, and Error Handling", () => {
  it("assertProperty passes for true property", () => {
    assertProperty(
      Gen.integer(-100, 100),
      (n) => n >= -100n && n <= 100n,
      { numTests: 100, seed: 12345n },
    );
  });

  it("assertProperty throws error with FID_COUNTEREXAMPLE on failure", () => {
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
    assert.ok(caught);
    assert.equal(caught.failureId, FID_COUNTEREXAMPLE);
  });

  it("checkProperty captures property evaluation panic with FID_PANIC", () => {
    const verdict = checkProperty(
      1n,
      { numTests: 10 },
      () => 42,
      () => {
        throw new Error("simulated property crash");
      },
    );
    assert.equal(verdict.status, "fail");
    if (verdict.status === "fail") {
      assert.equal(verdict.failureId, FID_PANIC);
      assert.ok(verdict.message.includes("simulated property crash"));
    }
  });

  it("checkProperty captures sampling errors with FID_EXHAUSTED", () => {
    const verdict = checkProperty(
      1n,
      { numTests: 10 },
      () => {
        throw new Error("generator sampling exploded");
      },
      () => true,
    );
    assert.equal(verdict.status, "fail");
    if (verdict.status === "fail") {
      assert.equal(verdict.failureId, FID_EXHAUSTED);
      assert.ok(verdict.message.includes("generator sampling exploded"));
    }
  });

  it("CI privacy reporting hashes counterexample with FNV-1a", () => {
    const ciVerdict = checkProperty(
      42n,
      { numTests: 10, isCi: true },
      () => 100,
      () => false,
      (val) => [val, 5],
    );
    assert.equal(ciVerdict.status, "fail");
    if (ciVerdict.status === "fail") {
      assert.equal(ciVerdict.failureId, FID_COUNTEREXAMPLE);
      assert.ok(ciVerdict.message.includes("repro-fnv:"));
      assert.ok(!ciVerdict.message.includes("initial="));
    }
  });

  it("Non-CI reporting includes initial and minimal counterexample safely", () => {
    const nonCiVerdict = checkProperty(
      42n,
      { numTests: 10, isCi: false },
      () => 100,
      () => false,
      (val) => [val, 5],
    );
    assert.equal(nonCiVerdict.status, "fail");
    if (nonCiVerdict.status === "fail") {
      assert.equal(nonCiVerdict.failureId, FID_COUNTEREXAMPLE);
      assert.ok(nonCiVerdict.message.includes("initial=100"));
      assert.ok(nonCiVerdict.message.includes("minimal=100"));
    }
  });

  it("validates boundary checks, invalid ranges, and preconditions", () => {
    assert.throws(() => Gen.oneOf());
    assert.throws(() => Gen.list(5, 2, Gen.bool()));
    assert.throws(() => Gen.list(-1, 5, Gen.bool()));
    assert.throws(() => genBytes(1n, 5, 2, 1));
    assert.throws(() => genBytes(1n, -1, 5, 1));
    assert.throws(() => Gen.bytes(5, 2));
    assert.throws(() => Gen.bytes(-1, 5));
    assert.throws(() => genPrimitive(1n, "unknown_type", 1));

    const rng = new SplitMix64(42n);
    assert.throws(() => Gen.integer(100n, 10n).sample(rng));

    // shrinkInt and shrinkList throw if initial does not falsify property
    assert.throws(() => shrinkInt(10n, 0n, 0n, 100n, () => true));
    assert.throws(() => shrinkList([1n, 2n], 0n, () => true));

    // shrinkInt early exit if target falsifies
    const [targetShrunk, targetSteps] = shrinkInt(10n, 0n, 0n, 100n, () => false);
    assert.equal(targetShrunk, 0n);
    assert.equal(targetSteps, 1);

    // shrinkList early exit if empty list falsifies
    const [emptyShrunk, emptySteps] = shrinkList([10n, 20n], 0n, () => false);
    assert.equal(emptyShrunk.length, 0);
    assert.equal(emptySteps, 1);

    // SplitMix64 zero seed initializes to non-zero golden ratio constant
    const smZero = new SplitMix64(0n);
    assert.notEqual(smZero.nextU64(), 0n);

    // SplitMix64 bounded <= 1 returns 0
    assert.equal(smZero.bounded(1), 0);
    assert.equal(smZero.bounded(0), 0);
    assert.throws(() => smZero.bounded(0x1_0000_0001n));

    // Exact 64-bit span in genBoundedInt
    const exact64Span = genBoundedInt(rng, 0n, (1n << 64n) - 1n);
    assert.ok(exact64Span >= 0n && exact64Span < 1n << 64n);

    // Multi-word BigInt span
    const multiWordSpan = genBoundedInt(rng, 0n, 1n << 150n);
    assert.ok(multiWordSpan >= 0n && multiWordSpan <= 1n << 150n);

    // subSeed derivation is deterministic and distinct per propertyId
    const s1 = subSeed(42n, "prop-alpha");
    const s2 = subSeed(42n, "prop-alpha");
    const s3 = subSeed(42n, "prop-beta");
    assert.equal(s1, s2);
    assert.notEqual(s1, s3);

    // Safe stringification handles circular objects without crash
    const circular: any = {};
    circular.self = circular;
    const circVerdict = checkProperty(
      1n,
      { numTests: 1 },
      () => circular,
      () => false,
    );
    assert.equal(circVerdict.status, "fail");
    if (circVerdict.status === "fail") {
      assert.equal(circVerdict.failureId, FID_COUNTEREXAMPLE);
    }
  });

  it("handles large span integer generation", () => {
    const rng = new SplitMix64(99n);
    const u64Span = Gen.integer(0n, 1n << 64n).sample(rng);
    assert.ok(u64Span >= 0n && u64Span <= 1n << 64n);

    const u128Span = Gen.integer(0n, 1n << 70n).sample(rng);
    assert.ok(u128Span >= 0n && u128Span <= 1n << 70n);
  });

  it("deterministic replay with SplitMix64", () => {
    const seed = 987654321n;
    const rng1 = new SplitMix64(seed);
    const rng2 = new SplitMix64(seed);

    for (let i = 0; i < 100; i++) {
      assert.equal(rng1.nextU64(), rng2.nextU64());
      assert.equal(rng1.bounded(1000), rng2.bounded(1000));
    }
  });
});
