import { describe, expect, it } from "vitest";
import { blank, named, rewrite, rule } from "@coding-adventures/cas-pattern-matching";
import { ADD, EXP, LOG, MUL, NEG, POW, app, int, sym } from "@coding-adventures/symbolic-ir";
import { IDENTITY_RULES, buildIdentityRules, canonical, numericFold, simplify } from "../src/index";

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

  it("exports a fresh identity rule table matching the Python and Rust parity set", () => {
    const rules = buildIdentityRules();
    const x = named("x", blank());

    expect(IDENTITY_RULES).toHaveLength(15);
    expect(rules).toHaveLength(15);
    expect(rules).not.toBe(buildIdentityRules());
    expect(rules[0]).toEqual(rule(app(ADD, [x, int(0)]), x));
    expect(rules[11]).toEqual(rule(app(LOG, [app(EXP, [x])]), x));
    for (const candidate of rules) {
      expect(candidate.kind).toBe("apply");
      if (candidate.kind === "apply") {
        expect(candidate.head).toEqual(sym("Rule"));
        expect(candidate.args).toHaveLength(2);
      }
    }
  });

  it("rewrites additive identity through the identity rule table", () => {
    expect(rewrite(app(ADD, [sym("x"), int(0)]), IDENTITY_RULES)).toEqual(sym("x"));
  });

  it("rewrites log and exp inverses through captured pattern variables", () => {
    const expr = app(LOG, [app(EXP, [app(ADD, [sym("x"), int(1)])])]);

    expect(rewrite(expr, IDENTITY_RULES)).toEqual(app(ADD, [sym("x"), int(1)]));
    expect(simplify(expr)).toEqual(app(ADD, [int(1), sym("x")]));
  });

  it("simplifies double negation", () => {
    expect(simplify(app(NEG, [app(NEG, [sym("x")])]))).toEqual(sym("x"));
  });
});
