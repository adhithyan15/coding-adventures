/**
 * Tests for analog amplifier analysis.
 */

import { describe, it, expect } from "vitest";
import {
  analyzeCommonSourceAmp,
  analyzeCommonEmitterAmp,
} from "../src/amplifier.js";
import { NPN } from "../src/bjt.js";
import { NMOS } from "../src/mosfet.js";

describe("Common-Source Amplifier", () => {
  it("should have negative (inverting) voltage gain", () => {
    const t = new NMOS();
    const result = analyzeCommonSourceAmp(t, 1.5, 3.3, 10_000);
    expect(result.voltageGain).toBeLessThan(0);
  });

  it("should have very high input impedance", () => {
    const t = new NMOS();
    const result = analyzeCommonSourceAmp(t, 1.5, 3.3, 10_000);
    expect(result.inputImpedance).toBeGreaterThan(1e9);
  });

  it("should have positive transconductance", () => {
    const t = new NMOS();
    const result = analyzeCommonSourceAmp(t, 1.5, 3.3, 10_000);
    expect(result.transconductance).toBeGreaterThan(0);
  });

  it("should have positive bandwidth", () => {
    const t = new NMOS();
    const result = analyzeCommonSourceAmp(t, 1.5, 3.3, 10_000);
    expect(result.bandwidth).toBeGreaterThan(0);
  });

  it("operating point should contain required keys", () => {
    const t = new NMOS();
    const result = analyzeCommonSourceAmp(t, 1.5, 3.3, 10_000);
    expect(result.operatingPoint).toHaveProperty("vgs");
    expect(result.operatingPoint).toHaveProperty("vds");
    expect(result.operatingPoint).toHaveProperty("ids");
    expect(result.operatingPoint).toHaveProperty("gm");
  });

  it("higher Rd should give more voltage gain", () => {
    const t = new NMOS();
    const r1 = analyzeCommonSourceAmp(t, 1.5, 3.3, 5_000);
    const r2 = analyzeCommonSourceAmp(t, 1.5, 3.3, 20_000);
    expect(Math.abs(r2.voltageGain)).toBeGreaterThan(Math.abs(r1.voltageGain));
  });
});

describe("Common-Emitter Amplifier", () => {
  it("should have negative (inverting) voltage gain", () => {
    const t = new NPN();
    const result = analyzeCommonEmitterAmp(t, 0.7, 5.0, 4700);
    expect(result.voltageGain).toBeLessThan(0);
  });

  it("should have moderate input impedance", () => {
    const t = new NPN();
    const result = analyzeCommonEmitterAmp(t, 0.7, 5.0, 4700);
    expect(result.inputImpedance).toBeGreaterThan(100);
    expect(result.inputImpedance).toBeLessThan(1e6);
  });

  it("should have positive transconductance", () => {
    const t = new NPN();
    const result = analyzeCommonEmitterAmp(t, 0.7, 5.0, 4700);
    expect(result.transconductance).toBeGreaterThan(0);
  });

  it("higher beta should give higher input impedance", () => {
    const tLow = new NPN({ beta: 50 });
    const tHigh = new NPN({ beta: 200 });
    const r1 = analyzeCommonEmitterAmp(tLow, 0.7, 5.0, 4700);
    const r2 = analyzeCommonEmitterAmp(tHigh, 0.7, 5.0, 4700);
    expect(r2.inputImpedance).toBeGreaterThan(r1.inputImpedance);
  });

  it("operating point should contain required keys", () => {
    const t = new NPN();
    const result = analyzeCommonEmitterAmp(t, 0.7, 5.0, 4700);
    expect(result.operatingPoint).toHaveProperty("vbe");
    expect(result.operatingPoint).toHaveProperty("vce");
    expect(result.operatingPoint).toHaveProperty("ic");
    expect(result.operatingPoint).toHaveProperty("ib");
  });
});
