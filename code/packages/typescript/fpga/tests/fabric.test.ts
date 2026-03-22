/**
 * Tests for FPGA fabric.
 */

import { describe, it, expect } from "vitest";
import { FPGA, Bitstream, IOMode } from "../src/index.js";
import type { Bit } from "@coding-adventures/logic-gates";

describe("FPGA", () => {
  // AND gate truth table for 4-input LUT (only I0 AND I1)
  const andTt = (): Bit[] => {
    const tt = Array(16).fill(0) as Bit[];
    tt[3] = 1;
    return tt;
  };

  const zeros16 = (): Bit[] => Array(16).fill(0) as Bit[];

  it("creates from bitstream with CLBs and I/O", () => {
    const bs = Bitstream.fromObject({
      clbs: {
        clb_0: {
          slice0: { lutA: andTt(), lutB: zeros16() },
          slice1: { lutA: zeros16(), lutB: zeros16() },
        },
      },
      io: {
        inA: { mode: "input" },
        inB: { mode: "input" },
        out: { mode: "output" },
      },
    });

    const fpga = new FPGA(bs);
    expect(Object.keys(fpga.clbs)).toEqual(["clb_0"]);
    expect(Object.keys(fpga.ios)).toEqual(["inA", "inB", "out"]);
  });

  it("evaluateCLB returns correct output", () => {
    const bs = Bitstream.fromObject({
      clbs: {
        clb_0: {
          slice0: { lutA: andTt(), lutB: zeros16() },
          slice1: { lutA: zeros16(), lutB: zeros16() },
        },
      },
    });

    const fpga = new FPGA(bs);
    const out = fpga.evaluateCLB(
      "clb_0",
      [1, 1, 0, 0], [0, 0, 0, 0],
      [0, 0, 0, 0], [0, 0, 0, 0],
      0,
    );

    expect(out.slice0.outputA).toBe(1); // AND(1,1) = 1
    expect(out.slice0.outputB).toBe(0);
  });

  it("evaluateCLB throws for unknown CLB", () => {
    const bs = Bitstream.fromObject({});
    const fpga = new FPGA(bs);
    expect(() =>
      fpga.evaluateCLB("nonexistent", [0,0,0,0], [0,0,0,0], [0,0,0,0], [0,0,0,0], 0),
    ).toThrow("not found");
  });

  it("setInput and readOutput work with I/O blocks", () => {
    const bs = Bitstream.fromObject({
      io: {
        inPin: { mode: "input" },
        outPin: { mode: "output" },
      },
    });

    const fpga = new FPGA(bs);
    fpga.setInput("inPin", 1);
    expect(fpga.readOutput("inPin")).toBe(1);
  });

  it("driveOutput and readOutput for output pin", () => {
    const bs = Bitstream.fromObject({
      io: { outPin: { mode: "output" } },
    });

    const fpga = new FPGA(bs);
    fpga.driveOutput("outPin", 1);
    expect(fpga.readOutput("outPin")).toBe(1);
  });

  it("tristate pin returns null", () => {
    const bs = Bitstream.fromObject({
      io: { triPin: { mode: "tristate" } },
    });

    const fpga = new FPGA(bs);
    expect(fpga.readOutput("triPin")).toBeNull();
  });

  it("route sends signals through switch matrix", () => {
    const bs = Bitstream.fromObject({
      routing: {
        sw_0: [
          { src: "clbOut", dst: "east" },
          { src: "north", dst: "south" },
        ],
      },
    });

    const fpga = new FPGA(bs);
    const result = fpga.route("sw_0", { clbOut: 1, north: 0 });
    expect(result).toEqual({ east: 1, south: 0 });
  });

  it("route throws for unknown switch matrix", () => {
    const bs = Bitstream.fromObject({});
    const fpga = new FPGA(bs);
    expect(() => fpga.route("nonexistent", {})).toThrow("not found");
  });

  it("setInput throws for unknown pin", () => {
    const bs = Bitstream.fromObject({});
    const fpga = new FPGA(bs);
    expect(() => fpga.setInput("nonexistent", 0)).toThrow("not found");
  });

  it("readOutput throws for unknown pin", () => {
    const bs = Bitstream.fromObject({});
    const fpga = new FPGA(bs);
    expect(() => fpga.readOutput("nonexistent")).toThrow("not found");
  });

  it("driveOutput throws for unknown pin", () => {
    const bs = Bitstream.fromObject({});
    const fpga = new FPGA(bs);
    expect(() => fpga.driveOutput("nonexistent", 0)).toThrow("not found");
  });

  it("bitstream property returns original bitstream", () => {
    const bs = new Bitstream();
    const fpga = new FPGA(bs);
    expect(fpga.bitstream).toBe(bs);
  });

  it("switches property returns switch matrices", () => {
    const bs = Bitstream.fromObject({
      routing: {
        sw_0: [{ src: "a", dst: "b" }],
      },
    });
    const fpga = new FPGA(bs);
    expect(Object.keys(fpga.switches)).toEqual(["sw_0"]);
  });

  it("end-to-end: configure AND gate and evaluate", () => {
    const bs = Bitstream.fromObject({
      clbs: {
        clb_0: {
          slice0: { lutA: andTt() },
          slice1: {},
        },
      },
      io: {
        a: { mode: "input" },
        b: { mode: "input" },
        y: { mode: "output" },
      },
    });

    const fpga = new FPGA(bs);

    // Set inputs
    fpga.setInput("a", 1);
    fpga.setInput("b", 1);

    // Evaluate CLB
    const out = fpga.evaluateCLB(
      "clb_0",
      [1, 1, 0, 0], [0, 0, 0, 0],
      [0, 0, 0, 0], [0, 0, 0, 0],
      0,
    );

    // Drive output
    fpga.driveOutput("y", out.slice0.outputA);
    expect(fpga.readOutput("y")).toBe(1);
  });
});
