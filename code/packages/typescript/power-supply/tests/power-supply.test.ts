import { describe, expect, test } from "vitest";
import { IdealDcSupply, IdealSineSupply } from "../src/index";

describe("IdealDcSupply", () => {
  test("returns a constant waveform", () => {
    const supply = new IdealDcSupply(5);
    expect(supply.asWaveform().sampleAt(0)).toBe(5);
    expect(supply.asWaveform().sampleAt(10)).toBe(5);
  });

  test("computes power from current", () => {
    const supply = new IdealDcSupply(12);
    expect(supply.powerForCurrent(0.5)).toBeCloseTo(6, 10);
  });
});

describe("IdealSineSupply", () => {
  test("creates a sinusoidal waveform", () => {
    const supply = new IdealSineSupply(2, 1);
    expect(supply.asWaveform().sampleAt(0.25)).toBeCloseTo(2, 10);
  });
});
