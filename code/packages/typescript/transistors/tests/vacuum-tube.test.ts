/**
 * Tests for the vacuum tube (triode) model.
 *
 * These tests verify the Child-Langmuir equation implementation
 * against known physical behavior of triode vacuum tubes.
 */

import { describe, it, expect } from "vitest";
import {
  triodePlateCurrent,
  isConducting,
  defaultTriodeParams,
} from "../src/index.js";

describe("Vacuum Tube Model", () => {
  // -----------------------------------------------------------------------
  // Cutoff behavior
  // -----------------------------------------------------------------------

  it("produces zero current at cutoff voltage", () => {
    // Cutoff occurs when Vg + Vp/mu <= 0
    // With defaults: Vp/mu = 250/20 = 12.5
    // So cutoff at Vg = -12.5
    const current = triodePlateCurrent(-12.5);
    expect(current).toBe(0);
  });

  it("produces zero current for large negative grid voltage", () => {
    const current = triodePlateCurrent(-50);
    expect(current).toBe(0);
  });

  // -----------------------------------------------------------------------
  // Conducting behavior
  // -----------------------------------------------------------------------

  it("produces positive current for positive grid voltage", () => {
    const current = triodePlateCurrent(5);
    expect(current).toBeGreaterThan(0);
  });

  it("produces positive current for slightly above cutoff", () => {
    const current = triodePlateCurrent(-12);
    expect(current).toBeGreaterThan(0);
  });

  // -----------------------------------------------------------------------
  // Monotonicity: current increases with grid voltage
  // -----------------------------------------------------------------------

  it("current increases monotonically with grid voltage", () => {
    const voltages = [-10, -5, 0, 2, 5, 10];
    let prevCurrent = -1;

    for (const v of voltages) {
      const current = triodePlateCurrent(v);
      expect(current).toBeGreaterThan(prevCurrent);
      prevCurrent = current;
    }
  });

  // -----------------------------------------------------------------------
  // Default parameters produce reasonable values
  // -----------------------------------------------------------------------

  it("default params produce milliamp-range current at Vg=0", () => {
    const params = defaultTriodeParams();
    const current = triodePlateCurrent(0, params);

    // With Vg=0: effectiveV = 250/20 = 12.5
    // Ip = 0.001 * 12.5^1.5 = 0.001 * 44.19 = 0.04419 A
    expect(current).toBeGreaterThan(0.01); // More than 10mA
    expect(current).toBeLessThan(0.1);     // Less than 100mA
  });

  // -----------------------------------------------------------------------
  // isConducting helper
  // -----------------------------------------------------------------------

  it("isConducting returns false below cutoff", () => {
    expect(isConducting(-15)).toBe(false);
  });

  it("isConducting returns true above cutoff", () => {
    expect(isConducting(0)).toBe(true);
  });

  it("isConducting returns false at exact cutoff", () => {
    // At Vg = -12.5 (exact cutoff), effectiveV = 0 -> current = 0
    expect(isConducting(-12.5)).toBe(false);
  });

  // -----------------------------------------------------------------------
  // Custom parameters
  // -----------------------------------------------------------------------

  it("respects custom mu parameter", () => {
    // Higher mu means the grid has more control
    const highMu = triodePlateCurrent(-5, { mu: 100 });
    const lowMu = triodePlateCurrent(-5, { mu: 5 });

    // With high mu, Vp/mu is smaller so cutoff happens sooner
    // At Vg=-5: high mu effectiveV = -5 + 250/100 = -2.5 (cutoff!)
    expect(highMu).toBe(0);
    // At Vg=-5: low mu effectiveV = -5 + 250/5 = 45 (conducting)
    expect(lowMu).toBeGreaterThan(0);
  });

  it("respects custom K parameter", () => {
    const highK = triodePlateCurrent(0, { K: 0.01 });
    const lowK = triodePlateCurrent(0, { K: 0.0001 });
    expect(highK).toBeGreaterThan(lowK);
    // K scales linearly
    expect(highK / lowK).toBeCloseTo(100, 0);
  });
});
