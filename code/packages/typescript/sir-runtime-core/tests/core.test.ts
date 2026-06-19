/** Tests for @coding-adventures/sir-runtime-core. */

import { describe, it, expect, vi } from "vitest";
import * as sir from "../src/index.js";

describe("truthiness (false/nil-only)", () => {
  it("only false and nil are falsy", () => {
    expect(sir.truthy(false)).toBe(false);
    expect(sir.truthy(null)).toBe(false);
  });

  it("everything else is truthy", () => {
    for (const v of [0, "", 1, "x", sir.intern("s")] as sir.Val[]) {
      expect(sir.truthy(v)).toBe(true);
    }
  });
});

describe("symbols", () => {
  it("interned symbols share identity", () => {
    expect(sir.intern("a")).toBe(sir.intern("a"));
    expect(sir.intern("a")).not.toBe(sir.intern("b"));
  });

  it("eq is symbol-aware", () => {
    expect(sir.eq(sir.intern("a"), sir.intern("a"))).toBe(true);
    expect(sir.eq(sir.intern("a"), sir.intern("b"))).toBe(false);
    expect(sir.eq(1, 1)).toBe(true);
  });
});

describe("pairs", () => {
  it("cons/car/cdr", () => {
    const p = sir.cons(1, 2);
    expect(sir.car(p)).toBe(1);
    expect(sir.cdr(p)).toBe(2);
    expect(sir.isPair(p)).toBe(true);
    expect(sir.isPair(1)).toBe(false);
  });

  it("car/cdr reject non-pairs", () => {
    expect(() => sir.car(1)).toThrow(TypeError);
    expect(() => sir.cdr(1)).toThrow(TypeError);
  });

  it("list display", () => {
    const proper = sir.cons(1, sir.cons(2, sir.cons(3, null)));
    expect(sir.toDisplay(proper)).toBe("(1 2 3)");
    expect(sir.toDisplay(sir.cons(1, 2))).toBe("(1 . 2)");
  });
});

describe("predicates and display", () => {
  it("predicates", () => {
    expect(sir.isNull(null)).toBe(true);
    expect(sir.isNull(0)).toBe(false);
    expect(sir.isNumber(3)).toBe(true);
    expect(sir.isSymbol(sir.intern("x"))).toBe(true);
    expect(sir.isSymbol("x")).toBe(false);
  });

  it("display forms", () => {
    expect(sir.toDisplay(null)).toBe("nil");
    expect(sir.toDisplay(true)).toBe("#t");
    expect(sir.toDisplay(false)).toBe("#f");
    expect(sir.toDisplay(sir.intern("sym"))).toBe("sym");
    expect(sir.toDisplay(42)).toBe("42");
    expect(sir.toDisplay("hi")).toBe("hi");
  });

  it("print returns null and writes display form", () => {
    const lines: string[] = [];
    const spy = vi.spyOn(console, "log").mockImplementation((s: string) => {
      lines.push(s);
    });
    expect(sir.print(null)).toBe(null);
    spy.mockRestore();
    expect(lines).toEqual(["nil"]);
  });
});

describe("arithmetic", () => {
  it("variadic", () => {
    expect(sir.add(1, 2, 3)).toBe(6);
    expect(sir.add()).toBe(0);
    expect(sir.sub(10, 3, 2)).toBe(5);
    expect(sir.sub(5)).toBe(-5);
    expect(sir.sub()).toBe(0);
    expect(sir.mul(2, 3, 4)).toBe(24);
    expect(sir.mul()).toBe(1);
  });

  it("truncating division", () => {
    expect(sir.div(7, 2)).toBe(3);
    expect(sir.div(-7, 2)).toBe(-3);
    expect(sir.div()).toBe(0);
    expect(sir.div(9, 3, 1)).toBe(3);
  });

  it("comparisons", () => {
    expect(sir.lt(1, 2)).toBe(true);
    expect(sir.gt(2, 1)).toBe(true);
  });
});

describe("closures, globals, dispatch", () => {
  it("makeClosure prepends captures", () => {
    const f = (a: sir.Val, b: sir.Val, c: sir.Val): sir.Val =>
      (a as number) + (b as number) + (c as number);
    const c = sir.makeClosure(f, [10, 20]);
    expect(sir.apply(c, [5])).toBe(35);
  });

  it("apply rejects non-closures", () => {
    expect(() => sir.apply(42, [])).toThrow(TypeError);
  });

  it("apply on a nil target raises LocalJumpError (no block given)", () => {
    // No-block-given case: a `yield` reached through a nil block parameter.
    expect(() => sir.apply(null, [1, 2])).toThrow(sir.LocalJumpError);
    expect(() => sir.apply(null, [])).toThrow(/no block given/);
    // Distinct from the non-closure TypeError so the two stay separable.
    expect(new sir.LocalJumpError()).not.toBeInstanceOf(TypeError);
  });

  it("global store roundtrip", () => {
    sir.globalSet("g1", 99);
    expect(sir.globalGet("g1")).toBe(99);
    expect(sir.globalGetStatic("g1")).toBe(99);
    sir.globalSet(sir.intern("g2"), 7);
    expect(sir.globalGet(sir.intern("g2"))).toBe(7);
  });

  it("undefined globals throw", () => {
    expect(() => sir.globalGet("nope")).toThrow();
    expect(() => sir.globalGetStatic("nope2")).toThrow();
  });

  it("callBuiltin and builtinClosure", () => {
    expect(sir.callBuiltin("+", [1, 2, 3])).toBe(6);
    expect(() => sir.callBuiltin("nope", [])).toThrow();
    const plus = sir.builtinClosure("+");
    expect(sir.apply(plus, [4, 5])).toBe(9);
  });
});
