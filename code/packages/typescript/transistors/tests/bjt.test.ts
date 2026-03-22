/**
 * Tests for BJT transistors (NPN and PNP).
 */

import { describe, it, expect } from "vitest";
import { NPN, PNP } from "../src/bjt.js";
import { BJTRegion } from "../src/types.js";

describe("NPN", () => {
  it("should be in cutoff when Vbe below threshold", () => {
    const t = new NPN();
    expect(t.region(0.0, 5.0)).toBe(BJTRegion.CUTOFF);
    expect(t.collectorCurrent(0.0, 5.0)).toBe(0.0);
    expect(t.isConducting(0.0)).toBe(false);
  });

  it("should be in active region with Vbe at threshold and Vce > Vce_sat", () => {
    const t = new NPN();
    expect(t.region(0.7, 3.0)).toBe(BJTRegion.ACTIVE);
    const ic = t.collectorCurrent(0.7, 3.0);
    expect(ic).toBeGreaterThan(0);
  });

  it("should be in saturation with Vbe at threshold and Vce <= Vce_sat", () => {
    const t = new NPN();
    expect(t.region(0.7, 0.1)).toBe(BJTRegion.SATURATION);
  });

  it("isConducting should be true when Vbe >= Vbe_on", () => {
    const t = new NPN();
    expect(t.isConducting(0.5)).toBe(false);
    expect(t.isConducting(0.7)).toBe(true);
    expect(t.isConducting(1.0)).toBe(true);
  });

  it("current gain should be approximately beta", () => {
    const t = new NPN({ beta: 100 });
    const ic = t.collectorCurrent(0.7, 3.0);
    const ib = t.baseCurrent(0.7, 3.0);
    if (ib > 0) {
      expect(Math.abs(ic / ib - 100.0)).toBeLessThan(1.0);
    }
  });

  it("base current should be 0 in cutoff", () => {
    const t = new NPN();
    expect(t.baseCurrent(0.0, 5.0)).toBe(0.0);
  });

  it("transconductance should be 0 in cutoff", () => {
    const t = new NPN();
    expect(t.transconductance(0.0, 5.0)).toBe(0.0);
  });

  it("transconductance should be positive in active region", () => {
    const t = new NPN();
    const gm = t.transconductance(0.7, 3.0);
    expect(gm).toBeGreaterThan(0);
  });

  it("lower beta should result in more base current", () => {
    const tLow = new NPN({ beta: 50 });
    const tHigh = new NPN({ beta: 200 });
    const ibLow = tLow.baseCurrent(0.7, 3.0);
    const ibHigh = tHigh.baseCurrent(0.7, 3.0);
    expect(ibLow).toBeGreaterThan(ibHigh);
  });

  it("at Vce = Vce_sat should be in saturation", () => {
    const t = new NPN();
    expect(t.region(0.7, 0.2)).toBe(BJTRegion.SATURATION);
  });

  it("just above Vce_sat should be in active", () => {
    const t = new NPN();
    expect(t.region(0.7, 0.3)).toBe(BJTRegion.ACTIVE);
  });
});

describe("PNP", () => {
  it("should be in cutoff with small |Vbe|", () => {
    const t = new PNP();
    expect(t.region(0.0, 0.0)).toBe(BJTRegion.CUTOFF);
    expect(t.collectorCurrent(0.0, 0.0)).toBe(0.0);
    expect(t.isConducting(0.0)).toBe(false);
  });

  it("should conduct with negative Vbe", () => {
    const t = new PNP();
    expect(t.isConducting(-0.7)).toBe(true);
    expect(t.region(-0.7, -3.0)).toBe(BJTRegion.ACTIVE);
  });

  it("should be in saturation when |Vce| <= Vce_sat", () => {
    const t = new PNP();
    expect(t.region(-0.7, -0.1)).toBe(BJTRegion.SATURATION);
  });

  it("collector current magnitude should be positive when conducting", () => {
    const t = new PNP();
    const ic = t.collectorCurrent(-0.7, -3.0);
    expect(ic).toBeGreaterThan(0);
  });

  it("base current should be non-zero when conducting", () => {
    const t = new PNP();
    const ib = t.baseCurrent(-0.7, -3.0);
    expect(ib).toBeGreaterThan(0);
  });

  it("base current should be 0 in cutoff", () => {
    const t = new PNP();
    expect(t.baseCurrent(0.0, 0.0)).toBe(0.0);
  });

  it("transconductance should be positive when conducting", () => {
    const t = new PNP();
    const gm = t.transconductance(-0.7, -3.0);
    expect(gm).toBeGreaterThan(0);
  });

  it("transconductance should be 0 in cutoff", () => {
    const t = new PNP();
    expect(t.transconductance(0.0, 0.0)).toBe(0.0);
  });
});
