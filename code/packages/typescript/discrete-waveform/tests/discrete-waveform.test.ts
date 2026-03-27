import { describe, expect, test } from "vitest";
import { ConstantWaveform, SineWaveform } from "@coding-adventures/analog-waveform";
import { SampledWaveform } from "../src/index";

describe("SampledWaveform", () => {
  test("can sample an analog waveform", () => {
    const sampled = SampledWaveform.fromAnalog(new ConstantWaveform(3), 10, 4);
    expect(sampled.samples).toEqual([3, 3, 3, 3]);
    expect(sampled.durationSeconds()).toBeCloseTo(0.4, 10);
  });

  test("zero-order hold returns the current sample value", () => {
    const sampled = new SampledWaveform([0, 1, 2], 2);
    expect(sampled.heldValueAt(0.1)).toBe(0);
    expect(sampled.heldValueAt(0.6)).toBe(1);
    expect(sampled.heldValueAt(1.1)).toBe(2);
  });

  test("samples a sine waveform at the right cadence", () => {
    const sampled = SampledWaveform.fromAnalog(new SineWaveform(1, 1), 4, 4);
    expect(sampled.samples[0]).toBeCloseTo(0, 10);
    expect(sampled.samples[1]).toBeCloseTo(1, 10);
  });

  test("rejects invalid construction", () => {
    expect(() => new SampledWaveform([], 1)).toThrow(
      "SampledWaveform requires at least one sample"
    );
    expect(() => new SampledWaveform([1], 0)).toThrow("Sample rate must be > 0");
  });
});
