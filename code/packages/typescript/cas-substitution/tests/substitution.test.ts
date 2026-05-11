import { describe, expect, it } from "vitest";
import { ADD, MUL, POW, app, equals, int, sym } from "@coding-adventures/symbolic-ir";
import { blank, pattern, replaceAll, replaceAllMany, rule, subst, substMany } from "../src/index";

describe("cas-substitution", () => {
  it("performs structural substitution", () => {
    const expr = app(POW, [sym("x"), int(2)]);
    expect(subst(int(3), sym("x"), expr)).toEqual(app(POW, [int(3), int(2)]));
  });

  it("applies substitutions sequentially", () => {
    const expr = app(ADD, [sym("x"), sym("y")]);
    expect(substMany([[sym("x"), sym("y")], [sym("y"), int(1)]], expr)).toEqual(app(ADD, [int(1), int(1)]));
  });

  it("replaces with structural rules", () => {
    const expr = app(ADD, [sym("x"), int(0)]);
    expect(replaceAll(expr, rule(app(ADD, [sym("x"), int(0)]), sym("x")))).toEqual(sym("x"));
  });

  it("supports single-pass pattern rules", () => {
    const a = pattern("a", blank());
    const expr = app(POW, [sym("y"), int(2)]);
    const out = replaceAll(expr, rule(app(POW, [a, int(2)]), app(MUL, [a, a])));
    expect(equals(out, app(MUL, [sym("y"), sym("y")]))).toBe(true);
  });

  it("applies multiple rules in order", () => {
    const out = replaceAllMany(sym("x"), [rule(sym("x"), sym("y")), rule(sym("y"), int(1))]);
    expect(out).toEqual(int(1));
  });
});
