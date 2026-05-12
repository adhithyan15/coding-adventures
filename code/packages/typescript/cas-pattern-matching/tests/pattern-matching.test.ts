import { describe, expect, it } from "vitest";
import { ADD, COS, LIST, MUL, POW, SIN, app, int, numberNode, rational, stringNode, sym } from "@coding-adventures/symbolic-ir";
import {
  Bindings,
  MatchDeclareContext,
  RuleStore,
  applyRule,
  blank,
  blankTyped,
  isPattern,
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

describe("matchdeclare context", () => {
  it("mutates and queries declarations", () => {
    const ctx = new MatchDeclareContext();
    expect(ctx.isDeclared("x")).toBe(false);
    expect(ctx.getPredicate("x")).toBeUndefined();

    ctx.declare("x", "IntegerP");
    expect(ctx.isDeclared("x")).toBe(true);
    expect(ctx.getPredicate("x")).toBe("integerp");

    ctx.declare("x", "floatp");
    expect(ctx.getPredicate("x")).toBe("floatp");

    ctx.forget("x");
    expect(ctx.isDeclared("x")).toBe(false);

    ctx.declare("a", "any");
    ctx.declare("b", "symbolp");
    ctx.forgetAll();
    expect(ctx.isDeclared("a")).toBe(false);
    expect(ctx.isDeclared("b")).toBe(false);
  });

  it("maps predicates to matcher blank constraints", () => {
    const integerCtx = new MatchDeclareContext();
    integerCtx.declare("n", "integerp");
    const integerPattern = integerCtx.compilePattern(sym("n"));
    expect(isPattern(integerPattern)).toBe(true);
    expect(matchPattern(integerPattern, int(3))).not.toBeNull();
    expect(matchPattern(integerPattern, sym("x"))).toBeNull();

    const symbolCtx = new MatchDeclareContext();
    symbolCtx.declare("s", "symbolp");
    expect(matchPattern(symbolCtx.compilePattern(sym("s")), sym("x"))).not.toBeNull();
    expect(matchPattern(symbolCtx.compilePattern(sym("s")), int(1))).toBeNull();

    const floatCtx = new MatchDeclareContext();
    floatCtx.declare("f", "floatp");
    expect(matchPattern(floatCtx.compilePattern(sym("f")), numberNode(1.5))).not.toBeNull();
    expect(matchPattern(floatCtx.compilePattern(sym("f")), rational(3, 2))).toBeNull();

    const rationalCtx = new MatchDeclareContext();
    rationalCtx.declare("r", "rationalp");
    expect(matchPattern(rationalCtx.compilePattern(sym("r")), rational(1, 4))).not.toBeNull();
    expect(matchPattern(rationalCtx.compilePattern(sym("r")), numberNode(0.25))).toBeNull();

    const listCtx = new MatchDeclareContext();
    listCtx.declare("xs", "listp");
    expect(matchPattern(listCtx.compilePattern(sym("xs")), app(LIST, [int(1), int(2)]))).not.toBeNull();
    expect(matchPattern(listCtx.compilePattern(sym("xs")), sym("xs"))).toBeNull();

    const stringCtx = new MatchDeclareContext();
    stringCtx.declare("text", "stringp");
    expect(matchPattern(stringCtx.compilePattern(sym("text")), stringNode("hello"))).not.toBeNull();
    expect(matchPattern(stringCtx.compilePattern(sym("text")), sym("hello"))).toBeNull();
  });

  it("keeps unconstrained and unknown predicates safe", () => {
    for (const tag of ["true", "all", "any", "numberp", "mysteryp"]) {
      const ctx = new MatchDeclareContext();
      ctx.declare("x", tag);
      const compiled = ctx.compilePattern(sym("x"));
      expect(matchPattern(compiled, int(1))).not.toBeNull();
      expect(matchPattern(compiled, sym("anything"))).not.toBeNull();
    }
  });

  it("recursively compiles apply heads and arguments", () => {
    const ctx = new MatchDeclareContext();
    ctx.declare("f", "symbolp");
    ctx.declare("x", "any");
    ctx.declare("n", "integerp");

    const raw = app(sym("f"), [app(ADD, [sym("x"), sym("n")])]);
    const compiled = ctx.compilePattern(raw);
    expect(compiled.kind).toBe("apply");
    if (compiled.kind !== "apply") return;
    expect(isPattern(compiled.head)).toBe(true);
    expect(isPattern(compiled.args[0].kind === "apply" ? compiled.args[0].args[0] : sym("bad"))).toBe(true);

    const target = app(SIN, [app(ADD, [sym("theta"), int(2)])]);
    const bindings = matchPattern(compiled, target);
    expect(bindings?.get("f")).toEqual(SIN);
    expect(bindings?.get("x")).toEqual(sym("theta"));
    expect(bindings?.get("n")).toEqual(int(2));

    expect(matchPattern(compiled, app(SIN, [app(ADD, [sym("theta"), sym("two")])]))).toBeNull();
  });

  it("supports end-to-end applyRule and rewrite flows", () => {
    const ctx = new MatchDeclareContext();
    ctx.declare("x", "any");
    const x = sym("x");
    const lhs = app(ADD, [
      app(POW, [app(SIN, [x]), int(2)]),
      app(POW, [app(COS, [x]), int(2)]),
    ]);
    const pythagorean = rule(ctx.compilePattern(lhs), int(1));
    const theta = sym("theta");
    const target = app(ADD, [
      app(POW, [app(SIN, [theta]), int(2)]),
      app(POW, [app(COS, [theta]), int(2)]),
    ]);
    expect(applyRule(pythagorean, target)).toEqual(int(1));

    const removePowerOne = rule(ctx.compilePattern(app(POW, [x, int(1)])), ctx.compilePattern(x));
    expect(rewrite(app(ADD, [app(POW, [sym("a"), int(1)]), app(POW, [sym("b"), int(1)])]), [removePowerOne]))
      .toEqual(app(ADD, [sym("a"), sym("b")]));
  });
});

describe("rule store", () => {
  it("stores, queries, removes, and clears named rules", () => {
    const store = new RuleStore();
    expect(store.size).toBe(0);
    expect(store.names()).toEqual([]);
    expect(store.contains("r1")).toBe(false);
    expect(store.get("r1")).toBeUndefined();

    const first = rule(sym("x"), int(1));
    const second = rule(sym("y"), int(2));
    store.store("r2", second);
    store.store("r1", first);
    expect(store.size).toBe(2);
    expect(store.contains("r1")).toBe(true);
    expect(store.get("r1")).toBe(first);
    expect(store.names()).toEqual(["r1", "r2"]);

    store.store("r1", second);
    expect(store.size).toBe(2);
    expect(store.get("r1")).toBe(second);

    store.remove("r2");
    store.remove("missing");
    expect(store.names()).toEqual(["r1"]);

    store.clear();
    expect(store.size).toBe(0);
    expect(store.names()).toEqual([]);
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
