/** The public surface of index.ts must not drift when internals move. */
import { describe, it, expect } from "vitest";
import * as pkg from "../src/index.js";

describe("index.ts exports", () => {
  it("exports exactly the expected runtime names", () => {
    expect(Object.keys(pkg).sort()).toMatchInlineSnapshot(`
      [
        "AND",
        "InternalGateError",
        "MAX_COUNTER_WIDTH",
        "MAX_NAME_LENGTH",
        "MAX_SELECT_BITS",
        "MAX_WIDTH",
        "NAND",
        "NOR",
        "NOT",
        "OR",
        "XNOR",
        "XOR",
        "andN",
        "counter",
        "dFlipFlop",
        "dLatch",
        "decoder",
        "demuxN",
        "dmux",
        "encoder",
        "mux",
        "mux2",
        "mux4",
        "muxN",
        "nandAnd",
        "nandNor",
        "nandNot",
        "nandOr",
        "nandXnor",
        "nandXor",
        "orN",
        "priorityEncoder",
        "register",
        "shiftRegister",
        "srLatch",
        "triState",
        "validateBit",
        "xorN",
      ]
    `);
  });
  it("keeps the bound values", () => {
    expect(pkg.MAX_WIDTH).toBe(65536);
    expect(pkg.MAX_COUNTER_WIDTH).toBe(64);
    expect(pkg.MAX_SELECT_BITS).toBe(16);
    expect(pkg.MAX_NAME_LENGTH).toBe(64);
  });
});
