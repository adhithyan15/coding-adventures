/**
 * Tests for logic gates — exhaustive truth table verification.
 */

import { describe, it, expect } from "vitest";
import {
  NOT,
  AND,
  OR,
  XOR,
  NAND,
  NOR,
  XNOR,
  nandNot,
  nandAnd,
  nandOr,
  nandXor,
  nandNor,
  nandXnor,
  andN,
  orN,
  mux,
  dmux,
  type Bit,
} from "../src/index.js";

// === Fundamental gates: truth table tests ===

describe("NOT", () => {
  it("NOT(0) = 1", () => {
    expect(NOT(0)).toBe(1);
  });

  it("NOT(1) = 0", () => {
    expect(NOT(1)).toBe(0);
  });
});

describe("AND", () => {
  it("AND(0, 0) = 0", () => {
    expect(AND(0, 0)).toBe(0);
  });

  it("AND(0, 1) = 0", () => {
    expect(AND(0, 1)).toBe(0);
  });

  it("AND(1, 0) = 0", () => {
    expect(AND(1, 0)).toBe(0);
  });

  it("AND(1, 1) = 1", () => {
    expect(AND(1, 1)).toBe(1);
  });
});

describe("OR", () => {
  it("OR(0, 0) = 0", () => {
    expect(OR(0, 0)).toBe(0);
  });

  it("OR(0, 1) = 1", () => {
    expect(OR(0, 1)).toBe(1);
  });

  it("OR(1, 0) = 1", () => {
    expect(OR(1, 0)).toBe(1);
  });

  it("OR(1, 1) = 1", () => {
    expect(OR(1, 1)).toBe(1);
  });
});

describe("XOR", () => {
  it("XOR(0, 0) = 0", () => {
    expect(XOR(0, 0)).toBe(0);
  });

  it("XOR(0, 1) = 1", () => {
    expect(XOR(0, 1)).toBe(1);
  });

  it("XOR(1, 0) = 1", () => {
    expect(XOR(1, 0)).toBe(1);
  });

  it("XOR(1, 1) = 0", () => {
    expect(XOR(1, 1)).toBe(0);
  });
});

describe("NAND", () => {
  it("NAND(0, 0) = 1", () => {
    expect(NAND(0, 0)).toBe(1);
  });

  it("NAND(0, 1) = 1", () => {
    expect(NAND(0, 1)).toBe(1);
  });

  it("NAND(1, 0) = 1", () => {
    expect(NAND(1, 0)).toBe(1);
  });

  it("NAND(1, 1) = 0", () => {
    expect(NAND(1, 1)).toBe(0);
  });
});

describe("NOR", () => {
  it("NOR(0, 0) = 1", () => {
    expect(NOR(0, 0)).toBe(1);
  });

  it("NOR(0, 1) = 0", () => {
    expect(NOR(0, 1)).toBe(0);
  });

  it("NOR(1, 0) = 0", () => {
    expect(NOR(1, 0)).toBe(0);
  });

  it("NOR(1, 1) = 0", () => {
    expect(NOR(1, 1)).toBe(0);
  });
});

describe("XNOR", () => {
  it("XNOR(0, 0) = 1", () => {
    expect(XNOR(0, 0)).toBe(1);
  });

  it("XNOR(0, 1) = 0", () => {
    expect(XNOR(0, 1)).toBe(0);
  });

  it("XNOR(1, 0) = 0", () => {
    expect(XNOR(1, 0)).toBe(0);
  });

  it("XNOR(1, 1) = 1", () => {
    expect(XNOR(1, 1)).toBe(1);
  });
});

// === NAND-derived gates: verify they match direct implementations ===

describe("NAND-derived gates", () => {
  const inputs: [Bit, Bit][] = [
    [0, 0],
    [0, 1],
    [1, 0],
    [1, 1],
  ];

  describe("nandNot matches NOT", () => {
    it.each([0, 1] as Bit[])("nandNot(%i) === NOT(%i)", (a) => {
      expect(nandNot(a)).toBe(NOT(a));
    });
  });

  describe("nandAnd matches AND", () => {
    it.each(inputs)("nandAnd(%i, %i) === AND(%i, %i)", (a, b) => {
      expect(nandAnd(a, b)).toBe(AND(a, b));
    });
  });

  describe("nandOr matches OR", () => {
    it.each(inputs)("nandOr(%i, %i) === OR(%i, %i)", (a, b) => {
      expect(nandOr(a, b)).toBe(OR(a, b));
    });
  });

  describe("nandXor matches XOR", () => {
    it.each(inputs)("nandXor(%i, %i) === XOR(%i, %i)", (a, b) => {
      expect(nandXor(a, b)).toBe(XOR(a, b));
    });
  });

  describe("nandNor matches NOR", () => {
    it.each(inputs)("nandNor(%i, %i) === NOR(%i, %i)", (a, b) => {
      expect(nandNor(a, b)).toBe(NOR(a, b));
    });
  });

  describe("nandXnor matches XNOR", () => {
    it.each(inputs)("nandXnor(%i, %i) === XNOR(%i, %i)", (a, b) => {
      expect(nandXnor(a, b)).toBe(XNOR(a, b));
    });
  });
});

// === Multi-input variants ===

describe("andN", () => {
  it("all ones returns 1", () => {
    expect(andN(1, 1, 1, 1)).toBe(1);
  });

  it("one zero returns 0", () => {
    expect(andN(1, 1, 0, 1)).toBe(0);
  });

  it("all zeros returns 0", () => {
    expect(andN(0, 0, 0)).toBe(0);
  });

  it("two inputs", () => {
    expect(andN(1, 1)).toBe(1);
    expect(andN(1, 0)).toBe(0);
  });

  it("too few inputs throws", () => {
    expect(() => andN(1 as Bit)).toThrow("at least 2");
  });
});

describe("orN", () => {
  it("all zeros returns 0", () => {
    expect(orN(0, 0, 0, 0)).toBe(0);
  });

  it("one one returns 1", () => {
    expect(orN(0, 0, 1, 0)).toBe(1);
  });

  it("all ones returns 1", () => {
    expect(orN(1, 1, 1)).toBe(1);
  });

  it("two inputs", () => {
    expect(orN(0, 0)).toBe(0);
    expect(orN(0, 1)).toBe(1);
  });

  it("too few inputs throws", () => {
    expect(() => orN(0 as Bit)).toThrow("at least 2");
  });
});

// === Multiplexer and Demultiplexer ===

describe("mux", () => {
  it("sel=0 selects a", () => {
    expect(mux(0, 1, 0)).toBe(0);
    expect(mux(1, 0, 0)).toBe(1);
  });

  it("sel=1 selects b", () => {
    expect(mux(0, 1, 1)).toBe(1);
    expect(mux(1, 0, 1)).toBe(0);
  });

  it("both same value", () => {
    expect(mux(1, 1, 0)).toBe(1);
    expect(mux(0, 0, 1)).toBe(0);
  });
});

describe("dmux", () => {
  it("sel=0 routes to a", () => {
    expect(dmux(1, 0)).toEqual([1, 0]);
    expect(dmux(0, 0)).toEqual([0, 0]);
  });

  it("sel=1 routes to b", () => {
    expect(dmux(1, 1)).toEqual([0, 1]);
    expect(dmux(0, 1)).toEqual([0, 0]);
  });
});

// === Input validation ===

describe("Input validation", () => {
  it("invalid int value throws RangeError", () => {
    expect(() => AND(2 as Bit, 1)).toThrow(RangeError);
  });

  it("negative value throws RangeError", () => {
    expect(() => OR(-1 as Bit, 0)).toThrow(RangeError);
  });

  it("string input throws TypeError", () => {
    expect(() => NOT("a" as unknown as Bit)).toThrow(TypeError);
  });

  it("boolean input throws TypeError", () => {
    expect(() => AND(true as unknown as Bit, false as unknown as Bit)).toThrow(
      TypeError,
    );
  });

  it("float input throws RangeError", () => {
    expect(() => XOR(1.5 as Bit, 0)).toThrow(RangeError);
  });
});
