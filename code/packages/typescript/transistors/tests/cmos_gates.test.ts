/**
 * Tests for CMOS logic gates built from transistors.
 */

import { describe, it, expect } from "vitest";
import {
  CMOSInverter,
  CMOSNand,
  CMOSNor,
  CMOSAnd,
  CMOSOr,
  CMOSXor,
  CMOSXnor,
} from "../src/cmos_gates.js";

describe("CMOSInverter", () => {
  it("should implement NOT truth table", () => {
    const inv = new CMOSInverter();
    expect(inv.evaluateDigital(0)).toBe(1);
    expect(inv.evaluateDigital(1)).toBe(0);
  });

  it("input HIGH should produce output near GND", () => {
    const inv = new CMOSInverter({ vdd: 3.3 });
    const result = inv.evaluate(3.3);
    expect(result.voltage).toBeLessThan(0.1);
  });

  it("input LOW should produce output near Vdd", () => {
    const inv = new CMOSInverter({ vdd: 3.3 });
    const result = inv.evaluate(0.0);
    expect(result.voltage).toBeGreaterThan(3.2);
  });

  it("should have near-zero static power", () => {
    const inv = new CMOSInverter();
    expect(inv.staticPower).toBeLessThan(1e-9);
  });

  it("should have positive dynamic power", () => {
    const inv = new CMOSInverter({ vdd: 3.3 });
    const p = inv.dynamicPower(1e9, 1e-12);
    expect(p).toBeGreaterThan(0);
  });

  it("halving Vdd should reduce dynamic power by ~4x", () => {
    const invHigh = new CMOSInverter({ vdd: 3.3 });
    const invLow = new CMOSInverter({ vdd: 1.65 });
    const pHigh = invHigh.dynamicPower(1e9, 1e-12);
    const pLow = invLow.dynamicPower(1e9, 1e-12);
    const ratio = pHigh / pLow;
    expect(ratio).toBeGreaterThan(3.5);
    expect(ratio).toBeLessThan(4.5);
  });

  it("VTC should show sharp transition", () => {
    const inv = new CMOSInverter({ vdd: 3.3 });
    const vtc = inv.voltageTranferCharacteristic(10);
    expect(vtc.length).toBe(11);
    // First point: input=0, output should be HIGH
    expect(vtc[0][1]).toBeGreaterThan(3.0);
    // Last point: input=Vdd, output should be LOW
    expect(vtc[vtc.length - 1][1]).toBeLessThan(0.5);
  });

  it("should reject invalid digital input", () => {
    const inv = new CMOSInverter();
    expect(() => inv.evaluateDigital(2)).toThrow();
    expect(() => inv.evaluateDigital(true as unknown as number)).toThrow();
  });

  it("should report transistor count of 2", () => {
    const inv = new CMOSInverter();
    const result = inv.evaluate(0.0);
    expect(result.transistorCount).toBe(2);
  });
});

describe("CMOSNand", () => {
  it("should implement NAND truth table", () => {
    const nand = new CMOSNand();
    expect(nand.evaluateDigital(0, 0)).toBe(1);
    expect(nand.evaluateDigital(0, 1)).toBe(1);
    expect(nand.evaluateDigital(1, 0)).toBe(1);
    expect(nand.evaluateDigital(1, 1)).toBe(0);
  });

  it("should have transistor count of 4", () => {
    const nand = new CMOSNand();
    expect(nand.transistorCount).toBe(4);
  });

  it("should output HIGH voltage for (0,0)", () => {
    const nand = new CMOSNand({ vdd: 3.3 });
    const result = nand.evaluate(0.0, 0.0);
    expect(result.voltage).toBeGreaterThan(3.0);
  });

  it("should output LOW voltage for (1,1)", () => {
    const nand = new CMOSNand({ vdd: 3.3 });
    const result = nand.evaluate(3.3, 3.3);
    expect(result.voltage).toBeLessThan(0.5);
  });

  it("should reject invalid digital input", () => {
    const nand = new CMOSNand();
    expect(() => nand.evaluateDigital(2, 0)).toThrow();
  });
});

describe("CMOSNor", () => {
  it("should implement NOR truth table", () => {
    const nor = new CMOSNor();
    expect(nor.evaluateDigital(0, 0)).toBe(1);
    expect(nor.evaluateDigital(0, 1)).toBe(0);
    expect(nor.evaluateDigital(1, 0)).toBe(0);
    expect(nor.evaluateDigital(1, 1)).toBe(0);
  });

  it("should reject invalid digital input", () => {
    const nor = new CMOSNor();
    expect(() => nor.evaluateDigital(0, 2)).toThrow();
  });
});

describe("CMOSAnd", () => {
  it("should implement AND truth table", () => {
    const andGate = new CMOSAnd();
    expect(andGate.evaluateDigital(0, 0)).toBe(0);
    expect(andGate.evaluateDigital(0, 1)).toBe(0);
    expect(andGate.evaluateDigital(1, 0)).toBe(0);
    expect(andGate.evaluateDigital(1, 1)).toBe(1);
  });

  it("should reject invalid digital input", () => {
    const andGate = new CMOSAnd();
    expect(() => andGate.evaluateDigital(true as unknown as number, 0)).toThrow();
  });
});

describe("CMOSOr", () => {
  it("should implement OR truth table", () => {
    const orGate = new CMOSOr();
    expect(orGate.evaluateDigital(0, 0)).toBe(0);
    expect(orGate.evaluateDigital(0, 1)).toBe(1);
    expect(orGate.evaluateDigital(1, 0)).toBe(1);
    expect(orGate.evaluateDigital(1, 1)).toBe(1);
  });

  it("should reject invalid digital input", () => {
    const orGate = new CMOSOr();
    expect(() => orGate.evaluateDigital(-1, 0)).toThrow();
  });
});

describe("CMOSXor", () => {
  it("should implement XOR truth table", () => {
    const xorGate = new CMOSXor();
    expect(xorGate.evaluateDigital(0, 0)).toBe(0);
    expect(xorGate.evaluateDigital(0, 1)).toBe(1);
    expect(xorGate.evaluateDigital(1, 0)).toBe(1);
    expect(xorGate.evaluateDigital(1, 1)).toBe(0);
  });

  it("NAND-based XOR should match direct XOR", () => {
    const xorGate = new CMOSXor();
    for (const a of [0, 1]) {
      for (const b of [0, 1]) {
        expect(xorGate.evaluateFromNands(a, b)).toBe(
          xorGate.evaluateDigital(a, b),
        );
      }
    }
  });

  it("should reject invalid digital input", () => {
    const xorGate = new CMOSXor();
    expect(() => xorGate.evaluateDigital(0, 2)).toThrow();
  });
});

describe("CMOSXnor", () => {
  it("should implement XNOR truth table", () => {
    const xnorGate = new CMOSXnor();
    expect(xnorGate.evaluateDigital(0, 0)).toBe(1);
    expect(xnorGate.evaluateDigital(0, 1)).toBe(0);
    expect(xnorGate.evaluateDigital(1, 0)).toBe(0);
    expect(xnorGate.evaluateDigital(1, 1)).toBe(1);
  });

  it("should report transistor count of 8", () => {
    const xnorGate = new CMOSXnor();
    const result = xnorGate.evaluate(0.0, 0.0);
    expect(result.transistorCount).toBe(8);
  });

  it("should reject invalid digital input", () => {
    const xnorGate = new CMOSXnor();
    expect(() => xnorGate.evaluateDigital(2, 0)).toThrow();
    expect(() => xnorGate.evaluateDigital(0, -1)).toThrow();
  });
});
