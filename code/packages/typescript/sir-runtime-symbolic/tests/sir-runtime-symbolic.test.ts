import { describe, expect, it } from "vitest";
import { ADD, app, int, sym } from "@coding-adventures/symbolic-ir";
import {
  Bindings,
  DepthLimitError,
  MAX_TERM_DEPTH,
  RewriteCycleError,
  apply,
  applyRule,
  blank,
  isDepthLimitError,
  isRewriteCycleError,
  matchPattern,
  named,
  replaceAll,
  replaceRepeated,
  rule,
  ruleDelayed,
  substitute,
} from "../src/index";

// Reusable "x_ + 0 -> x_" identity-elimination rule, the running example in
// this package's own doc comments and the Rust/TS cas-pattern-matching
// crates' own examples.
const xPat = named("x", blank());
const dropAddZero = rule(app(ADD, [xPat, int(0)]), xPat);

describe("re-exported matcher/rewriter primitives", () => {
  it("matchPattern, applyRule, substitute, Bindings all work through this package's surface", () => {
    const bindings = matchPattern(named("a", blank()), int(5), Bindings.empty());
    expect(bindings?.get("a")).toEqual(int(5));
    expect(applyRule(dropAddZero, app(ADD, [sym("z"), int(0)]))).toEqual(sym("z"));
    expect(substitute(xPat, Bindings.empty().bind("x", int(9)))).toEqual(int(9));
  });

  it("apply() is a thin wrapper producing the same node as symbolic-ir's app()", () => {
    expect(apply(ADD, [int(1), int(2)])).toEqual(app(ADD, [int(1), int(2)]));
  });
});

describe("replaceAll (/. — one pass)", () => {
  it("fires once per matching subtree, everywhere in the tree", () => {
    // Add(Add(z, 0), 0) -- both "+ 0"s should be eliminated in one call.
    const expr = app(ADD, [app(ADD, [sym("z"), int(0)]), int(0)]);
    expect(replaceAll(expr, [dropAddZero])).toEqual(sym("z"));
  });

  it("does not retry a rule against its own freshly-substituted replacement", () => {
    // A rule whose RHS is itself a fresh match candidate for the SAME rule:
    // f(a) -> f(f(a)). A single pass must fire exactly once at the root
    // (producing f(f(a))) and must NOT loop forever retrying at that spot --
    // that's the one behavior that actually distinguishes replaceAll from
    // replaceRepeated (which would need a cycle-detecting cap to survive
    // this same rule).
    const f = sym("f");
    const selfWrap = rule(app(f, [xPat]), app(f, [app(f, [xPat])]));
    const result = replaceAll(app(f, [sym("a")]), [selfWrap]);
    expect(result).toEqual(app(f, [app(f, [sym("a")])]));
  });

  it("walks bottom-up: a child's rewrite is visible to a rule tried at the parent", () => {
    // g(Add(z, 0)) with dropAddZero -- if children are rewritten first
    // (bottom-up), the parent sees g(z). A hypothetical rule targeting
    // exactly g(z) can then fire in the SAME pass, proving traversal order.
    const g = sym("g");
    const gOfZ = rule(app(g, [sym("z")]), sym("matched-g-of-z"));
    const expr = app(g, [app(ADD, [sym("z"), int(0)])]);
    expect(replaceAll(expr, [dropAddZero, gOfZ])).toEqual(sym("matched-g-of-z"));
  });

  it("leaves a non-matching tree unchanged", () => {
    const expr = app(ADD, [sym("a"), sym("b")]);
    expect(replaceAll(expr, [dropAddZero])).toEqual(expr);
  });
});

describe("replaceRepeated (//. — fixed point)", () => {
  it("matches replaceAll on a single-firing-per-subtree case", () => {
    const expr = app(ADD, [app(ADD, [sym("z"), int(0)]), int(0)]);
    expect(replaceRepeated(expr, [dropAddZero], 100)).toEqual(sym("z"));
  });

  it("keeps applying until a true fixed point, unlike replaceAll", () => {
    // Add(Add(Add(z, 0), 0), 0) -- three nested "+0"s. A single replaceAll
    // pass only guarantees each EXISTING subtree is tried once, but since
    // this walks bottom-up the innermost eliminations are already visible
    // by the time outer ones are tried, so replaceAll happens to also
    // reach the fixed point here. The real distinguishing case is
    // `does not retry a rule against its own freshly-substituted
    // replacement`'s sibling test below, which replaceRepeated finishes
    // (further) instead of stopping after one substitution.
    const expr = app(ADD, [app(ADD, [app(ADD, [sym("z"), int(0)]), int(0)]), int(0)]);
    expect(replaceRepeated(expr, [dropAddZero], 100)).toEqual(sym("z"));
  });

  it("reports RewriteCycleError instead of hanging on a non-terminating rule set", () => {
    // f(a) -> f(f(a)) retried forever would never reach a fixed point.
    const f = sym("f");
    const neverConverges = rule(app(f, [xPat]), app(f, [app(f, [xPat])]));
    const result = replaceRepeated(app(f, [sym("a")]), [neverConverges], 50);
    expect(isRewriteCycleError(result)).toBe(true);
    if (isRewriteCycleError(result)) {
      expect(result.maxIterations).toBe(50);
    }
  });

  it("leaves a non-matching tree unchanged", () => {
    const expr = app(ADD, [sym("a"), sym("b")]);
    expect(replaceRepeated(expr, [dropAddZero], 100)).toEqual(expr);
  });

  it("survives a huge maxIterations on a non-converging, non-deepening rule set without a stack overflow", () => {
    // Isolates the exact bug this function's retry loop was rewritten to
    // fix: a -> b, b -> a cycles forever WITHOUT ever building deeper tree
    // structure (both sides are bare symbols, never an Apply), so
    // MAX_TERM_DEPTH's tree-descent check never even triggers here -- if
    // the retry step still recursed once per firing (the earlier,
    // /security-review-flagged design), 50,000 firings would mean 50,000
    // nested native stack frames and a real stack overflow. The current
    // (loop-based-retry) implementation costs O(1) stack per firing
    // regardless, so this resolves cleanly to RewriteCycleError instead.
    const a = sym("a");
    const b = sym("b");
    const aToB = rule(a, b);
    const bToA = rule(b, a);
    const result = replaceRepeated(a, [aToB, bToA], 50_000);
    expect(isRewriteCycleError(result)).toBe(true);
    if (isRewriteCycleError(result)) {
      expect(result.maxIterations).toBe(50_000);
    }
  });
});

describe("depth guard (MAX_TERM_DEPTH)", () => {
  // Build a right-leaning chain f(f(f(...f(leaf)...))) `depth` levels deep.
  function deepChain(depth: number): ReturnType<typeof app> {
    const f = sym("f");
    let node: ReturnType<typeof app> | ReturnType<typeof sym> = sym("leaf");
    for (let i = 0; i < depth; i += 1) {
      node = app(f, [node]);
    }
    return node as ReturnType<typeof app>;
  }

  it("replaceAll succeeds on a tree within the cap", () => {
    const shallow = deepChain(MAX_TERM_DEPTH - 10);
    const result = replaceAll(shallow, []);
    expect(isDepthLimitError(result)).toBe(false);
  });

  it("replaceAll reports DepthLimitError (not a stack overflow) past the cap", () => {
    const tooDeep = deepChain(MAX_TERM_DEPTH * 4);
    const result = replaceAll(tooDeep, []);
    expect(isDepthLimitError(result)).toBe(true);
    if (isDepthLimitError(result)) {
      expect(result.maxDepth).toBe(MAX_TERM_DEPTH);
    }
  });

  it("replaceRepeated reports DepthLimitError (not a stack overflow) past the cap", () => {
    const tooDeep = deepChain(MAX_TERM_DEPTH * 4);
    const result = replaceRepeated(tooDeep, [], 100);
    expect(isDepthLimitError(result)).toBe(true);
  });

  it("isDepthLimitError and isRewriteCycleError do not cross-match each other's shape", () => {
    const depthErr: DepthLimitError = { kind: "depth-limit", maxDepth: MAX_TERM_DEPTH };
    const cycleErr: RewriteCycleError = { kind: "rewrite-cycle", maxIterations: 100 };
    expect(isDepthLimitError(depthErr)).toBe(true);
    expect(isDepthLimitError(cycleErr)).toBe(false);
    expect(isRewriteCycleError(cycleErr)).toBe(true);
    expect(isRewriteCycleError(depthErr)).toBe(false);
    expect(isDepthLimitError(int(1))).toBe(false);
    expect(isRewriteCycleError(int(1))).toBe(false);
  });
});

describe("rule vs ruleDelayed", () => {
  it("currently match and substitute identically (no evaluator wired in yet)", () => {
    const eager = rule(app(ADD, [xPat, int(0)]), xPat);
    const delayed = ruleDelayed(app(ADD, [xPat, int(0)]), xPat);
    const target = app(ADD, [sym("z"), int(0)]);

    expect(applyRule(eager, target)).toEqual(applyRule(delayed, target));
    expect(replaceAll(target, [eager])).toEqual(replaceAll(target, [delayed]));
    expect(replaceRepeated(target, [eager], 10)).toEqual(replaceRepeated(target, [delayed], 10));
  });
});
