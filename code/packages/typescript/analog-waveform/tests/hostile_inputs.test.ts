import assert from "node:assert/strict";
import test from "node:test";
import { ConstantWaveform, SineWaveform } from "../src/index.js";

test("CWE-20: ConstantWaveform rejects non-finite and non-number inputs", () => {
  for (const bad of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, "5", null, undefined, true]) {
    // @ts-expect-error hostile runtime input
    assert.throws(() => new ConstantWaveform(bad), Error);
    const c = new ConstantWaveform(1);
    // @ts-expect-error hostile runtime input
    assert.throws(() => c.sampleAt(bad), Error);
  }
  const negZero = new ConstantWaveform(-0);
  assert.equal(Object.is(negZero.level, 0), true);
  assert.equal(Object.is(negZero.sampleAt(-0), 0), true);
});

test("CWE-20 / CWE-190: SineWaveform rejects non-finite values, wrong types, and overflow", () => {
  for (const bad of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, "1", null, true]) {
    // @ts-expect-error hostile runtime input
    assert.throws(() => new SineWaveform(bad, 1), Error);
    // @ts-expect-error hostile runtime input
    assert.throws(() => new SineWaveform(1, bad), Error);
    // @ts-expect-error hostile runtime input
    assert.throws(() => new SineWaveform(1, 1, bad), Error);
    // @ts-expect-error hostile runtime input
    assert.throws(() => new SineWaveform(1, 1, 0, bad), Error);
    const s = new SineWaveform(1, 1);
    // @ts-expect-error hostile runtime input
    assert.throws(() => s.sampleAt(bad), Error);
  }

  assert.throws(() => new SineWaveform(1, (Number.MAX_VALUE / (2 * Math.PI)) * 1.0001), RangeError);

  const overflowing = new SineWaveform(Number.MAX_VALUE, 1, Math.PI / 2, Number.MAX_VALUE);
  assert.throws(() => overflowing.sampleAt(0), RangeError);
});

test("CWE-400: subnormal frequency and extreme finite time remain bounded", () => {
  const subnormal = new SineWaveform(1, Number.MIN_VALUE, Number.MAX_VALUE, 0);
  assert.equal(subnormal.periodSeconds(), Number.POSITIVE_INFINITY);
  for (const t of [-Number.MAX_VALUE, -1e100, 0, 1e100, Number.MAX_VALUE]) {
    const y = subnormal.sampleAt(t);
    assert.equal(Number.isFinite(y), true);
    assert.ok(Math.abs(y) <= 1);
  }
});

test("CWE-1321: instances are frozen and forged prototypes fail closed", () => {
  const c = new ConstantWaveform(2);
  const s = new SineWaveform(1, 2);
  assert.equal(Object.isFrozen(c), true);
  assert.equal(Object.isFrozen(s), true);

  assert.throws(() => {
    (c as { level: number }).level = Number.NaN;
  }, TypeError);

  const forgedConst = Object.assign(Object.create(ConstantWaveform.prototype), {
    level: Number.NaN,
  }) as ConstantWaveform;
  assert.throws(() => forgedConst.sampleAt(0), RangeError);

  const forgedSine = Object.assign(Object.create(SineWaveform.prototype), {
    amplitude: 1,
    frequencyHz: Number.NaN,
    phaseRadians: 0,
    offset: 0,
  }) as SineWaveform;
  assert.throws(() => forgedSine.periodSeconds(), RangeError);
  assert.throws(() => forgedSine.angularFrequency(), RangeError);
  assert.throws(() => forgedSine.sampleAt(0), RangeError);

  const forgedZeroAmpNegOffset = Object.assign(Object.create(SineWaveform.prototype), {
    amplitude: 0,
    frequencyHz: 1,
    phaseRadians: 0,
    offset: -0,
  }) as SineWaveform;
  assert.equal(Object.is(forgedZeroAmpNegOffset.sampleAt(0), 0), true);
});
