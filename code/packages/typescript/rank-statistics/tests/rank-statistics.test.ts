import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { median, medianInterval } from "../src/index.js";
import { finiteBinary64Campaign, permuted, seededIntegerArray, XorShift32 } from "./campaigns.js";
import {
  bitsOf,
  exactMedian,
  exactMidpoint,
  expectedInterval,
  numberFromBits,
  pascalRankProfile,
  selectedRank,
  signEnumerationRankProfile,
  sortedCopy,
} from "./oracles.js";

interface MidpointVectorFile {
  readonly version: number;
  readonly vectors: readonly {
    readonly name: string;
    readonly left_bits: string;
    readonly right_bits: string;
    readonly expected_bits: string;
  }[];
}

interface MidpointVectorMetadata {
  readonly generator: string;
  readonly python: string;
  readonly arithmetic: string;
  readonly special_ordered_pair_count: number;
  readonly seed: string;
  readonly seeded_accepted_pairs: number;
  readonly seeded_rejected_nonfinite_bit_patterns: number;
  readonly vector_count: number;
  readonly fixture_sha256: string;
}

interface PairFixture {
  readonly version: number;
  readonly confidence_percent: number;
  readonly cases: readonly {
    readonly name: string;
    readonly baseline_ns: number;
    readonly candidate_ns: readonly number[];
    readonly ratio: number;
    readonly interval: readonly number[] | null;
  }[];
}

const here = new URL(".", import.meta.url);
const midpointFixturePath = fileURLToPath(new URL("./fixtures/midpoint-vectors-v1.json", here));
const midpointMetadataPath = fileURLToPath(new URL("./fixtures/midpoint-vectors-v1.meta.json", here));
const packagePairsPath = fileURLToPath(new URL("../testdata/microbenchmark-pairs-v1.json", here));
const canonicalPairsPath = fileURLToPath(new URL("../../../../specs/fixtures/microbenchmark-pairs-v1.json", here));

function expectMedianAgainstOracle(values: readonly number[], actual: number): void {
  expect(bitsOf(actual)).toBe(bitsOf(exactMedian(values)));
}

function expectIntervalAgainstRank(values: readonly number[], actual: ReturnType<typeof medianInterval>, rank: number): void {
  const expected = expectedInterval(values, rank);
  if (expected === null) {
    expect(actual).toBeNull();
    return;
  }
  expect(actual).not.toBeNull();
  expect(Object.is(actual?.low, expected.low)).toBe(true);
  expect(Object.is(actual?.high, expected.high)).toBe(true);
}

function expectInputUnchanged(values: readonly number[], before: readonly number[]): void {
  expect(values.length).toBe(before.length);
  for (let index = 0; index < before.length; index += 1) {
    expect(bitsOf(values[index])).toBe(bitsOf(before[index]));
  }
}

function throwsSame(thunk: () => unknown, sentinel: Error): void {
  try {
    thunk();
    throw new Error("expected the caller exception to propagate");
  } catch (error) {
    expect(error).toBe(sentinel);
  }
}

function campaignRankProfile(sampleSize: number, confidence: number): boolean[] {
  const key = `${sampleSize}:${confidence}`;
  const cached = rankProfileCache.get(key);
  if (cached !== undefined) return cached;
  const profile = sampleSize <= 12
    ? signEnumerationRankProfile(sampleSize, confidence)
    : pascalRankProfile(sampleSize, confidence);
  rankProfileCache.set(key, profile);
  return profile;
}

const rankProfileCache = new Map<string, boolean[]>();

describe("rank-statistics validation and stable API behavior", () => {
  it("rejects invalid containers, empty samples, and non-finite values", () => {
    for (const invalid of [null, undefined, {}, new Float64Array([1]), "1,2"] as unknown[]) {
      expect(() => median(invalid as number[])).toThrow(TypeError);
      expect(() => medianInterval(invalid as number[], 95)).toThrow(TypeError);
    }
    expect(() => median([])).toThrow(RangeError);
    expect(() => medianInterval([], 95)).toThrow(RangeError);
    for (const invalid of [NaN, Infinity, -Infinity]) {
      expect(() => median([1, invalid])).toThrow(RangeError);
      expect(() => medianInterval([invalid], 95)).toThrow(RangeError);
    }
    expect(() => median([NaN, "bad"] as unknown as number[])).toThrow(RangeError);
    expect(() => median(["bad", NaN] as unknown as number[])).toThrow(TypeError);
  });

  it("applies confidence errors before all sample checks", () => {
    for (const invalid of [null, undefined, "95", true, new Number(95), Symbol("95")]) {
      expect(() => medianInterval([], invalid as number)).toThrow(TypeError);
    }
    for (const invalid of [NaN, Infinity, -Infinity, 0, 100, 1.5, -1]) {
      expect(() => medianInterval([], invalid)).toThrow(RangeError);
    }

    let sampleTouched = false;
    const hostile = new Proxy([], {
      get(_target, property) {
        sampleTouched = true;
        throw new Error(`sample read ${String(property)}`);
      },
      getOwnPropertyDescriptor() {
        sampleTouched = true;
        throw new Error("sample descriptor read");
      },
    });
    expect(() => medianInterval(hostile, 100)).toThrow(RangeError);
    expect(() => medianInterval(hostile, "90" as unknown as number)).toThrow(TypeError);
    expect(sampleTouched).toBe(false);
  });

  it("validates one captured proxy length before checking slots", () => {
    const invalidLengthCases: readonly [unknown, ErrorConstructor][] = [
      [null, TypeError],
      [true, TypeError],
      ["2", TypeError],
      [new Number(2), TypeError],
      [NaN, RangeError],
      [Infinity, RangeError],
      [-Infinity, RangeError],
      [-1, RangeError],
      [0.5, RangeError],
      [0x1_0000_0000, RangeError],
      [0, RangeError],
    ];
    for (const [reportedLength, errorType] of invalidLengthCases) {
      let slotChecks = 0;
      const values = new Proxy([2, 10], {
        get(target, property, receiver) {
          if (property === "length") return reportedLength;
          return Reflect.get(target, property, receiver);
        },
        getOwnPropertyDescriptor(target, property) {
          slotChecks += 1;
          return Reflect.getOwnPropertyDescriptor(target, property);
        },
      });
      expect(() => median(values)).toThrow(errorType);
      expect(slotChecks).toBe(0);
    }

    let lengthReads = 0;
    const valid = new Proxy([2, 10], {
      get(target, property, receiver) {
        if (property === "length") {
          lengthReads += 1;
          return 2;
        }
        return Reflect.get(target, property, receiver);
      },
    });
    expect(median(valid)).toBe(6);
    expect(lengthReads).toBe(1);

    const maximumLengthFailure = new Error("maximum valid length reached slot walk");
    const maximumLength = new Proxy([], {
      get(_target, property) {
        if (property === "length") return 0xffff_ffff;
        return undefined;
      },
      getOwnPropertyDescriptor() {
        throw maximumLengthFailure;
      },
    });
    throwsSame(() => median(maximumLength), maximumLengthFailure);
  });

  it("propagates length, own-property, and slot-access exceptions unchanged", () => {
    const lengthFailure = new Error("length trap sentinel");
    const lengthProxy = new Proxy([], {
      get(_target, property) {
        if (property === "length") throw lengthFailure;
        return undefined;
      },
    });
    throwsSame(() => median(lengthProxy), lengthFailure);

    const descriptorFailure = new Error("own-property trap sentinel");
    const descriptorProxy = new Proxy([1], {
      getOwnPropertyDescriptor() {
        throw descriptorFailure;
      },
    });
    throwsSame(() => median(descriptorProxy), descriptorFailure);

    const slotFailure = new Error("slot getter sentinel");
    const slotProxy = new Proxy([1], {
      get(target, property, receiver) {
        if (property === "0") throw slotFailure;
        return Reflect.get(target, property, receiver);
      },
    });
    throwsSame(() => median(slotProxy), slotFailure);
  });

  it("checks the array brand before reading a lookalike object's length", () => {
    let lengthTouched = false;
    const lookalike = new Proxy({ length: 1 }, {
      get(target, property, receiver) {
        if (property === "length") lengthTouched = true;
        return Reflect.get(target, property, receiver);
      },
    });
    expect(() => median(lookalike as unknown as number[])).toThrow(TypeError);
    expect(lengthTouched).toBe(false);
  });

  it("reads each slot once and never calls caller sort or iterator hooks", () => {
    let slotReads = 0;
    let sortCalls = 0;
    let iteratorCalls = 0;
    const values = [2, 10, 3];
    Object.defineProperty(values, "1", {
      configurable: true,
      enumerable: true,
      get() {
        slotReads += 1;
        return 10;
      },
    });
    values.sort = (() => {
      sortCalls += 1;
      throw new Error("caller sort must not run");
    }) as typeof values.sort;
    Object.defineProperty(values, Symbol.iterator, {
      configurable: true,
      value: () => {
        iteratorCalls += 1;
        throw new Error("caller iterator must not run");
      },
    });

    expect(median(values)).toBe(3);
    expect(slotReads).toBe(1);
    expect(sortCalls).toBe(0);
    expect(iteratorCalls).toBe(0);
  });

  it("rejects holes and inherited-only indexed values", () => {
    const sparse = new Array<number>(1);
    expect(() => median(sparse)).toThrow(TypeError);

    const prototype = Object.create(Array.prototype) as number[];
    prototype[0] = 4;
    const inherited = Object.setPrototypeOf(new Array<number>(1), prototype) as number[];
    expect(() => median(inherited)).toThrow(TypeError);
  });

  it("uses numeric order and exact TypeScript signed-zero behavior", () => {
    expect(median([2, 10, 3])).toBe(3);
    expect(median([2, 10, 3, 20])).toBe(6.5);
    expect(Object.is(median([-0, -0]), -0)).toBe(true);
    expect(Object.is(median([-0, 0]), 0)).toBe(true);
    expect(Object.is(median([0, -0]), 0)).toBe(true);
    expect(median([9, 1, 5])).toBe(5);
  });

  it("returns stable interval endpoint observations without mutating input", () => {
    const values = Object.freeze([3, -0, 2, 1, 0]) as readonly number[];
    const before = [...values];
    const sorted = sortedCopy(values);
    for (const confidence of [1, 50, 75, 95, 99]) {
      const rank = selectedRank(signEnumerationRankProfile(values.length, confidence));
      const result = medianInterval(values, confidence);
      expectIntervalAgainstRank(values, result, rank);
      if (rank > 0) {
        expect(values.includes(result!.low)).toBe(true);
        expect(values.includes(result!.high)).toBe(true);
        expect(Object.is(result!.low, sorted[rank - 1])).toBe(true);
        expect(Object.is(result!.high, sorted[sorted.length - rank])).toBe(true);
      }
    }
    expectInputUnchanged(values, before);
  });
});

describe("exact binary64 median and interval contracts", () => {
  it("matches independently generated exact-rational midpoint bit vectors", () => {
    const fixtureBytes = readFileSync(midpointFixturePath);
    const metadata = JSON.parse(readFileSync(midpointMetadataPath, "utf8")) as MidpointVectorMetadata;
    const fixture = JSON.parse(fixtureBytes.toString("utf8")) as MidpointVectorFile;
    expect(fixture.version).toBe(1);
    expect(metadata.generator).toBe("tests/generate-midpoint-vectors.py");
    expect(metadata.arithmetic).toContain("Fraction.from_float exact rational mean");
    expect(metadata.special_ordered_pair_count).toBeGreaterThan(0);
    expect(metadata.seed).toBe("0x52415449");
    expect(metadata.seeded_accepted_pairs).toBe(256);
    expect(metadata.seeded_rejected_nonfinite_bit_patterns).toBeGreaterThanOrEqual(0);
    expect(metadata.vector_count).toBe(fixture.vectors.length);
    expect(createHash("sha256").update(fixtureBytes).digest("hex")).toBe(metadata.fixture_sha256);
    for (const vector of fixture.vectors) {
      const left = numberFromBits(vector.left_bits);
      const right = numberFromBits(vector.right_bits);
      const expected = numberFromBits(vector.expected_bits);
      expect(bitsOf(exactMidpoint(left, right))).toBe(vector.expected_bits);
      expect(bitsOf(median([left, right]))).toBe(vector.expected_bits);
      expect(bitsOf(expected)).toBe(vector.expected_bits);
    }
  });

  it("matches direct sign enumeration for every rank and confidence at n=1..12", () => {
    for (let sampleSize = 1; sampleSize <= 12; sampleSize += 1) {
      const values = Array.from({ length: sampleSize }, (_, index) => index + 1);
      for (let confidence = 1; confidence <= 99; confidence += 1) {
        const profile = signEnumerationRankProfile(sampleSize, confidence);
        let expectedRank = 0;
        for (let rank = 1; rank <= sampleSize; rank += 1) {
          if (profile[rank - 1]) expectedRank = rank;
        }
        expect(selectedRank(profile)).toBe(expectedRank);
        expectIntervalAgainstRank(values, medianInterval(values, confidence), expectedRank);
      }
    }
  });

  it("differentially checks the multiplicative rank recurrence at BigInt boundaries", () => {
    for (const sampleSize of [63, 64, 65, 127, 128, 129, 257]) {
      const values = Array.from({ length: sampleSize }, (_, index) => index + 1);
      for (let confidence = 1; confidence <= 99; confidence += 1) {
        const profile = pascalRankProfile(sampleSize, confidence);
        const rank = selectedRank(profile);
        expectIntervalAgainstRank(values, medianInterval(values, confidence), rank);
      }
    }
  });

  it("replays canonical paired samples from byte-identical package-local input", () => {
    const localBytes = readFileSync(packagePairsPath);
    const canonicalBytes = readFileSync(canonicalPairsPath);
    expect(localBytes.equals(canonicalBytes)).toBe(true);
    const fixture = JSON.parse(localBytes.toString("utf8")) as PairFixture;
    expect(fixture.version).toBe(1);
    for (const testCase of fixture.cases) {
      const ratios = testCase.candidate_ns.map((candidate) => candidate / testCase.baseline_ns);
      expect(bitsOf(median(ratios))).toBe(bitsOf(testCase.ratio));
      const interval = medianInterval(ratios, fixture.confidence_percent);
      if (testCase.interval === null) {
        expect(interval).toBeNull();
      } else {
        expect(interval).not.toBeNull();
        expect(bitsOf(interval!.low)).toBe(bitsOf(testCase.interval[0]));
        expect(bitsOf(interval!.high)).toBe(bitsOf(testCase.interval[1]));
      }
    }
  });
});

describe("bounded deterministic property campaigns", () => {
  it("checks 2,048 seeded median cases against exact outputs and permutations", () => {
    const rng = new XorShift32(0x4d454449);
    for (let trial = 0; trial < 2048; trial += 1) {
      const values = seededIntegerArray(rng);
      const before = [...values];
      const firstMedian = median(values);
      const reordered = permuted(values, rng);
      const reorderedMedian = median(reordered);
      expectMedianAgainstOracle(values, firstMedian);
      expectMedianAgainstOracle(reordered, reorderedMedian);
      expect(firstMedian === reorderedMedian).toBe(true);
      expectInputUnchanged(values, before);
    }
  });

  it("uses numeric equality for zero-tied permutations but checks each bit oracle", () => {
    const original = [-0, 0, 1];
    const reordered = [0, -0, 1];
    const first = median(original);
    const second = median(reordered);
    expect(first === second).toBe(true);
    expectMedianAgainstOracle(original, first);
    expectMedianAgainstOracle(reordered, second);
  });

  it("checks 2,048 seeded interval cases against exact rank and endpoints", () => {
    const rng = new XorShift32(0x494e5445);
    const confidenceOccurrences = new Array<number>(100).fill(0);
    for (let trial = 0; trial < 2048; trial += 1) {
      const values = seededIntegerArray(rng);
      const confidence = (trial % 99) + 1;
      confidenceOccurrences[confidence] += 1;
      const profile = campaignRankProfile(values.length, confidence);
      expectIntervalAgainstRank(values, medianInterval(values, confidence), selectedRank(profile));
    }
    expect(confidenceOccurrences.slice(1).every((count) => count > 0)).toBe(true);
  });

  describe("the 2,048-array finite-binary64 campaign", () => {
    // Generate the stream once. Each ordinary `it.each` case below owns one
    // disjoint contiguous slice, so Vitest's per-test timer applies to a
    // bounded portion without changing the input sequence or oracle work.
    const campaign = finiteBinary64Campaign(0x52414e4b, 2048);
    const slices = [
      [0, 256],
      [256, 512],
      [512, 768],
      [768, 1024],
      [1024, 1280],
      [1280, 1536],
      [1536, 1792],
      [1792, 2048],
    ] as const;

    it.each(slices)("checks finite-binary64 array indices [%i, %i)", (start, end) => {
      expect(campaign.arrays).toHaveLength(2048);
      expect(Number.isSafeInteger(campaign.rejectedNonFiniteBitPatterns)).toBe(true);
      expect(end - start).toBe(256);
      const confidenceOccurrences = new Array<number>(100).fill(0);
      let medianAssertions = 0;
      let intervalAssertions = 0;
      for (let index = start; index < end; index += 1) {
        const values = campaign.arrays[index];
        expectMedianAgainstOracle(values, median(values));
        medianAssertions += 1;
        for (let confidence = 1; confidence <= 99; confidence += 1) {
          confidenceOccurrences[confidence] += 1;
          const profile = campaignRankProfile(values.length, confidence);
          expectIntervalAgainstRank(values, medianInterval(values, confidence), selectedRank(profile));
          intervalAssertions += 1;
        }
      }
      expect(medianAssertions).toBe(256);
      expect(intervalAssertions).toBe(256 * 99);
      expect(confidenceOccurrences.slice(1).every((count) => count === 256)).toBe(true);
    }, 20_000);
  });
});

describe("negative controls for the shared assertions", () => {
  it("catches lexicographic ordering with the numeric median oracle", () => {
    const values = [2, 10, 3];
    const wrong = [...values].sort() as number[];
    expect(() => expectMedianAgainstOracle(values, wrong[1])).toThrow();
    expectMedianAgainstOracle(values, median(values));
  });

  it("catches a one-rank error with the interval endpoint assertion", () => {
    const values = [9, 1, 7, 3, 5, 11];
    const confidence = 50;
    const trueRank = selectedRank(signEnumerationRankProfile(values.length, confidence));
    expect(trueRank).toBeGreaterThan(0);
    const wrongRank = trueRank - 1;
    expect(() => expectIntervalAgainstRank(values, medianInterval(values, confidence), wrongRank)).toThrow();
    expectIntervalAgainstRank(values, medianInterval(values, confidence), trueRank);
  });

  it("catches naive overflowing midpoint arithmetic with the same bit assertion", () => {
    const values = [Number.MAX_VALUE, Number.MAX_VALUE];
    const naive = (values[0] + values[1]) / 2;
    expect(() => expectMedianAgainstOracle(values, naive)).toThrow();
    expectMedianAgainstOracle(values, median(values));
  });

  it("catches an in-place sort with the shared input-preservation assertion", () => {
    const values = [3, 1, 2];
    const before = [...values];
    values.sort(compareNumbersForControl);
    expect(() => expectInputUnchanged(values, before)).toThrow();
    expectMedianAgainstOracle(before, median(before));
  });
});

function compareNumbersForControl(left: number, right: number): number {
  return left < right ? -1 : left > right ? 1 : 0;
}
