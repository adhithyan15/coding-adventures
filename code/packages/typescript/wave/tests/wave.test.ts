// ============================================================================
// wave.test.ts — Comprehensive Tests for the Wave Class
// ============================================================================

import assert from "node:assert/strict";
import test from "node:test";
import { PI } from "trig/dist/trig.js";
import { Wave } from "../src/wave.js";

function assertClose(actual: number, expected: number, digits = 10): void {
  const tolerance = 0.5 * 10 ** -digits;
  assert.ok(
    Math.abs(actual - expected) < tolerance,
    `Expected ${actual} to be close to ${expected} (tol ${tolerance})`
  );
}

test("wave at t=0 with phase 0 gives 0", () => {
  const wave = new Wave(5.0, 1.0, 0.0);
  assertClose(wave.evaluate(0.0), 0.0, 10);
});

test("1 Hz wave reaches amplitude at t=0.25", () => {
  const wave = new Wave(3.0, 1.0, 0.0);
  assertClose(wave.evaluate(0.25), 3.0, 10);
});

test("1 Hz wave returns to zero at t=0.5", () => {
  const wave = new Wave(2.0, 1.0, 0.0);
  assertClose(wave.evaluate(0.5), 0.0, 10);
});

test("1 Hz wave reaches negative amplitude at t=0.75", () => {
  const wave = new Wave(4.0, 1.0, 0.0);
  assertClose(wave.evaluate(0.75), -4.0, 10);
});

test("zero amplitude wave is always zero", () => {
  const wave = new Wave(0.0, 5.0, 0.0);
  assertClose(wave.evaluate(0.0), 0.0, 10);
  assertClose(wave.evaluate(0.25), 0.0, 10);
  assertClose(wave.evaluate(1.0), 0.0, 10);
});

test("wave repeats after one period", () => {
  const wave = new Wave(2.5, 3.0, 0.5);
  const period = wave.period();

  for (const t of [0.0, 0.1, 0.25, 0.33]) {
    assertClose(wave.evaluate(t + period), wave.evaluate(t), 10);
  }
});

test("wave repeats after multiple periods", () => {
  const wave = new Wave(1.0, 2.0, 0.0);
  const t = 0.1;
  const period = wave.period();

  assertClose(wave.evaluate(t + 5 * period), wave.evaluate(t), 10);
});

test("phase PI/2 starts at peak", () => {
  const wave = new Wave(1.0, 1.0, PI / 2);
  assertClose(wave.evaluate(0.0), 1.0, 10);
});

test("phase PI inverts the wave", () => {
  const wave = new Wave(1.0, 1.0, PI);
  assertClose(wave.evaluate(0.25), -1.0, 10);
});

test("phase is stored correctly and default phase is 0", () => {
  const wave = new Wave(1.0, 1.0, 1.234);
  assert.equal(wave.phase, 1.234);

  const defaultWave = new Wave(1.0, 1.0);
  assert.equal(defaultWave.phase, 0.0);
});

test("period is reciprocal of frequency", () => {
  assertClose(new Wave(1.0, 2.0).period(), 0.5, 10);
  assertClose(new Wave(1.0, 1.0).period(), 1.0, 10);
  assertClose(new Wave(1.0, 440.0).period(), 1.0 / 440.0, 10);
});

test("angular frequency is 2*PI*f", () => {
  assertClose(new Wave(1.0, 1.0).angularFrequency(), 2.0 * PI, 10);
  assertClose(new Wave(1.0, 50.0).angularFrequency(), 100.0 * PI, 10);
});

test("properties are stored correctly", () => {
  const wave = new Wave(3.5, 7.5, 0.1);
  assert.equal(wave.amplitude, 3.5);
  assert.equal(wave.frequency, 7.5);
  assert.equal(wave.phase, 0.1);
});

test("validation rejects invalid parameters and non-finite time", () => {
  assert.throws(() => new Wave(-1.0, 1.0), /Amplitude must be non-negative/);
  assert.throws(() => new Wave(1.0, 0.0), /Frequency must be positive/);
  assert.throws(() => new Wave(1.0, -5.0), /Frequency must be positive/);

  const invalidCases: [number, number, number][] = [
    [Number.NaN, 1.0, 0.0],
    [Number.POSITIVE_INFINITY, 1.0, 0.0],
    [1.0, Number.NaN, 0.0],
    [1.0, Number.POSITIVE_INFINITY, 0.0],
    [1.0, Number.MAX_VALUE, 0.0],
    [1.0, 1.0, Number.NaN],
    [1.0, 1.0, Number.POSITIVE_INFINITY],
  ];
  for (const [amplitude, frequency, phase] of invalidCases) {
    assert.throws(() => new Wave(amplitude, frequency, phase), Error);
  }

  const zeroWave = new Wave(0.0, 1.0);
  for (const t of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
    assert.throws(() => zeroWave.evaluate(t), /Time must be finite/);
  }
});

test("extreme inputs stay finite and bounded", () => {
  const zero = new Wave(0.0, Number.MIN_VALUE, Number.MAX_VALUE);
  assert.equal(zero.period(), Number.POSITIVE_INFINITY);
  assert.equal(Object.is(zero.evaluate(Number.MAX_VALUE), 0.0), true);

  const wave = new Wave(Number.MAX_VALUE, Number.MIN_VALUE, Number.MAX_VALUE);
  const result = wave.evaluate(Number.MAX_VALUE);
  assert.equal(Number.isFinite(result), true);
  assert.ok(Math.abs(result) <= Number.MAX_VALUE);
});

test("10 Hz wave peaks at t=0.025 and crosses zero at t=0.05", () => {
  const wave = new Wave(2.0, 10.0, 0.0);
  assertClose(wave.evaluate(0.025), 2.0, 10);
  assertClose(wave.evaluate(0.05), 0.0, 10);
});
