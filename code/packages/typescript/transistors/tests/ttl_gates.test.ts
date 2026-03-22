/**
 * Tests for TTL logic gates (historical BJT-based).
 */

import { describe, it, expect } from "vitest";
import { TTLNand, RTLInverter } from "../src/ttl_gates.js";

describe("TTLNand", () => {
  it("should implement NAND truth table", () => {
    const nand = new TTLNand();
    expect(nand.evaluateDigital(0, 0)).toBe(1);
    expect(nand.evaluateDigital(0, 1)).toBe(1);
    expect(nand.evaluateDigital(1, 0)).toBe(1);
    expect(nand.evaluateDigital(1, 1)).toBe(0);
  });

  it("should dissipate milliwatts of static power", () => {
    const nand = new TTLNand();
    expect(nand.staticPower).toBeGreaterThan(1e-3);
  });

  it("output LOW should be near Vce_sat", () => {
    const nand = new TTLNand();
    const result = nand.evaluate(5.0, 5.0);
    expect(result.voltage).toBeLessThan(0.5);
    expect(result.logicValue).toBe(0);
  });

  it("output HIGH should be near Vcc - 0.7V", () => {
    const nand = new TTLNand();
    const result = nand.evaluate(0.0, 0.0);
    expect(result.voltage).toBeGreaterThan(3.0);
    expect(result.logicValue).toBe(1);
  });

  it("propagation delay should be in nanosecond range", () => {
    const nand = new TTLNand();
    const result = nand.evaluate(5.0, 5.0);
    expect(result.propagationDelay).toBeGreaterThan(1e-9);
    expect(result.propagationDelay).toBeLessThan(100e-9);
  });

  it("should reject invalid digital input", () => {
    const nand = new TTLNand();
    expect(() => nand.evaluateDigital(2, 0)).toThrow();
  });

  it("should respect custom Vcc", () => {
    const nand = new TTLNand(3.3);
    expect(nand.vcc).toBe(3.3);
  });
});

describe("RTLInverter", () => {
  it("should implement NOT truth table", () => {
    const inv = new RTLInverter();
    expect(inv.evaluateDigital(0)).toBe(1);
    expect(inv.evaluateDigital(1)).toBe(0);
  });

  it("input LOW should produce output near Vcc", () => {
    const inv = new RTLInverter();
    const result = inv.evaluate(0.0);
    expect(result.voltage).toBeGreaterThan(4.0);
    expect(result.logicValue).toBe(1);
  });

  it("input HIGH should produce output near GND", () => {
    const inv = new RTLInverter();
    const result = inv.evaluate(5.0);
    expect(result.voltage).toBeLessThan(1.0);
    expect(result.logicValue).toBe(0);
  });

  it("propagation delay should be slower than TTL", () => {
    const inv = new RTLInverter();
    const result = inv.evaluate(5.0);
    expect(result.propagationDelay).toBeGreaterThan(10e-9);
  });

  it("should reject invalid digital input", () => {
    const inv = new RTLInverter();
    expect(() => inv.evaluateDigital(true as unknown as number)).toThrow();
  });

  it("should respect custom resistor values", () => {
    const inv = new RTLInverter(5.0, 5000, 2000);
    expect(inv.rBase).toBe(5000);
    expect(inv.rCollector).toBe(2000);
  });
});
