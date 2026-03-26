/**
 * Tests for CLB (Configurable Logic Block).
 */

import { describe, it, expect } from "vitest";
import { CLB } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

describe("CLB", () => {
  const andTt = (): Bit[] => {
    const tt = Array(16).fill(0) as Bit[];
    tt[3] = 1;
    return tt;
  };

  const xorTt = (): Bit[] => {
    const tt = Array(16).fill(0) as Bit[];
    tt[1] = 1;
    tt[2] = 1;
    return tt;
  };

  it("creates with default k=4", () => {
    const clb = new CLB();
    expect(clb.k).toBe(4);
  });

  it("slice0 and slice1 are independent", () => {
    const clb = new CLB(4);
    clb.slice0.configure(andTt(), xorTt());
    clb.slice1.configure(xorTt(), andTt());

    const out = clb.evaluate(
      [1, 1, 0, 0], [1, 0, 0, 0], // slice 0
      [1, 0, 0, 0], [1, 1, 0, 0], // slice 1
      0,
    );

    // Slice 0: AND(1,1)=1, XOR(1,0)=1
    expect(out.slice0.outputA).toBe(1);
    expect(out.slice0.outputB).toBe(1);

    // Slice 1: XOR(1,0)=1, AND(1,1)=1
    expect(out.slice1.outputA).toBe(1);
    expect(out.slice1.outputB).toBe(1);
  });

  it("carry chain flows from slice0 to slice1", () => {
    const clb = new CLB(4);
    clb.slice0.configure(andTt(), xorTt(), false, false, true);
    clb.slice1.configure(andTt(), xorTt(), false, false, true);

    // Slice 0: AND(1,1)=1, XOR(1,0)=1
    // carry_out_0 = (1 AND 1) OR (0 AND (1 XOR 1)) = 1
    const out = clb.evaluate(
      [1, 1, 0, 0], [1, 0, 0, 0],
      [0, 0, 0, 0], [1, 0, 0, 0],
      0,
      0,
    );

    expect(out.slice0.carryOut).toBe(1);
    // Slice 1 gets carry_in=1 from slice 0
    // AND(0,0)=0, XOR(1,0)=1
    // carry_out_1 = (0 AND 1) OR (1 AND (0 XOR 1)) = 0 OR 1 = 1
    expect(out.slice1.carryOut).toBe(1);
  });

  it("external carry_in feeds into slice0", () => {
    const clb = new CLB(4);
    clb.slice0.configure(andTt(), xorTt(), false, false, true);

    const out = clb.evaluate(
      [0, 0, 0, 0], [1, 0, 0, 0],
      [0, 0, 0, 0], [0, 0, 0, 0],
      0,
      1, // carry_in
    );

    // AND(0,0)=0, XOR(1,0)=1
    // carry_out = (0 AND 1) OR (1 AND (0 XOR 1)) = 0 OR 1 = 1
    expect(out.slice0.carryOut).toBe(1);
  });

  it("all-zero inputs and LUTs produce all-zero outputs", () => {
    const clb = new CLB(4);
    const zeros = Array(16).fill(0) as Bit[];
    clb.slice0.configure(zeros, zeros);
    clb.slice1.configure(zeros, zeros);

    const out = clb.evaluate(
      [0, 0, 0, 0], [0, 0, 0, 0],
      [0, 0, 0, 0], [0, 0, 0, 0],
      0,
    );

    expect(out.slice0.outputA).toBe(0);
    expect(out.slice0.outputB).toBe(0);
    expect(out.slice1.outputA).toBe(0);
    expect(out.slice1.outputB).toBe(0);
  });
});
