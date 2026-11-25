/**
 * LGE-TS-HARDEN: type-confusion matrix, validateBit name hardening,
 * mux/dmux arity, n-ary caps, checked dependency results.
 * Spies assert that NO caller coercion hook ever runs for rejected values.
 */
import { describe, it, expect } from "vitest";
import {
  NOT, AND, OR, XOR, NAND, NOR, XNOR, nandNot, nandAnd, nandOr, nandXor,
  nandNor, nandXnor, andN, orN, xorN, mux, dmux, validateBit,
  MAX_NAME_LENGTH, MAX_WIDTH, InternalGateError,
} from "../src/index.js";
import { CMOSNand } from "@coding-adventures/transistors";

const hooks = { n: 0 };

function makeHostile(): [string, unknown][] {
  const toPrim = {
    [Symbol.toPrimitive]() { hooks.n++; return 1; },
    valueOf() { hooks.n++; return 1; },
    toString() { hooks.n++; return "1"; },
  };
  const getter = {};
  Object.defineProperty(getter, "valueOf", { get() { hooks.n++; return () => 1; } });
  const proxy = new Proxy({}, {
    get() { hooks.n++; return () => 1; },
    has() { hooks.n++; return true; },
    getPrototypeOf() { hooks.n++; return null; },
  });
  return [
    ["Symbol.toPrimitive object", toPrim],
    ["getter-backed", getter],
    ["Proxy", proxy],
    ["Object.create(null)", Object.create(null)],
    ['"1 "', "1 "], ['" 1"', " 1"], ['"1\\n"', "1\n"], ['"0x1"', "0x1"], ['"1e0"', "1e0"],
    ["2**32", 2 ** 32], ["2**32+1", 2 ** 32 + 1],
    ["new Number(1)", new Number(1)], ["-0", -0], ["NaN", NaN], ["true", true],
    ["null", null], ["undefined", undefined], ["1n", 1n], ["Symbol", Symbol("x")],
  ];
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Fn = (...a: any[]) => unknown;
const gates: [string, Fn, number][] = [
  ["NOT", NOT, 1], ["AND", AND, 2], ["OR", OR, 2], ["XOR", XOR, 2],
  ["NAND", NAND, 2], ["NOR", NOR, 2], ["XNOR", XNOR, 2],
  ["nandNot", nandNot, 1], ["nandAnd", nandAnd, 2], ["nandOr", nandOr, 2],
  ["nandXor", nandXor, 2], ["nandNor", nandNor, 2], ["nandXnor", nandXnor, 2],
  ["mux", mux, 3], ["dmux", dmux, 2], ["andN", andN, 2], ["orN", orN, 2], ["xorN", xorN, 2],
];

describe("type-confusion matrix: no coercion hook runs", () => {
  for (const [gname, fn, arity] of gates) {
    for (const [vname, bad] of makeHostile()) {
      for (let pos = 0; pos < arity; pos++) {
        it(`${gname} rejects ${vname} at arg ${pos}`, () => {
          const args: unknown[] = new Array(arity).fill(1);
          args[pos] = bad;
          hooks.n = 0;
          expect(() => fn(...args)).toThrow();
          expect(hooks.n).toBe(0);
        });
      }
    }
  }
});

describe("validateBit name hardening", () => {
  it("never interpolates a non-string name", () => {
    hooks.n = 0;
    const evil = {
      toString() { hooks.n++; return "x"; },
      [Symbol.toPrimitive]() { hooks.n++; return "x"; },
    };
    for (const bad of ["s", 0.5, null]) {
      expect(() => validateBit(bad, evil as unknown as string)).toThrow(TypeError);
    }
    expect(() => validateBit(2, evil as unknown as string)).toThrow(/^argument must be 0 or 1/);
    expect(() => validateBit(2, Symbol("s") as unknown as string)).toThrow(RangeError);
    expect(() => validateBit(2, Object.create(null))).toThrow(RangeError);
    expect(hooks.n).toBe(0);
  });
  it("replaces over-long names", () => {
    const long = "n".repeat(MAX_NAME_LENGTH + 1);
    expect(() => validateBit(2, long)).toThrow(/^argument must be 0 or 1/);
    expect(() => validateBit(2, "n".repeat(MAX_NAME_LENGTH))).toThrow(/^n{64} must/);
  });
});

describe("arity", () => {
  it("mux/dmux reject wrong argument counts", () => {
    expect(() => (mux as Fn)(0, 1, 0, 1)).toThrow(TypeError);
    expect(() => (mux as Fn)(0, 1)).toThrow(TypeError);
    expect(() => (dmux as Fn)(1, 0, 0)).toThrow(TypeError);
    expect(() => (dmux as Fn)(1)).toThrow(TypeError);
    expect(mux(0, 1, 1)).toBe(1);
    expect(dmux(1, 1)).toEqual([0, 1]);
  });
});

describe("n-ary caps", () => {
  it("accepts large inputs and rejects beyond MAX_WIDTH", () => {
    expect(xorN(...new Array<0 | 1>(1000).fill(1))).toBe(0);
    const big = new Array<0 | 1>(MAX_WIDTH + 1).fill(1);
    for (const f of [andN, orN, xorN] as Fn[]) {
      expect(() => f(...big)).toThrow(RangeError);
    }
  });
  it("accepts exactly MAX_WIDTH arguments", () => {
    const ones = new Array<0 | 1>(MAX_WIDTH).fill(1);
    expect(andN(...ones)).toBe(1);
    expect(orN(...ones)).toBe(1);
    expect(xorN(...ones)).toBe(0);
  });
  it("cap is checked before any element is inspected", () => {
    hooks.n = 0;
    const bad = { valueOf() { hooks.n++; return 1; } };
    const big = new Array(MAX_WIDTH + 1).fill(bad);
    expect(() => andN(...(big as never[]))).toThrow(RangeError);
    expect(hooks.n).toBe(0);
  });
  it("andN/orN still need 2 inputs", () => {
    expect(() => (andN as Fn)(1)).toThrow(RangeError);
    expect(() => (orN as Fn)()).toThrow(RangeError);
    expect(xorN(1, 1, 1)).toBe(1);
    expect(andN(1, 1, 1)).toBe(1);
    expect(orN(0, 0, 1)).toBe(1);
  });
});

describe("checked dependency results", () => {
  it("throws InternalGateError (not a caller error) on a non-bit result", () => {
    const proto = CMOSNand.prototype as unknown as { evaluateDigital: unknown };
    const orig = proto.evaluateDigital;
    proto.evaluateDigital = () => 2;
    try {
      let err: unknown;
      try { NAND(1, 1); } catch (e) { err = e; }
      expect(err).toBeInstanceOf(InternalGateError);
      expect(err).not.toBeInstanceOf(TypeError);
      expect(err).not.toBeInstanceOf(RangeError);
    } finally {
      proto.evaluateDigital = orig;
    }
    expect(NAND(1, 1)).toBe(0);
  });
});
