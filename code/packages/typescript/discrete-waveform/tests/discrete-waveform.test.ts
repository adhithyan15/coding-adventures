import assert from "node:assert/strict";
import test from "node:test";
import { ConstantWaveform, SineWaveform } from "@coding-adventures/analog-waveform";
import { SampledWaveform } from "../src/index.js";

function assertClose(actual: number, expected: number, digits = 10): void {
  const tolerance = 0.5 * 10 ** -digits;
  assert.ok(
    Math.abs(actual - expected) < tolerance,
    `Expected ${actual} to be close to ${expected} (tol ${tolerance})`
  );
}

test("can sample an analog waveform", () => {
  const sampled = SampledWaveform.fromAnalog(new ConstantWaveform(3), 10, 4);
  assert.deepEqual(Array.from(sampled.samples), [3, 3, 3, 3]);
  assertClose(sampled.durationSeconds(), 0.4, 10);
  assertClose(sampled.samplePeriodSeconds(), 0.1, 10);
  assert.equal(sampled.sampleAtIndex(0), 3);
  assert.equal(sampled.sampleAtIndex(3), 3);
});

test("zero-order hold returns the current sample value", () => {
  const sampled = new SampledWaveform([0, 1, 2], 2);
  assert.equal(sampled.heldValueAt(-0.5), 0);
  assert.equal(sampled.heldValueAt(0.1), 0);
  assert.equal(sampled.heldValueAt(0.6), 1);
  assert.equal(sampled.heldValueAt(1.1), 2);
  assert.equal(sampled.heldValueAt(10.0), 2);
});

test("samples a sine waveform at the right cadence", () => {
  const sampled = SampledWaveform.fromAnalog(new SineWaveform(1, 1), 4, 4);
  assertClose(sampled.samples[0], 0, 10);
  assertClose(sampled.samples[1], 1, 10);
  assertClose(sampled.samples[2], 0, 10);
  assertClose(sampled.samples[3], -1, 10);
});

test("rejects invalid construction", () => {
  assert.throws(
    () => new SampledWaveform([], 1),
    /SampledWaveform requires at least one sample/
  );
  assert.throws(() => new SampledWaveform([1], 0), /Sample rate must be > 0/);
  assert.throws(
    () => SampledWaveform.fromAnalog(new ConstantWaveform(1), 10, 0),
    /Sample count must be > 0/
  );
});
