/**
 * Tests for Slice.
 */

import { describe, it, expect } from "vitest";
import { Slice } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

describe("Slice", () => {
  // Helper: create an AND truth table for 4-input LUT
  const andTt = (): Bit[] => {
    const tt = Array(16).fill(0) as Bit[];
    tt[3] = 1; // I0=1, I1=1
    return tt;
  };

  // Helper: create an XOR truth table for 4-input LUT
  const xorTt = (): Bit[] => {
    const tt = Array(16).fill(0) as Bit[];
    tt[1] = 1; // I0=1, I1=0
    tt[2] = 1; // I0=0, I1=1
    return tt;
  };

  it("combinational output (no flip-flops)", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt());
    const out = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 0);
    expect(out.outputA).toBe(1); // AND(1,1) = 1
    expect(out.outputB).toBe(1); // XOR(1,0) = 1
  });

  it("combinational output respects LUT tables", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt());
    const out = s.evaluate([1, 0, 0, 0], [1, 1, 0, 0], 0);
    expect(out.outputA).toBe(0); // AND(1,0) = 0
    expect(out.outputB).toBe(0); // XOR(1,1) = 0
  });

  it("carry chain computes full-adder carry equation", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt(), false, false, true);
    // carry_out = (A AND B) OR (carry_in AND (A XOR B))
    // LUT_A=AND(1,1)=1, LUT_B=XOR(1,0)=1
    // carry_out = (1 AND 1) OR (0 AND (1 XOR 1)) = 1 OR 0 = 1
    const out = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 0, 0);
    expect(out.carryOut).toBe(1);
  });

  it("carry chain disabled returns 0", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt(), false, false, false);
    const out = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 0);
    expect(out.carryOut).toBe(0);
  });

  it("carry chain with carry_in=1", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt(), false, false, true);
    // LUT_A=AND(0,0)=0, LUT_B=XOR(1,0)=1
    // carry_out = (0 AND 1) OR (1 AND (0 XOR 1)) = 0 OR 1 = 1
    const out = s.evaluate([0, 0, 0, 0], [1, 0, 0, 0], 0, 1);
    expect(out.carryOut).toBe(1);
  });

  it("with flip-flop A enabled, output changes on clock", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt(), true, false, false);

    // Clock low: master absorbs data
    const out0 = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 0);
    // Clock high: slave outputs
    const out1 = s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 1);

    // With FF enabled, output goes through MUX selecting registered value
    // After one full cycle, FF should capture the LUT output
    expect(out1.outputA).toBe(1); // AND(1,1) registered
    expect(out1.outputB).toBe(1); // XOR not registered, still combinational
  });

  it("k property reflects LUT input count", () => {
    const s = new Slice(3);
    expect(s.k).toBe(3);
  });

  it("lutA and lutB properties return LUT instances", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt());
    expect(s.lutA.k).toBe(4);
    expect(s.lutB.k).toBe(4);
    expect(s.lutA.truthTable[3]).toBe(1);
    expect(s.lutB.truthTable[1]).toBe(1);
  });

  it("reconfigure resets flip-flop state", () => {
    const s = new Slice(4);
    s.configure(andTt(), xorTt(), true, false, false);
    s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 0);
    s.evaluate([1, 1, 0, 0], [1, 0, 0, 0], 1);

    // Reconfigure -- should reset FF state
    const allZeros = Array(16).fill(0) as Bit[];
    s.configure(allZeros, allZeros, true, false, false);
    const out = s.evaluate([0, 0, 0, 0], [0, 0, 0, 0], 1);
    expect(out.outputA).toBe(0);
  });
});
