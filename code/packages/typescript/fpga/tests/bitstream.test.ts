/**
 * Tests for Bitstream.
 */

import { describe, it, expect } from "vitest";
import { Bitstream } from "../src/index.js";

describe("Bitstream", () => {
  it("creates empty bitstream with defaults", () => {
    const bs = new Bitstream();
    expect(bs.lutK).toBe(4);
    expect(Object.keys(bs.clbs)).toHaveLength(0);
    expect(Object.keys(bs.routing)).toHaveLength(0);
    expect(Object.keys(bs.io)).toHaveLength(0);
  });

  it("fromObject parses CLBs", () => {
    const data = {
      clbs: {
        clb_0: {
          slice0: {
            lutA: Array(16).fill(0),
            lutB: Array(16).fill(0),
            ffA: true,
            ffB: false,
            carry: true,
          },
          slice1: {
            lutA: Array(16).fill(0),
            lutB: Array(16).fill(0),
          },
        },
      },
    };

    const bs = Bitstream.fromObject(data);
    expect(Object.keys(bs.clbs)).toEqual(["clb_0"]);
    expect(bs.clbs.clb_0.slice0.ffAEnabled).toBe(true);
    expect(bs.clbs.clb_0.slice0.ffBEnabled).toBe(false);
    expect(bs.clbs.clb_0.slice0.carryEnabled).toBe(true);
    expect(bs.clbs.clb_0.slice1.ffAEnabled).toBe(false);
  });

  it("fromObject parses routing", () => {
    const data = {
      routing: {
        sw_0: [
          { src: "clbOut", dst: "east" },
          { src: "north", dst: "south" },
        ],
      },
    };

    const bs = Bitstream.fromObject(data);
    expect(bs.routing.sw_0).toHaveLength(2);
    expect(bs.routing.sw_0[0].source).toBe("clbOut");
    expect(bs.routing.sw_0[0].destination).toBe("east");
  });

  it("fromObject parses I/O", () => {
    const data = {
      io: {
        pinA: { mode: "input" },
        pinB: { mode: "output" },
        pinC: { mode: "tristate" },
      },
    };

    const bs = Bitstream.fromObject(data);
    expect(bs.io.pinA.mode).toBe("input");
    expect(bs.io.pinB.mode).toBe("output");
    expect(bs.io.pinC.mode).toBe("tristate");
  });

  it("fromObject defaults missing fields", () => {
    const data = {
      clbs: {
        clb_0: {
          slice0: {},
          slice1: {},
        },
      },
    };

    const bs = Bitstream.fromObject(data);
    expect(bs.clbs.clb_0.slice0.lutA).toHaveLength(16);
    expect(bs.clbs.clb_0.slice0.ffAEnabled).toBe(false);
    expect(bs.clbs.clb_0.slice0.carryEnabled).toBe(false);
  });

  it("fromObject respects custom lutK", () => {
    const data = {
      lutK: 3,
      clbs: {
        clb_0: {
          slice0: {},
          slice1: {},
        },
      },
    };

    const bs = Bitstream.fromObject(data);
    expect(bs.lutK).toBe(3);
    expect(bs.clbs.clb_0.slice0.lutA).toHaveLength(8); // 2^3
  });

  it("fromJSON parses JSON string", () => {
    const json = JSON.stringify({
      io: { pin0: { mode: "input" } },
    });

    const bs = Bitstream.fromJSON(json);
    expect(bs.io.pin0.mode).toBe("input");
  });

  it("fromObject with snake_case keys (Python compat)", () => {
    const data = {
      lut_k: 4,
      clbs: {
        clb_0: {
          slice0: {
            lut_a: Array(16).fill(0),
            lut_b: Array(16).fill(0),
            ff_a: true,
            ff_b: false,
          },
          slice1: {},
        },
      },
    };

    const bs = Bitstream.fromObject(data);
    expect(bs.lutK).toBe(4);
    expect(bs.clbs.clb_0.slice0.ffAEnabled).toBe(true);
  });

  it("fromObject with empty data returns empty bitstream", () => {
    const bs = Bitstream.fromObject({});
    expect(Object.keys(bs.clbs)).toHaveLength(0);
    expect(Object.keys(bs.routing)).toHaveLength(0);
    expect(Object.keys(bs.io)).toHaveLength(0);
  });
});
