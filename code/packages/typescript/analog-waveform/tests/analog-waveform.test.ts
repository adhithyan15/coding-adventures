import assert from "node:assert/strict";
import test from "node:test";
import { ConstantWaveform, SineWaveform } from "../src/index.js";

function assertClose(actual: number, expected: number, digits = 10): void {
  const tolerance = 0.5 * 10 ** -digits;
  assert.ok(
    Math.abs(actual - expected) < tolerance,
    `Expected ${actual} to be close to ${expected} (tol ${tolerance})`
  );
}

test("ConstantWaveform returns the same value for any finite time", () => {
  const waveform = new ConstantWaveform(5);
  assert.equal(waveform.sampleAt(0), 5);
  assert.equal(waveform.sampleAt(10), 5);
  assert.equal(waveform.sampleAt(-3), 5);
});

test("SineWaveform evaluates a 1 Hz sine at key points", () => {
  const waveform = new SineWaveform(2, 1);
  assertClose(waveform.sampleAt(0), 0, 10);
  assertClose(waveform.sampleAt(0.25), 2, 10);
  assertClose(waveform.sampleAt(0.5), 0, 10);
  assertClose(waveform.sampleAt(0.75), -2, 10);
});

test("SineWaveform supports offset and zero amplitude", () => {
  const waveform = new SineWaveform(1, 1, 0, 3);
  assertClose(waveform.sampleAt(0), 3, 10);

  const zero = new SineWaveform(0, 1, 0, -2.5);
  assert.equal(zero.sampleAt(0.25), -2.5);
});

test("SineWaveform computes period and angular frequency", () => {
  const waveform = new SineWaveform(1, 4);
  assertClose(waveform.periodSeconds(), 0.25, 10);
  assertClose(waveform.angularFrequency(), 8 * Math.PI, 10);
});

test("SineWaveform rejects invalid amplitude and frequency", () => {
  assert.throws(() => new SineWaveform(-1, 1), /Amplitude must be >= 0/);
  assert.throws(() => new SineWaveform(1, 0), /Frequency must be > 0/);
});
