/**
 * Tests for electrical analysis functions.
 */

import { describe, it, expect } from "vitest";
import {
  computeNoiseMargins,
  analyzePower,
  analyzeTiming,
  compareCmosVsTtl,
  demonstrateCmosScaling,
} from "../src/analysis.js";
import { CMOSInverter, CMOSNand, CMOSNor } from "../src/cmos_gates.js";
import { TTLNand } from "../src/ttl_gates.js";

describe("Noise Margins", () => {
  it("CMOS noise margins should be positive", () => {
    const nm = computeNoiseMargins(new CMOSInverter());
    expect(nm.nml).toBeGreaterThan(0);
    expect(nm.nmh).toBeGreaterThan(0);
  });

  it("CMOS noise margins should be roughly symmetric", () => {
    const nm = computeNoiseMargins(new CMOSInverter());
    expect(Math.abs(nm.nml - nm.nmh)).toBeLessThan(nm.nml * 0.5);
  });

  it("TTL noise margins should be positive", () => {
    const nm = computeNoiseMargins(new TTLNand());
    expect(nm.nml).toBeGreaterThan(0);
    expect(nm.nmh).toBeGreaterThan(0);
  });

  it("CMOS output LOW should be near 0V", () => {
    const nm = computeNoiseMargins(new CMOSInverter());
    expect(nm.vol).toBeLessThan(0.1);
  });

  it("TTL output LOW should be near Vce_sat", () => {
    const nm = computeNoiseMargins(new TTLNand());
    expect(nm.vol).toBeLessThan(0.5);
  });
});

describe("Power Analysis", () => {
  it("CMOS should have near-zero static power", () => {
    const power = analyzePower(new CMOSInverter());
    expect(power.staticPower).toBeLessThan(1e-9);
  });

  it("TTL should have milliwatt-level static power", () => {
    const power = analyzePower(new TTLNand());
    expect(power.staticPower).toBeGreaterThan(1e-3);
  });

  it("dynamic power should be positive at non-zero frequency", () => {
    const power = analyzePower(new CMOSInverter(), 1e9);
    expect(power.dynamicPower).toBeGreaterThan(0);
  });

  it("total power should be static + dynamic", () => {
    const power = analyzePower(new CMOSInverter(), 1e9);
    expect(
      Math.abs(power.totalPower - (power.staticPower + power.dynamicPower)),
    ).toBeLessThan(1e-15);
  });

  it("energy per switch should be positive", () => {
    const power = analyzePower(new CMOSInverter());
    expect(power.energyPerSwitch).toBeGreaterThan(0);
  });

  it("CMOSNand should work with analyzePower", () => {
    const power = analyzePower(new CMOSNand());
    expect(power.staticPower).toBe(0.0);
  });

  it("CMOSNor should work with analyzePower", () => {
    const power = analyzePower(new CMOSNor());
    expect(power.staticPower).toBe(0.0);
  });
});

describe("Timing Analysis", () => {
  it("CMOS propagation delays should be positive", () => {
    const timing = analyzeTiming(new CMOSInverter());
    expect(timing.tphl).toBeGreaterThan(0);
    expect(timing.tplh).toBeGreaterThan(0);
    expect(timing.tpd).toBeGreaterThan(0);
  });

  it("tpd should be the average of tphl and tplh", () => {
    const timing = analyzeTiming(new CMOSInverter());
    const expected = (timing.tphl + timing.tplh) / 2.0;
    expect(Math.abs(timing.tpd - expected)).toBeLessThan(1e-20);
  });

  it("CMOS delay should be faster than TTL delay", () => {
    const cmosTiming = analyzeTiming(new CMOSInverter());
    const ttlTiming = analyzeTiming(new TTLNand());
    expect(cmosTiming.tpd).toBeLessThan(ttlTiming.tpd);
  });

  it("rise and fall times should be positive", () => {
    const timing = analyzeTiming(new CMOSInverter());
    expect(timing.riseTime).toBeGreaterThan(0);
    expect(timing.fallTime).toBeGreaterThan(0);
  });

  it("max frequency should be positive", () => {
    const timing = analyzeTiming(new CMOSInverter());
    expect(timing.maxFrequency).toBeGreaterThan(0);
  });

  it("CMOSNand should work with analyzeTiming", () => {
    const timing = analyzeTiming(new CMOSNand());
    expect(timing.tpd).toBeGreaterThan(0);
  });

  it("CMOSNor should work with analyzeTiming", () => {
    const timing = analyzeTiming(new CMOSNor());
    expect(timing.tpd).toBeGreaterThan(0);
  });
});

describe("Comparison Utilities", () => {
  it("compare_cmos_vs_ttl should return both CMOS and TTL data", () => {
    const result = compareCmosVsTtl();
    expect(result).toHaveProperty("cmos");
    expect(result).toHaveProperty("ttl");
  });

  it("CMOS should have much less static power than TTL", () => {
    const result = compareCmosVsTtl();
    expect(result["cmos"]["static_power_w"]).toBeLessThan(
      result["ttl"]["static_power_w"],
    );
  });

  it("demonstrateCmosScaling should return a list", () => {
    const result = demonstrateCmosScaling();
    expect(Array.isArray(result)).toBe(true);
    expect(result.length).toBeGreaterThan(0);
  });

  it("default should produce 6 technology nodes", () => {
    const result = demonstrateCmosScaling();
    expect(result.length).toBe(6);
  });

  it("custom technology nodes should be respected", () => {
    const result = demonstrateCmosScaling([180e-9, 45e-9]);
    expect(result.length).toBe(2);
  });

  it("supply voltage should decrease with scaling", () => {
    const result = demonstrateCmosScaling();
    expect(result[0]["vdd_v"]).toBeGreaterThan(result[result.length - 1]["vdd_v"]);
  });

  it("scaling results should have expected keys", () => {
    const result = demonstrateCmosScaling([180e-9]);
    const entry = result[0];
    expect(entry).toHaveProperty("node_nm");
    expect(entry).toHaveProperty("vdd_v");
    expect(entry).toHaveProperty("vth_v");
    expect(entry).toHaveProperty("propagation_delay_s");
    expect(entry).toHaveProperty("dynamic_power_w");
    expect(entry).toHaveProperty("leakage_current_a");
  });
});
