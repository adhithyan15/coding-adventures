/**
 * Lane-local exhaustive validation of the 11 basic gates.
 *
 * Validation rule under test (see validateBit in src/gates.ts):
 *   non-number or non-integer number -> TypeError
 *   integer other than 0/1, or -0    -> RangeError
 *   wrong argument count             -> TypeError
 * JS has no int type: 1.0 === 1, so 1.0 is accepted (it is the same value).
 */
import { describe, expect, it } from "vitest";
import {
  AND, NAND, NOR, NOT, OR, XNOR, XOR,
  nandAnd, nandNot, nandOr, nandXor, xorN, andN, orN,
} from "../src/index.js";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Fn = (...a: any[]) => unknown;
const GATES: Record<string, { fn: Fn; arity: number; table: number[][] }> = {
  NOT: { fn: NOT, arity: 1, table: [[0, 1], [1, 0]] },
  AND: { fn: AND, arity: 2, table: [[0, 0, 0], [0, 1, 0], [1, 0, 0], [1, 1, 1]] },
  OR: { fn: OR, arity: 2, table: [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 1]] },
  XOR: { fn: XOR, arity: 2, table: [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]] },
  NAND: { fn: NAND, arity: 2, table: [[0, 0, 1], [0, 1, 1], [1, 0, 1], [1, 1, 0]] },
  NOR: { fn: NOR, arity: 2, table: [[0, 0, 1], [0, 1, 0], [1, 0, 0], [1, 1, 0]] },
  XNOR: { fn: XNOR, arity: 2, table: [[0, 0, 1], [0, 1, 0], [1, 0, 0], [1, 1, 1]] },
  nandNot: { fn: nandNot, arity: 1, table: [[0, 1], [1, 0]] },
  nandAnd: { fn: nandAnd, arity: 2, table: [[0, 0, 0], [0, 1, 0], [1, 0, 0], [1, 1, 1]] },
  nandOr: { fn: nandOr, arity: 2, table: [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 1]] },
  nandXor: { fn: nandXor, arity: 2, table: [[0, 0, 0], [0, 1, 1], [1, 0, 1], [1, 1, 0]] },
};
const NAMES = Object.keys(GATES);
const isBit = (r: unknown) => Object.is(r, 0) || Object.is(r, 1);

/** Build an argument vector with `bad` at `pos` and `other` elsewhere. */
function vec(arity: number, pos: number, bad: unknown, other: unknown): unknown[] {
  return Array.from({ length: arity }, (_, i) => (i === pos ? bad : other));
}

describe("literal truth tables", () => {
  for (const n of NAMES) {
    it(n, () => {
      const { fn, arity, table } = GATES[n];
      expect(table.length).toBe(1 << arity);
      for (const row of table) {
        const r = fn(...row.slice(0, arity));
        expect(r).toBe(row[arity]);
        expect(isBit(r)).toBe(true);
      }
    });
  }
  it("NAND-built equals direct on all inputs", () => {
    for (const a of [0, 1]) {
      expect(nandNot(a)).toBe(NOT(a));
      for (const b of [0, 1]) {
        expect(nandAnd(a, b)).toBe(AND(a, b));
        expect(nandOr(a, b)).toBe(OR(a, b));
        expect(nandXor(a, b)).toBe(XOR(a, b));
      }
    }
  });
  it("1.0 is the same value as 1 and is accepted", () => {
    expect(AND(1.0, 1.0)).toBe(1);
    expect(NOT(0.0)).toBe(1);
  });
});

describe("exhaustive integer rejection -> RangeError", () => {
  const bad: number[] = [];
  for (let i = -1024; i <= 1024; i++) if (i !== 0 && i !== 1) bad.push(i);
  bad.push(2 ** 31, -(2 ** 31), 2 ** 53, -(2 ** 53), Number.MAX_SAFE_INTEGER,
    -Number.MAX_SAFE_INTEGER, Number.MAX_VALUE, -Number.MAX_VALUE);
  for (const n of NAMES) {
    const { fn, arity } = GATES[n];
    for (let pos = 0; pos < arity; pos++) {
      it(`${n} arg ${pos}`, () => {
        for (const v of bad) {
          // other argument valid (0 and 1) and also out-of-range (7)
          for (const other of [0, 1, 7]) {
            expect(() => fn(...vec(arity, pos, v, other))).toThrow(RangeError);
          }
        }
      });
    }
  }
});

describe("type-confusion matrix", () => {
  const typeErr: [string, unknown][] = [
    ["true", true], ["false", false], ["null", null], ["undefined", undefined],
    ["0.5", 0.5], ["1.0000000001", 1.0000000001], ["NaN", NaN],
    ["Infinity", Infinity], ["-Infinity", -Infinity],
    ["'1'", "1"], ["'0'", "0"], ["''", ""], ["'1abc'", "1abc"],
    ["1n", 1n], ["0n", 0n], ["[1]", [1]], ["[]", []], ["{}", {}],
    ["valueOf obj", { valueOf() { return 1; } }],
    ["toString obj", { toString() { return "1"; } }],
    ["new Number(1)", new Number(1)], ["Symbol", Symbol("1")], ["fn", () => 1],
  ];
  for (const n of NAMES) {
    const { fn, arity } = GATES[n];
    for (let pos = 0; pos < arity; pos++) {
      it(`${n} arg ${pos}`, () => {
        for (const [label, v] of typeErr) {
          for (const other of [0, 1, v]) {
            expect(() => fn(...vec(arity, pos, v, other)), label).toThrow(TypeError);
          }
        }
        // -0 is an integer but is not the bit 0: RangeError.
        for (const other of [0, 1]) {
          expect(() => fn(...vec(arity, pos, -0, other))).toThrow(RangeError);
        }
      });
    }
  }
});

describe("results are exactly +0 or 1, never -0", () => {
  for (const n of NAMES) {
    it(n, () => {
      const { fn, arity } = GATES[n];
      for (let m = 0; m < 1 << arity; m++) {
        const args = Array.from({ length: arity }, (_, i) => (m >> i) & 1);
        const r = fn(...args);
        expect(typeof r).toBe("number");
        expect(isBit(r)).toBe(true);
      }
    });
  }
});

describe("arity", () => {
  for (const n of NAMES) {
    const { fn, arity } = GATES[n];
    it(`${n} too few arguments throws`, () => {
      expect(() => fn()).toThrow(TypeError);
      if (arity === 2) expect(() => fn(1)).toThrow(TypeError);
    });
    it(`${n} extra arguments throw`, () => {
      expect(() => fn(...Array(arity + 1).fill(1))).toThrow(TypeError);
      expect(() => fn(...Array(arity).fill(1), undefined)).toThrow(TypeError);
    });
  }
});

describe("xorN (parity)", () => {
  it("covers 0, 1, and many inputs", () => {
    expect(xorN()).toBe(0);
    expect(xorN(1)).toBe(1);
    expect(xorN(1, 1)).toBe(0);
    expect(xorN(1, 1, 1)).toBe(1);
    expect(xorN(1, 0, 1, 0)).toBe(0);
  });
  it("validates every input, including the single-input case", () => {
    for (const bad of [2, -0, "1", {}, NaN]) {
      expect(() => (xorN as Fn)(bad)).toThrow();
      expect(() => (xorN as Fn)(1, bad)).toThrow();
      expect(() => (xorN as Fn)(bad, 1, 0)).toThrow();
    }
    expect(() => (xorN as Fn)(2)).toThrow(RangeError);
    expect(() => (xorN as Fn)(-0)).toThrow(RangeError);
    expect(() => (xorN as Fn)("1")).toThrow(TypeError);
    expect(() => (xorN as Fn)({})).toThrow(TypeError);
    expect(() => (xorN as Fn)(NaN)).toThrow(TypeError);
  });
  it("andN/orN require at least 2 inputs and validate all", () => {
    expect(() => (andN as Fn)()).toThrow(RangeError);
    expect(() => (orN as Fn)(1)).toThrow(RangeError);
    expect(() => (andN as Fn)(1, 1, 2)).toThrow(RangeError);
    expect(() => (orN as Fn)(0, 0, "x")).toThrow(TypeError);
  });
});
