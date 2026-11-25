import assert from "node:assert/strict";
import test from "node:test";
import { ConstantWaveform } from "@coding-adventures/analog-waveform";
import { MAX_SAMPLES, SampledWaveform } from "../src/index.js";

test("CWE-20: rejects non-arrays, sparse holes, non-finite samples, and bad rates", () => {
  // @ts-expect-error hostile runtime input
  assert.throws(() => new SampledWaveform(null, 10), TypeError);
  // @ts-expect-error hostile runtime input
  assert.throws(() => new SampledWaveform({ 0: 1, length: 1 }, 10), TypeError);

  const sparse = new Array<number>(2);
  sparse[0] = 1;
  assert.throws(() => new SampledWaveform(sparse, 10), TypeError);

  for (const bad of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, "1", null]) {
    // @ts-expect-error hostile runtime input
    assert.throws(() => new SampledWaveform([bad], 10), Error);
    // @ts-expect-error hostile runtime input
    assert.throws(() => new SampledWaveform([1], bad), Error);
  }
  assert.throws(() => new SampledWaveform([1], Number.MIN_VALUE), RangeError);

  const negZero = new SampledWaveform([-0], 10);
  assert.equal(Object.is(negZero.samples[0], 0), true);
});

test("CWE-20 / CWE-400: rejects oversized sample arrays and invalid fromAnalog arguments", () => {
  assert.throws(
    () => SampledWaveform.fromAnalog(new ConstantWaveform(1), 10, MAX_SAMPLES + 1),
    RangeError
  );
  assert.throws(
    () => SampledWaveform.fromAnalog(new ConstantWaveform(1), 10, 1.5),
    TypeError
  );
  assert.throws(
    () => SampledWaveform.fromAnalog(new ConstantWaveform(1), 10, Number.MAX_SAFE_INTEGER + 1),
    TypeError
  );
  // @ts-expect-error hostile runtime input
  assert.throws(() => SampledWaveform.fromAnalog(null, 10, 4), TypeError);
  // @ts-expect-error hostile runtime input
  assert.throws(() => SampledWaveform.fromAnalog({}, 10, 4), TypeError);

  const oversized = new Array<number>(MAX_SAMPLES + 1).fill(0);
  assert.throws(() => new SampledWaveform(oversized, 10), RangeError);
});

test("CWE-20 / CWE-1321: sampleAtIndex and heldValueAt reject hostile indices, times, and forged state", () => {
  const sw = new SampledWaveform([1, 2, 3], 10);
  assert.equal(Object.isFrozen(sw), true);
  assert.equal(Object.isFrozen(sw.samples), true);

  for (const badIdx of [-1, 3, 1.5, Number.NaN, Number.POSITIVE_INFINITY, "0", "__proto__", "constructor"]) {
    // @ts-expect-error hostile runtime input
    assert.throws(() => sw.sampleAtIndex(badIdx), Error);
  }

  for (const badTime of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, "0", null]) {
    // @ts-expect-error hostile runtime input
    assert.throws(() => sw.heldValueAt(badTime), Error);
  }

  // Huge finite time does not overflow time * sampleRateHz
  assert.equal(sw.heldValueAt(Number.MAX_VALUE), 3);

  const forged = Object.assign(Object.create(SampledWaveform.prototype), {
    samples: [],
    sampleRateHz: 10,
  }) as SampledWaveform;
  assert.throws(() => forged.samplePeriodSeconds(), RangeError);
  assert.throws(() => forged.durationSeconds(), RangeError);
  assert.throws(() => forged.sampleAtIndex(0), RangeError);
  assert.throws(() => forged.heldValueAt(0), RangeError);

  const sparseSamples = new Array<number>(2);
  const forgedSparse = Object.assign(Object.create(SampledWaveform.prototype), {
    samples: sparseSamples,
    sampleRateHz: 10,
  }) as SampledWaveform;
  assert.throws(() => forgedSparse.sampleAtIndex(0), RangeError);
  assert.throws(() => forgedSparse.heldValueAt(0), RangeError);

  const forgedNegZero = Object.assign(Object.create(SampledWaveform.prototype), {
    samples: [-0],
    sampleRateHz: 10,
  }) as SampledWaveform;
  assert.equal(Object.is(forgedNegZero.sampleAtIndex(0), 0), true);
  assert.equal(Object.is(forgedNegZero.heldValueAt(0), 0), true);
});
