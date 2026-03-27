import { describe, expect, test } from "vitest";
import { ConstantWaveform, SineWaveform } from "../src/index";

describe("ConstantWaveform", () => {
  test("returns the same value for any time", () => {
    const waveform = new ConstantWaveform(5);
    expect(waveform.sampleAt(0)).toBe(5);
    expect(waveform.sampleAt(10)).toBe(5);
    expect(waveform.sampleAt(-3)).toBe(5);
  });
});

describe("SineWaveform", () => {
  test("evaluates a 1 Hz sine at key points", () => {
    const waveform = new SineWaveform(2, 1);
    expect(waveform.sampleAt(0)).toBeCloseTo(0, 10);
    expect(waveform.sampleAt(0.25)).toBeCloseTo(2, 10);
    expect(waveform.sampleAt(0.5)).toBeCloseTo(0, 10);
    expect(waveform.sampleAt(0.75)).toBeCloseTo(-2, 10);
  });

  test("supports offset", () => {
    const waveform = new SineWaveform(1, 1, 0, 3);
    expect(waveform.sampleAt(0)).toBeCloseTo(3, 10);
  });

  test("computes period", () => {
    const waveform = new SineWaveform(1, 4);
    expect(waveform.periodSeconds()).toBeCloseTo(0.25, 10);
  });

  test("rejects invalid amplitude and frequency", () => {
    expect(() => new SineWaveform(-1, 1)).toThrow("Amplitude must be >= 0");
    expect(() => new SineWaveform(1, 0)).toThrow("Frequency must be > 0");
  });
});
