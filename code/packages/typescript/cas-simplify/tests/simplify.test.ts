import { describe, expect, it } from "vitest";
import { ADD, MUL, NEG, POW, app, int, sym } from "@coding-adventures/symbolic-ir";
import { canonical, numericFold, simplify } from "../src/index";

describe("cas-simplify", () => {
  it("canonicalizes flattened commutative expressions", () => {
    const expr = app(ADD, [sym("b"), app(ADD, [sym("a"), int(2)])]);
    expect(canonical(expr)).toEqual(app(ADD, [int(2), sym("a"), sym("b")]));
  });

  it("folds integer arithmetic", () => {
    expect(numericFold(app(MUL, [int(6), int(7)]))).toEqual(int(42));
    expect(numericFold(app(POW, [int(2), int(5)]))).toEqual(int(32));
  });

  it("applies identity simplification to fixed point", () => {
    const expr = app(MUL, [int(1), app(ADD, [sym("x"), int(0)])]);
    expect(simplify(expr)).toEqual(sym("x"));
  });

  it("simplifies double negation", () => {
    expect(simplify(app(NEG, [app(NEG, [sym("x")])]))).toEqual(sym("x"));
  });
});
