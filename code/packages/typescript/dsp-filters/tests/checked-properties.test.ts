import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  CheckedBiquad,
  InvalidCoefficientError,
} from "../src/index.js";
import {
  checkProperty,
  CoverageBitmap,
  crashSignature,
  fuzzTarget,
  minimizeCrashingInput,
  SplitMix64,
  type FuzzOutcome,
} from "@coding-adventures/test-bootstrap";

type DirectCase = {
  coefficients: [number, number, number];
  samples: number[];
};

type FeedbackCase = DirectCase & { sign1: -1 | 1; sign2: -1 | 1 };

function boundedCase(rng: SplitMix64, maxLength: number): DirectCase {
  const coefficients = [
    rng.bounded(9) - 4,
    rng.bounded(9) - 4,
    rng.bounded(9) - 4,
  ] as [number, number, number];
  const length = 1 + rng.bounded(maxLength);
  const samples = Array.from({ length }, () => rng.bounded(17) - 8);
  return { coefficients, samples };
}

function directExpected(input: DirectCase): number[] {
  return input.samples.map((_, index) => {
    let numerator = 0n;
    for (let tap = 0; tap < 3; tap += 1) {
      if (index >= tap) {
        numerator += BigInt(input.coefficients[tap]) * BigInt(input.samples[index - tap]);
      }
    }
    return Number(numerator) / 128;
  });
}

function directProperty(input: DirectCase, sabotage = false): boolean {
  const [c0, c1, c2] = input.coefficients;
  const filter = new CheckedBiquad(c0 / 4, c1 / 4, c2 / 4, 2, 0, 0);
  const actual = Array.from(filter.process(input.samples.map((sample) => sample / 16)));
  if (sabotage) actual[0] += 1;
  const expected = directExpected(input);
  return actual.length === expected.length && actual.every(
    (value, index) => Number.isFinite(value) && value === expected[index],
  );
}

function feedbackExpected(input: FeedbackCase): number[] {
  let previous1 = 0n;
  let previous2 = 0n;
  return input.samples.map((_, index) => {
    let direct = 0n;
    for (let tap = 0; tap < 3; tap += 1) {
      if (index >= tap) {
        direct += BigInt(input.coefficients[tap]) * BigInt(input.samples[index - tap]);
      }
    }
    const scaled = (1n << BigInt(index)) * direct
      - BigInt(input.sign1) * previous1
      - BigInt(input.sign2) * previous2;
    previous2 = previous1;
    previous1 = scaled;
    return Number(scaled) / 2 ** (7 + index);
  });
}

function feedbackProperty(input: FeedbackCase, sabotage = false): boolean {
  const [c0, c1, c2] = input.coefficients;
  const filter = new CheckedBiquad(
    c0 / 4,
    c1 / 4,
    c2 / 4,
    2,
    input.sign1,
    input.sign2 / 2,
  );
  const actual = Array.from(filter.process(input.samples.map((sample) => sample / 16)));
  if (sabotage) actual[0] += 1;
  const expected = feedbackExpected(input);
  return actual.length === expected.length && actual.every(
    (value, index) => Number.isFinite(value) && value === expected[index],
  );
}

function shrinkInts(value: number): number[] {
  if (value === 0) return [];
  const half = Math.trunc(value / 2);
  return half === 0 ? [0] : [0, half];
}

function shrinkDirectCase(
  initial: DirectCase,
  property: (value: DirectCase) => boolean,
): [DirectCase, number] {
  let current: DirectCase = {
    coefficients: [...initial.coefficients] as [number, number, number],
    samples: [...initial.samples],
  };
  let steps = 0;
  let changed = true;
  while (changed && steps < 256) {
    changed = false;
    for (let i = 0; i < 3 && !changed; i += 1) {
      for (const value of shrinkInts(current.coefficients[i])) {
        const candidate = { ...current, coefficients: [...current.coefficients] as [number, number, number] };
        candidate.coefficients[i] = value;
        steps += 1;
        if (!property(candidate)) {
          current = candidate;
          changed = true;
          break;
        }
      }
    }
    if (changed) continue;
    if (current.samples.length > 1) {
      const candidate = { ...current, samples: current.samples.slice(0, Math.max(1, Math.floor(current.samples.length / 2))) };
      steps += 1;
      if (!property(candidate)) {
        current = candidate;
        changed = true;
        continue;
      }
    }
    for (let i = 0; i < current.samples.length && !changed; i += 1) {
      for (const value of shrinkInts(current.samples[i])) {
        const candidate = { ...current, samples: [...current.samples] };
        candidate.samples[i] = value;
        steps += 1;
        if (!property(candidate)) {
          current = candidate;
          changed = true;
          break;
        }
      }
    }
  }
  return [current, steps];
}

function shrinkFeedbackCase(
  initial: FeedbackCase,
  property: (value: FeedbackCase) => boolean,
): [FeedbackCase, number] {
  const [direct, steps] = shrinkDirectCase(initial, (value) =>
    property({ ...value, sign1: initial.sign1, sign2: initial.sign2 }),
  );
  return [{ ...direct, sign1: initial.sign1, sign2: initial.sign2 }, steps];
}

function inDirectDomain(value: DirectCase): boolean {
  return value.coefficients.length === 3
    && value.coefficients.every((item) => Number.isInteger(item) && item >= -4 && item <= 4)
    && value.samples.length >= 1
    && value.samples.length <= 32
    && value.samples.every((item) => Number.isInteger(item) && item >= -8 && item <= 8);
}

function fuzzValue(bytes: Uint8Array): number | undefined {
  if (bytes.length !== 8) return undefined;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return view.getFloat64(0, false);
}

function encodeFloat(value: number): Uint8Array {
  const bytes = new Uint8Array(8);
  new DataView(bytes.buffer).setFloat64(0, value, false);
  return bytes;
}

function checkedOutcome(input: Uint8Array, coverage: CoverageBitmap, faulty = false): FuzzOutcome {
  const sample = fuzzValue(input);
  if (sample === undefined) {
    coverage.recordEdge(0);
    return { status: "pass" };
  }
  try {
    const identity = new CheckedBiquad(1, 0, 0, 1, 0, 0);
    if (!Number.isFinite(sample)) {
      try {
        identity.processSample(sample);
        return { status: "crash", kind: "nonfinite-accepted", frames: ["checkedOutcome"] };
      } catch (error) {
        if (!(error instanceof InvalidCoefficientError)) {
          return { status: "crash", kind: "wrong-error", frames: ["checkedOutcome"] };
        }
      }
      const unchanged = Array.from(identity.process([0, 0, 0]));
      if (!unchanged.every((value) => value === 0)) {
        return { status: "crash", kind: "rejection-mutated-state", frames: ["checkedOutcome"] };
      }
      const bits = new DataView(input.buffer, input.byteOffset, input.byteLength).getBigUint64(0, false);
      const exponent = Number((bits >> 52n) & 0x7ffn);
      coverage.recordEdge(16 + exponent);
      return { status: "pass" };
    }

    let actual = identity.processSample(sample);
    if (faulty) actual += 1;
    if (actual !== sample || !Number.isFinite(actual)) {
      return { status: "crash", kind: "identity-oracle-mismatch", frames: ["checkedOutcome"] };
    }
    const delay = new CheckedBiquad(0, 0, 1, 1, 0, 0);
    const delayed = [delay.processSample(sample), delay.processSample(0), delay.processSample(0)];
    if (delayed[0] !== 0 || delayed[1] !== 0 || delayed[2] !== sample) {
      return { status: "crash", kind: "delay-state-mismatch", frames: ["checkedOutcome"] };
    }
    const bits = new DataView(input.buffer, input.byteOffset, input.byteLength).getBigUint64(0, false);
    const exponent = Number((bits >> 52n) & 0x7ffn);
    coverage.recordEdge(1 + exponent);
    coverage.recordEdge(sample === 0 ? 2048 : sample < 0 ? 2049 : 2050);
    if (Math.abs(sample) < 2 ** -1022 && sample !== 0) coverage.recordEdge(2051);
    return { status: "pass" };
  } catch {
    return { status: "crash", kind: "unexpected-target-exception", frames: ["checkedOutcome"] };
  }
}

function reportKey(report: ReturnType<typeof fuzzTarget>): string {
  return JSON.stringify({
    iterations: report.iterations,
    admittedSeeds: report.admittedSeeds,
    totalEdges: report.totalEdges,
    triage: report.triage,
    mutation: report.mutation,
    usedFallbackSeed: report.usedFallbackSeed,
    clean: report.clean,
    corpusLimitDrops: report.corpusLimitDrops,
    unrecordedCrashOutcomes: report.unrecordedCrashOutcomes,
    crashes: report.crashes.map((crash) => ({
      signature: crash.signature,
      hits: crash.hits,
      bytes: Array.from(crash.minimalInput),
    })),
  });
}

describe("checked Biquad bounded VER05/VER06 evidence", () => {
  it("checks 512 exact integer FIR and feedback recurrence cases with domain-preserving shrinkers", () => {
    const direct = checkProperty(
      0xAF02B101,
      { numTests: 512, isCi: true },
      (rng) => boundedCase(rng, 32),
      directProperty,
      shrinkDirectCase,
    );
    assert.equal(direct.status, "pass");

    const feedback = checkProperty(
      0xAF02B102,
      { numTests: 512, isCi: true },
      (rng): FeedbackCase => ({
        ...boundedCase(rng, 12),
        sign1: rng.bounded(2) === 0 ? -1 : 1,
        sign2: rng.bounded(2) === 0 ? -1 : 1,
      }),
      feedbackProperty,
      shrinkFeedbackCase,
    );
    assert.equal(feedback.status, "pass");

    const witness: DirectCase = { coefficients: [1, 0, 0], samples: [1] };
    let minimized: DirectCase | undefined;
    let shrinkSteps = 0;
    const faulty = checkProperty(
      0xAF02BF05,
      { numTests: 1, isCi: true },
      () => witness,
      (value) => directProperty(value, true),
      (value, property) => {
        [minimized, shrinkSteps] = shrinkDirectCase(value, property);
        return [minimized, shrinkSteps];
      },
    );
    assert.equal(faulty.status, "fail");
    assert.ok(minimized);
    assert.ok(shrinkSteps > 0);
    assert.ok(inDirectDomain(minimized));
    assert.equal(directProperty(minimized, true), false);
    assert.equal(directProperty(minimized), true);
    let replayMinimized: DirectCase | undefined;
    let replaySteps = 0;
    const faultyReplay = checkProperty(
      0xAF02BF05,
      { numTests: 1, isCi: true },
      () => witness,
      (value) => directProperty(value, true),
      (value, property) => {
        [replayMinimized, replaySteps] = shrinkDirectCase(value, property);
        return [replayMinimized, replaySteps];
      },
    );
    assert.equal(faultyReplay.status, "fail");
    assert.deepEqual(replayMinimized, minimized);
    assert.equal(replaySteps, shrinkSteps);

    const feedbackWitness: FeedbackCase = {
      coefficients: [1, 0, 0],
      samples: [1],
      sign1: 1,
      sign2: -1,
    };
    let feedbackMinimized: FeedbackCase | undefined;
    let feedbackShrinkSteps = 0;
    const faultyFeedback = checkProperty(
      0xAF02BF06,
      { numTests: 1, isCi: true },
      () => feedbackWitness,
      (value) => feedbackProperty(value, true),
      (value, property) => {
        const [shrunk, steps] = shrinkFeedbackCase(value, property);
        assert.ok(steps > 0);
        feedbackMinimized = shrunk;
        feedbackShrinkSteps = steps;
        return [shrunk, steps];
      },
    );
    assert.equal(faultyFeedback.status, "fail");
    assert.ok(feedbackMinimized);
    assert.ok(inDirectDomain(feedbackMinimized));
    assert.ok(feedbackMinimized.sign1 === -1 || feedbackMinimized.sign1 === 1);
    assert.ok(feedbackMinimized.sign2 === -1 || feedbackMinimized.sign2 === 1);
    assert.equal(feedbackProperty(feedbackMinimized, true), false);
    assert.equal(feedbackProperty(feedbackMinimized), true);
    let feedbackReplayMinimized: FeedbackCase | undefined;
    let feedbackReplaySteps = 0;
    const faultyFeedbackReplay = checkProperty(
      0xAF02BF06,
      { numTests: 1, isCi: true },
      () => feedbackWitness,
      (value) => feedbackProperty(value, true),
      (value, property) => {
        [feedbackReplayMinimized, feedbackReplaySteps] = shrinkFeedbackCase(value, property);
        return [feedbackReplayMinimized, feedbackReplaySteps];
      },
    );
    assert.equal(faultyFeedbackReplay.status, "fail");
    assert.deepEqual(feedbackReplayMinimized, feedbackMinimized);
    assert.equal(feedbackReplaySteps, feedbackShrinkSteps);
  });

  it("checks fixed-seed raw binary64 state/error cases separately", () => {
    const rng = new SplitMix64(0xAF02B103);
    let finite = 0;
    let nonfinite = 0;
    for (let index = 0; index < 128; index += 1) {
      const bytes = new Uint8Array(8);
      new DataView(bytes.buffer).setBigUint64(0, rng.nextU64(), false);
      const sample = fuzzValue(bytes)!;
      const delay = new CheckedBiquad(0, 0, 1, 1, 0, 0);
      if (Number.isFinite(sample)) {
        finite += 1;
        assert.equal(delay.processSample(sample), 0);
        assert.equal(delay.processSample(0), 0);
        assert.equal(delay.processSample(0), sample);
      } else {
        nonfinite += 1;
        assert.throws(() => delay.processSample(sample), InvalidCoefficientError);
        assert.deepEqual(Array.from(delay.process([0, 0, 0])), [0, 0, 0]);
      }
    }
    assert.equal(finite + nonfinite, 128);
    assert.equal(finite, 128);
    assert.equal(nonfinite, 0);

    const boundaryBits = [
      0x0000000000000000n,
      0x8000000000000000n,
      0x0000000000000001n,
      0x0010000000000000n,
      0x7fefffffffffffffn,
      0x7ff0000000000000n,
      0xfff0000000000000n,
      0x7ff8000000000001n,
    ];
    let boundaryFinite = 0;
    let boundaryNonfinite = 0;
    for (const bits of boundaryBits) {
      const bytes = new Uint8Array(8);
      new DataView(bytes.buffer).setBigUint64(0, bits, false);
      const sample = fuzzValue(bytes)!;
      const delay = new CheckedBiquad(0, 0, 1, 1, 0, 0);
      if (Number.isFinite(sample)) {
        boundaryFinite += 1;
        assert.equal(delay.processSample(sample), 0);
        assert.equal(delay.processSample(0), 0);
        assert.ok(delay.processSample(0) === sample);
      } else {
        boundaryNonfinite += 1;
        assert.throws(() => delay.processSample(sample), InvalidCoefficientError);
        assert.deepEqual(Array.from(delay.process([0, 0, 0])), [0, 0, 0]);
      }
    }
    assert.equal(boundaryFinite, 5);
    assert.equal(boundaryNonfinite, 3);
  });

  it("runs structured raw-bit fuzz with manual semantic outcome feedback, replay, and fault minimization", () => {
    const initial = [0, 1, -1, Number.MIN_VALUE, Number.MAX_VALUE, NaN, Infinity, -Infinity]
      .map(encodeFloat);
    const options = { maxIterations: 1024, seed: 0xAF02BF01, maxInputLen: 8 };
    const first = fuzzTarget(initial, options, (input, coverage) => checkedOutcome(input, coverage));
    const replay = fuzzTarget(initial, options, (input, coverage) => checkedOutcome(input, coverage));
    assert.equal(first.iterations, 1024);
    assert.equal(first.triage.calls, initial.length);
    assert.equal(first.mutation.calls, 1024);
    assert.equal(first.triage.calls, first.triage.pass + first.triage.crash + first.triage.timeout + first.triage.oom);
    assert.equal(first.mutation.calls, first.mutation.pass + first.mutation.crash + first.mutation.timeout + first.mutation.oom);
    assert.equal(first.clean, true);
    assert.equal(first.admittedSeeds, 196);
    assert.equal(first.totalEdges, 198);
    assert.equal(first.crashes.length, 0);
    assert.equal(reportKey(replay), reportKey(first));

    const witness = encodeFloat(1);
    const faultyRun = fuzzTarget(
      [witness],
      { maxIterations: 16, seed: 0xAF02BF02, maxInputLen: 8 },
      (input, coverage) => checkedOutcome(input, coverage, true),
    );
    assert.equal(faultyRun.iterations, 0);
    assert.equal(faultyRun.triage.calls, 1);
    assert.equal(faultyRun.triage.crash, 1);
    assert.equal(faultyRun.mutation.calls, 0);
    assert.equal(faultyRun.clean, false);
    assert.equal(faultyRun.admittedSeeds, 0);
    assert.equal(faultyRun.totalEdges, 0);
    assert.equal(faultyRun.crashes.length, 1);
    const crash = faultyRun.crashes.find((item) => item.kind === "identity-oracle-mismatch");
    assert.ok(crash);
    const minimized = minimizeCrashingInput(
      witness,
      crashSignature(crash.kind, crash.frames),
      (input) => checkedOutcome(input, new CoverageBitmap(), true),
    );
    assert.deepEqual(Array.from(minimized), Array<number>(8).fill(0));
    assert.notDeepEqual(Array.from(minimized), Array.from(witness));
    assert.equal(fuzzValue(minimized), 0);
    assert.equal(checkedOutcome(minimized, new CoverageBitmap(), true).status, "crash");
    assert.equal(checkedOutcome(minimized, new CoverageBitmap()).status, "pass");
    const healthyWitness = checkedOutcome(witness, new CoverageBitmap());
    assert.equal(healthyWitness.status, "pass");
  });
});
