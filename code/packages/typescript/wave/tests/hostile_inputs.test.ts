import assert from "node:assert/strict";
import test from "node:test";
import { PI } from "trig/dist/trig.js";
import { Wave } from "../src/wave.js";

test("CWE-20: rejects non-number runtime values and non-finite floats", () => {
  const badValues: unknown[] = [
    Number.NaN,
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
    "1.0",
    null,
    undefined,
    true,
    false,
    {},
    [],
    1n,
  ];
  for (const bad of badValues) {
    // @ts-expect-error hostile runtime input
    assert.throws(() => new Wave(bad, 1.0, 0.0), Error);
    // @ts-expect-error hostile runtime input
    assert.throws(() => new Wave(1.0, bad, 0.0), Error);
    if (bad !== undefined) {
      // @ts-expect-error hostile runtime input
      assert.throws(() => new Wave(1.0, 1.0, bad), Error);
    }
    const wave = new Wave(1.0, 1.0, 0.0);
    // @ts-expect-error hostile runtime input
    assert.throws(() => wave.evaluate(bad), Error);
  }
});

test("CWE-20 / CWE-190: rejects angular frequency overflow and normalizes -0", () => {
  assert.throws(() => new Wave(-Number.MIN_VALUE, 1.0), RangeError);
  assert.throws(() => new Wave(1.0, -0.0), RangeError);
  assert.throws(() => new Wave(1.0, (Number.MAX_VALUE / (2.0 * PI)) * 1.0001), RangeError);

  const negZero = new Wave(-0.0, 1.0, -0.0);
  assert.equal(Object.is(negZero.amplitude, 0.0), true);
  assert.equal(Object.is(negZero.phase, 0.0), true);
  assert.equal(Object.is(negZero.evaluate(0.25), 0.0), true);
});

test("CWE-400: extreme finite and subnormal inputs complete in O(1) and remain bounded", () => {
  const subnormal = new Wave(1.0, Number.MIN_VALUE, Number.MAX_VALUE);
  assert.equal(subnormal.period(), Number.POSITIVE_INFINITY);
  for (const t of [-Number.MAX_VALUE, -1e100, -1.0, 0.0, 1.0, 1e100, Number.MAX_VALUE]) {
    const y = subnormal.evaluate(t);
    assert.equal(Number.isFinite(y), true);
    assert.ok(Math.abs(y) <= 1.0);
  }

  const huge = new Wave(Number.MAX_VALUE, 1e300, -Number.MAX_VALUE);
  for (const t of [-Number.MAX_VALUE, -1e300, 0.0, 1e300, Number.MAX_VALUE]) {
    const y = huge.evaluate(t);
    assert.equal(Number.isFinite(y), true);
    assert.ok(Math.abs(y) <= Number.MAX_VALUE);
  }
});

test("CWE-1321: Wave instance is frozen and immune to prototype pollution", () => {
  const wave = new Wave(1.0, 440.0, 0.0);
  assert.equal(Object.isFrozen(wave), true);
  assert.throws(() => {
    (wave as { amplitude: number }).amplitude = -5.0;
  }, TypeError);

  const polluted = JSON.parse('{"__proto__": {"amplitude": -999}}');
  assert.equal(({} as Record<string, unknown>)["amplitude"], undefined);
  assert.notEqual(polluted, null);
  assert.equal(wave.amplitude, 1.0);

  const forged = Object.assign(Object.create(Wave.prototype), {
    amplitude: Number.NaN,
    frequency: 1.0,
    phase: 0.0,
  }) as Wave;
  assert.throws(() => forged.period(), RangeError);
  assert.throws(() => forged.angularFrequency(), RangeError);
  assert.throws(() => forged.evaluate(0.0), RangeError);
});
