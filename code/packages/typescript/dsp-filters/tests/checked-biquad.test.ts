import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  CheckedBiquad,
  EmptySignalError,
  InvalidCoefficientError,
  MAX_ORDER,
  MAX_SAMPLES,
  checkedSosfilt,
} from "../src/index.js";

const identity = (): CheckedBiquad => new CheckedBiquad(1, 0, 0, 1, 0, 0);

describe("checked binary64 Biquad", () => {
  it("uses direct division for the minimum-subnormal identity", () => {
    const minimumSubnormal = Number.MIN_VALUE;
    assert.equal(minimumSubnormal / minimumSubnormal, 1);
    const filter = new CheckedBiquad(
      minimumSubnormal,
      0,
      0,
      minimumSubnormal,
      0,
      0,
    );
    assert.deepEqual(Array.from(filter.process([1])), [1]);
  });

  it("accepts only finite primitive numbers and nonzero a0", () => {
    for (const value of [NaN, Infinity, "1", true, 1n, new Number(1)]) {
      assert.throws(
        () => new CheckedBiquad(value as number, 0, 0, 1, 0, 0),
        InvalidCoefficientError,
      );
    }
    const hook = { valueOf: () => { throw new Error("must not coerce"); } };
    assert.throws(
      () => identity().processSample(hook as unknown as number),
      InvalidCoefficientError,
    );
    assert.throws(
      () => new CheckedBiquad(1, 0, 0, -0, 0, 0),
      InvalidCoefficientError,
    );
    assert.throws(
      () => new CheckedBiquad(2, 0, 0, Number.MIN_VALUE, 0, 0),
      InvalidCoefficientError,
    );
  });

  it("rejects non-finite output and either candidate delay without committing", () => {
    const huge = 2 ** 1023;
    const output = new CheckedBiquad(2, 0, 0, 1, 0, 0);
    assert.throws(() => output.processSample(huge), InvalidCoefficientError);
    assert.equal(output.processSample(0), 0);

    const delay1 = new CheckedBiquad(0, 2, 0, 1, 0, 0);
    assert.throws(() => delay1.processSample(huge), InvalidCoefficientError);
    assert.equal(delay1.processSample(0), 0);

    const delay2 = new CheckedBiquad(0, 0, 2, 1, 0, 0);
    assert.throws(() => delay2.processSample(huge), InvalidCoefficientError);
    assert.equal(delay2.processSample(0), 0);
  });

  it("rolls block state back after late invalid values and throwing accessors", () => {
    const delay = new CheckedBiquad(0, 0, 1, 1, 0, 0);
    assert.deepEqual(Array.from(delay.process([1, 2])), [0, 0]);
    assert.throws(
      () => delay.process([3, NaN, 4]),
      InvalidCoefficientError,
    );
    assert.deepEqual(Array.from(delay.process([0, 0])), [1, 2]);

    delay.reset();
    const accessorError = new Error("caller accessor failed");
    const signal = {
      length: 3,
      0: 3,
      get 1(): number { throw accessorError; },
      2: 4,
    } as unknown as ArrayLike<unknown>;
    assert.throws(() => delay.process(signal), (error: unknown) => error === accessorError);
    assert.deepEqual(Array.from(delay.process([0, 0])), [0, 0]);
  });

  it("uses separately rounded binary64 steps and rolls back on overflow", () => {
    const filter = new CheckedBiquad(Number.MAX_VALUE, 0, 0, 1, 1, 0);
    assert.equal(filter.processSample(1), Number.MAX_VALUE);
    assert.throws(() => filter.processSample(2), InvalidCoefficientError);
    assert.equal(filter.processSample(0), -Number.MAX_VALUE);
  });
});

describe("checked SOS", () => {
  it("is one-shot, order-sensitive, and preserves negative zero for empty SOS", () => {
    const huge = 2 ** 1023;
    const half = [0.5, 0, 0, 1, 0, 0];
    const gain2 = [2, 0, 0, 1, 0, 0];
    assert.deepEqual(Array.from(checkedSosfilt([half, gain2], [huge])), [huge]);
    assert.throws(() => checkedSosfilt([gain2, half], [huge]), InvalidCoefficientError);

    const identityOutput = checkedSosfilt([], [-0, 1]);
    assert.equal(Object.is(identityOutput[0], -0), true);
    assert.equal(identityOutput[1], 1);
    assert.throws(() => checkedSosfilt([], []), EmptySignalError);
  });

  it("validates signal before section access and row shape before coefficient access", () => {
    let sectionLengthReads = 0;
    const poisonSections = Object.defineProperty({}, "length", {
      get() { sectionLengthReads += 1; throw new Error("section length touched"); },
    }) as ArrayLike<unknown>;
    assert.throws(() => checkedSosfilt(poisonSections, []), EmptySignalError);
    assert.equal(sectionLengthReads, 0);

    let signalIndexReads = 0;
    const overlongSignal = Object.defineProperties({}, {
      length: { value: MAX_SAMPLES + 1 },
      0: { get() { signalIndexReads += 1; throw new Error("sample touched"); } },
    }) as ArrayLike<unknown>;
    assert.throws(
      () => checkedSosfilt(poisonSections, overlongSignal),
      InvalidCoefficientError,
    );
    assert.equal(signalIndexReads, 0);
    assert.equal(sectionLengthReads, 0);

    for (const rowLength of [5, 7]) {
      let coefficientReads = 0;
      const row = Object.defineProperties({}, {
        length: { value: rowLength },
        0: { get() { coefficientReads += 1; throw new Error("coefficient touched"); } },
      }) as ArrayLike<unknown>;
      assert.throws(() => checkedSosfilt([row], [1]), InvalidCoefficientError);
      assert.equal(coefficientReads, 0);
    }
  });

  it("accepts the exact sample and SOS section limits", () => {
    const output = identity().process(new Float64Array(MAX_SAMPLES));
    assert.equal(output.length, MAX_SAMPLES);
    assert.equal(output[0], 0);
    assert.equal(output[MAX_SAMPLES - 1], 0);

    const sections = Array.from({ length: MAX_ORDER }, () => [1, 0, 0, 1, 0, 0]);
    assert.deepEqual(Array.from(checkedSosfilt(sections, [1])), [1]);
  });
});
