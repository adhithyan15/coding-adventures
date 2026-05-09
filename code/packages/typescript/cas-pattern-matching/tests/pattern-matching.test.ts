import { describe, expect, it } from "vitest";
import { ADD, MUL, POW, app, int, numberNode, rational, sym } from "@coding-adventures/symbolic-ir";
import {
  Bindings,
  applyRule,
  blank,
  blankTyped,
  isRewriteCycleError,
  matchPattern,
  named,
  rewrite,
  rule,
  ruleDelayed,
} from "../src/index";

describe("blank", () => {
  it("matches unconstrained nodes", () => {
    expect(matchPattern(blank(), int(42))).not.toBeNull();
    expect(matchPattern(blank(), sym("x"))).not.toBeNull();
    expect(matchPattern(blank(), app(ADD, [int(1), int(2)]))).not.toBeNull();
    expect(matchPattern(blank(), rational(1, 2))).not.toBeNull();
    expect(matchPattern(blank(), numberNode(3.14))).not.toBeNull();
  });

  it("honors head constraints", () => {
    expect(matchPattern(blankTyped("Integer"), int(7))).not.toBeNull();
    expect(matchPattern(blankTyped("Integer"), sym("x"))).toBeNull();
    expect(matchPattern(blankTyped("Symbol"), sym("Pi"))).not.toBeNull();
    expect(matchPattern(blankTyped("Rational"), rational(1, 3))).not.toBeNull();
    expect(matchPattern(blankTyped("Float"), numberNode(2.5))).not.toBeNull();
    expect(matchPattern(blankTyped("Add"), app(ADD, [int(1), int(2)]))).not.toBeNull();
    expect(matchPattern(blankTyped("Add"), app(MUL, [int(2), int(3)]))).toBeNull();
  });
});

describe("named captures", () => {
  it("captures values and multiple names", () => {
    const captured = matchPattern(named("x", blank()), int(5));
    expect(captured?.get("x")).toEqual(int(5));

    const pattern = app(ADD, [named("a", blank()), named("b", blank())]);
    const target = app(ADD, [int(2), int(3)]);
    const bindings = matchPattern(pattern, target);
    expect(bindings?.get("a")).toEqual(int(2));
    expect(bindings?.get("b")).toEqual(int(3));
  });

  it("checks repeated-name consistency", () => {
    const x = named("x", blank());
    expect(matchPattern(app(ADD, [x, x]), app(ADD, [sym("a"), sym("a")]))).not.toBeNull();
    expect(matchPattern(app(ADD, [x, x]), app(ADD, [sym("a"), sym("b")]))).toBeNull();
  });

  it("supports typed named captures", () => {
    const pattern = named("n", blankTyped("Integer"));
    expect(matchPattern(pattern, int(10))).not.toBeNull();
    expect(matchPattern(pattern, sym("x"))).toBeNull();
  });
});

describe("structural matching", () => {
  it("matches literal and apply structure", () => {
    expect(matchPattern(int(7), int(7))).not.toBeNull();
    expect(matchPattern(int(7), int(8))).toBeNull();
    expect(matchPattern(sym("Pi"), sym("Pi"))).not.toBeNull();
    expect(matchPattern(app(POW, [sym("x"), int(2)]), app(POW, [sym("x"), int(2)]))).not.toBeNull();
    expect(matchPattern(app(ADD, [int(1), int(2)]), app(MUL, [int(1), int(2)]))).toBeNull();
    expect(matchPattern(app(ADD, [int(1), int(2)]), app(ADD, [int(1), int(2), int(3)]))).toBeNull();
  });
});

describe("rules", () => {
  it("fires and substitutes captured patterns", () => {
    const x = named("x", blank());
    const double = rule(app(ADD, [x, x]), app(MUL, [int(2), x]));
    expect(applyRule(double, app(ADD, [sym("a"), sym("a")]))).toEqual(app(MUL, [int(2), sym("a")]));
  });

  it("returns null on no match and supports delayed rules", () => {
    const x = named("x", blank());
    const zeroPower = rule(app(POW, [x, int(0)]), int(1));
    expect(applyRule(zeroPower, app(POW, [sym("z"), int(0)]))).toEqual(int(1));
    expect(applyRule(zeroPower, sym("y"))).toBeNull();

    const delayed = ruleDelayed(app(ADD, [x, int(0)]), x);
    expect(applyRule(delayed, app(ADD, [int(7), int(0)]))).toEqual(int(7));
  });
});

describe("rewrite", () => {
  it("rewrites bottom-up to a fixed point", () => {
    const x = named("x", blank());
    const removeZero = rule(app(ADD, [x, int(0)]), x);
    const inner = app(ADD, [sym("z"), int(0)]);
    const outer = app(ADD, [inner, int(0)]);
    expect(rewrite(outer, [removeZero], 100)).toEqual(sym("z"));
  });

  it("leaves expressions unchanged when no rules are present", () => {
    const expr = app(ADD, [int(1), int(2)]);
    expect(rewrite(expr, [], 100)).toEqual(expr);
  });

  it("reports non-converging rule cycles", () => {
    const x = named("x", blank());
    const r1 = rule(app(sym("f"), [x]), app(sym("g"), [x]));
    const r2 = rule(app(sym("g"), [x]), app(sym("f"), [x]));
    const result = rewrite(app(sym("f"), [int(1)]), [r1, r2], 10);
    expect(isRewriteCycleError(result)).toBe(true);
    if (isRewriteCycleError(result)) {
      expect(result.maxIterations).toBe(10);
    }
  });
});

describe("bindings", () => {
  it("are immutable and inspectable", () => {
    const empty = Bindings.empty();
    expect(empty.isEmpty).toBe(true);
    expect(empty.size).toBe(0);
    const one = empty.bind("x", int(5));
    expect(one.get("x")).toEqual(int(5));
    expect(one.contains("x")).toBe(true);
    expect(one.bind("x", int(5)).equals(one)).toBe(true);
    expect(one.entries()).toEqual([["x", int(5)]]);
  });
});
