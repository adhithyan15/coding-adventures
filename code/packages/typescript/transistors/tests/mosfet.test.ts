/**
 * Tests for MOSFET transistors (NMOS and PMOS).
 */

import { describe, it, expect } from "vitest";
import { NMOS, PMOS } from "../src/mosfet.js";
import { MOSFETRegion } from "../src/types.js";

describe("NMOS", () => {
  it("should be in cutoff when Vgs below threshold", () => {
    const t = new NMOS();
    expect(t.region(0.0, 1.0)).toBe(MOSFETRegion.CUTOFF);
    expect(t.drainCurrent(0.0, 1.0)).toBe(0.0);
    expect(t.isConducting(0.0)).toBe(false);
  });

  it("should be in cutoff with negative Vgs", () => {
    const t = new NMOS();
    expect(t.region(-1.0, 0.0)).toBe(MOSFETRegion.CUTOFF);
    expect(t.drainCurrent(-1.0, 0.0)).toBe(0.0);
  });

  it("should be in linear region with Vgs above threshold and low Vds", () => {
    const t = new NMOS();
    expect(t.region(1.5, 0.1)).toBe(MOSFETRegion.LINEAR);
    const ids = t.drainCurrent(1.5, 0.1);
    expect(ids).toBeGreaterThan(0);
  });

  it("should be in saturation region with Vgs above threshold and high Vds", () => {
    const t = new NMOS();
    expect(t.region(1.0, 3.0)).toBe(MOSFETRegion.SATURATION);
    const ids = t.drainCurrent(1.0, 3.0);
    expect(ids).toBeGreaterThan(0);
  });

  it("saturation current should be independent of Vds", () => {
    const t = new NMOS();
    const ids1 = t.drainCurrent(1.5, 3.0);
    const ids2 = t.drainCurrent(1.5, 5.0);
    expect(Math.abs(ids1 - ids2)).toBeLessThan(1e-10);
  });

  it("linear current should increase with Vds", () => {
    const t = new NMOS();
    const idsLow = t.drainCurrent(3.0, 0.1);
    const idsHigh = t.drainCurrent(3.0, 0.5);
    expect(idsHigh).toBeGreaterThan(idsLow);
  });

  it("isConducting should be true when Vgs >= Vth", () => {
    const t = new NMOS();
    expect(t.isConducting(0.3)).toBe(false); // Below default Vth=0.4
    expect(t.isConducting(0.4)).toBe(true); // At Vth
    expect(t.isConducting(1.0)).toBe(true); // Above Vth
  });

  it("output voltage should be 0 when ON", () => {
    const t = new NMOS();
    expect(t.outputVoltage(3.3, 3.3)).toBe(0.0);
  });

  it("output voltage should be Vdd when OFF", () => {
    const t = new NMOS();
    expect(t.outputVoltage(0.0, 3.3)).toBe(3.3);
  });

  it("should respect custom params", () => {
    const t = new NMOS({ vth: 0.7, k: 0.002 });
    expect(t.isConducting(0.5)).toBe(false); // Below custom Vth
    expect(t.isConducting(0.7)).toBe(true); // At custom Vth
  });

  it("transconductance should be 0 in cutoff", () => {
    const t = new NMOS();
    expect(t.transconductance(0.0, 1.0)).toBe(0.0);
  });

  it("transconductance should be positive in saturation", () => {
    const t = new NMOS();
    const gm = t.transconductance(1.5, 3.0);
    expect(gm).toBeGreaterThan(0);
  });

  it("just above Vth with small Vds should be in linear", () => {
    const t = new NMOS();
    expect(t.region(0.5, 0.01)).toBe(MOSFETRegion.LINEAR);
  });

  it("at Vds = Vgs - Vth should enter saturation", () => {
    const t = new NMOS();
    const vgs = 1.0;
    const vds = vgs - 0.4; // Exactly at boundary
    expect(t.region(vgs, vds)).toBe(MOSFETRegion.SATURATION);
  });
});

describe("PMOS", () => {
  it("should be in cutoff when Vgs is zero", () => {
    const t = new PMOS();
    expect(t.region(0.0, 0.0)).toBe(MOSFETRegion.CUTOFF);
    expect(t.isConducting(0.0)).toBe(false);
  });

  it("should conduct when Vgs is sufficiently negative", () => {
    const t = new PMOS();
    expect(t.isConducting(-1.5)).toBe(true);
    expect(t.region(-1.5, -3.0)).toBe(MOSFETRegion.SATURATION);
  });

  it("should be in linear region with small |Vds|", () => {
    const t = new PMOS();
    expect(t.region(-1.5, -0.1)).toBe(MOSFETRegion.LINEAR);
  });

  it("drain current magnitude should be positive when conducting", () => {
    const t = new PMOS();
    const ids = t.drainCurrent(-1.5, -3.0);
    expect(ids).toBeGreaterThan(0);
  });

  it("should have zero current in cutoff", () => {
    const t = new PMOS();
    expect(t.drainCurrent(0.0, -1.0)).toBe(0.0);
  });

  it("output voltage should be Vdd when ON", () => {
    const t = new PMOS();
    expect(t.outputVoltage(-3.3, 3.3)).toBe(3.3);
  });

  it("output voltage should be 0 when OFF", () => {
    const t = new PMOS();
    expect(t.outputVoltage(0.0, 3.3)).toBe(0.0);
  });

  it("should be complementary to NMOS", () => {
    const nmos = new NMOS();
    const pmos = new PMOS();
    const vdd = 3.3;

    // Input HIGH: NMOS ON, PMOS OFF
    expect(nmos.isConducting(vdd)).toBe(true);
    expect(pmos.isConducting(0.0)).toBe(false);

    // Input LOW: NMOS OFF, PMOS ON
    expect(nmos.isConducting(0.0)).toBe(false);
    expect(pmos.isConducting(-vdd)).toBe(true);
  });

  it("transconductance should be 0 in cutoff", () => {
    const t = new PMOS();
    expect(t.transconductance(0.0, 0.0)).toBe(0.0);
  });

  it("transconductance should be positive when conducting", () => {
    const t = new PMOS();
    const gm = t.transconductance(-1.5, -3.0);
    expect(gm).toBeGreaterThan(0);
  });
});
