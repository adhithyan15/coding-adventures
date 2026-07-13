/**
 * sir-runtime-symbolic — the SIR23 symbolic-expression + pattern/rewrite
 * runtime imported by Semantic-IR-emitted TypeScript/JavaScript, bound as
 * `__SirSym` at call sites (see `code/specs/SIR23-symbolic-pattern-semantic-ir.md`
 * §"Backend impact"). A compiled Wolfram/Macsyma/Maxima program's `SymApply`,
 * `SymPatternBlank`, `SymPatternNamed`, `SymRule`, and `SymReplaceAll` IR
 * nodes all become calls into this package at runtime.
 *
 * ## Why this package is thin
 *
 * The hard part — a term-tree type and a faithful structural pattern
 * matcher/substitution algorithm — already exists as two separate, already
 * *published* packages this one builds on rather than re-invents:
 *
 * - `@coding-adventures/symbolic-ir` — the term-tree type (`IRNode`: one of
 *   `Symbol`/`Integer`/`Rational`/`Float`/`String`/`Apply`) this whole domain
 *   is built on.
 * - `@coding-adventures/cas-pattern-matching` — a pure-TypeScript port of the
 *   Rust `cas-pattern-matching` crate's `Bindings`/`matchPattern`/
 *   `applyRule`/`substitute` (five-case structural matcher: `Blank()`,
 *   `Blank(T)`, `Pattern(name, inner)`, compound-vs-compound, and structural
 *   equality — see that package's own docs for the full algorithm).
 *
 * This package re-exports those primitives unchanged, and adds exactly the
 * two things neither sibling package has: (1) `replaceAll`/`replaceRepeated`
 * — the `/.`/`//.` tree-wide replacement operators SIR23 requires and
 * `cas-pattern-matching` doesn't expose as a matched pair (see "Two new
 * operations" below), and (2) an explicit recursion-depth cap on the parts
 * of this package that walk a full, potentially attacker-influenced runtime
 * expression tree (see "Depth safety" below).
 *
 * ## Two new operations, one existing one reused as-is
 *
 * `cas-pattern-matching` ships `rewrite()`, which already conflates "walk
 * bottom-up" with "retry every rule at each node until none fire" (a fixed
 * point) — that IS `replaceRepeated`'s exact contract, so `replaceRepeated`
 * below mirrors its algorithm closely (see the "Depth safety" note for why
 * it's a parallel implementation rather than a direct call). But
 * `cas-pattern-matching` has no equivalent of Wolfram's `/.` (`ReplaceAll`):
 * try each rule **once** per subtree, first match wins, no retry at that
 * position, no fixed point. `replaceAll` below is genuinely new code, not a
 * port of anything in the Rust or TypeScript reference — see its own doc
 * comment for the traversal-order choice.
 *
 * ## Depth safety
 *
 * `matchPattern`/`substitute`/`applyRule` (re-exported below, unmodified)
 * recurse, but their recursion depth is bounded by the *static* structure of
 * a single rule's pattern/RHS — authored by a compiler frontend or written
 * as a rule literal, not by runtime data — so they're already safe
 * regardless of how deep the *target* expression is: matching only
 * continues past a node when the *pattern* itself also has more `Apply`
 * structure at that exact position, and real patterns bottom out within a
 * handful of levels no matter how deep the value being matched against is.
 *
 * `replaceAll`/`replaceRepeated`, by contrast, walk the *entire* target
 * expression tree — ordinary runtime data a compiled program can build up
 * to unbounded depth (e.g. many rounds of nested computation) — so this is
 * the recursion that needs an explicit cap. `cas-pattern-matching`'s own
 * `rewrite()` has no such cap; carrying that gap forward into a runtime
 * meant to execute compiled, potentially attacker-influenced programs would
 * reopen the exact class of stack-overflow DoS this repo's other SIR passes
 * already guard against (`semantic-ir::limits::MAX_IR_DEPTH`, the
 * `semantic-ir` walker's depth-bounded `Visitor` default implementations).
 * `MAX_TERM_DEPTH` below is that cap, enforced by both new functions.
 */

import {
  app,
  equals,
  type IRApply,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  Bindings,
  BLANK,
  PATTERN,
  RULE,
  RULE_DELAYED,
  applyRule,
  blank,
  blankTyped,
  isBlank,
  isPattern,
  isRule,
  matchPattern,
  named,
  rule,
  ruleDelayed,
  substitute,
  type RewriteCycleError,
} from "@coding-adventures/cas-pattern-matching";

// ---------------------------------------------------------------------------
// Re-exported term-tree type and constructors (symbolic-ir)
// ---------------------------------------------------------------------------

export type { IRNode, IRApply };

/**
 * Build a symbolic-expression `Apply` node `head(args…)` — the runtime
 * representation of a `SymApply` IR node. Thin wrapper over symbolic-ir's
 * `app`, exposed under the name SIR23's backend-impact section uses
 * (`__SirSym.apply(head, args)`).
 */
export function apply(head: IRNode, args: readonly IRNode[]): IRNode {
  return app(head, args);
}

// ---------------------------------------------------------------------------
// Re-exported matcher/rewriter primitives (cas-pattern-matching, unmodified)
// ---------------------------------------------------------------------------

export {
  Bindings,
  BLANK,
  PATTERN,
  RULE,
  RULE_DELAYED,
  applyRule,
  blank,
  blankTyped,
  isBlank,
  isPattern,
  isRule,
  matchPattern,
  named,
  substitute,
  type RewriteCycleError,
};

/**
 * Build an eager-substitution rule `Rule(lhs, rhs)` — SIR23's `SymRule {
 * delayed: false }`, Wolfram `->`.
 *
 * **Current behavior note:** `rule` and {@link ruleDelayed} produce
 * data that is matched and substituted *identically* today — this package
 * has no general expression evaluator (per the SIR23 spec's own "Explicitly
 * out of scope" section, wiring the `cas-*` algorithm surface into this
 * runtime is separate, later work), so there is nothing yet for "eager"
 * (evaluate the RHS once, at rule-construction time) to actually evaluate.
 * The `delayed` bit still round-trips faithfully through the data model —
 * `rule`/`ruleDelayed` construct distinct `Rule`/`RuleDelayed` sentinel
 * heads — so a future PR that adds a real evaluator has a clean, already-
 * tested seam to branch on (see `rule-vs-rule-delayed.test.ts`). This
 * mirrors `cas-pattern-matching`'s own `ruleDelayed` doc comment exactly
 * ("reserved separately for future passes that distinguish evaluated vs.
 * held RHSes").
 */
export { rule };

/**
 * Build a delayed-substitution rule `RuleDelayed(lhs, rhs)` — SIR23's
 * `SymRule { delayed: true }`, Wolfram `:>`. See {@link rule}'s doc comment
 * for the current (identical-to-`rule`) behavior and what's deferred.
 */
export { ruleDelayed };

// ---------------------------------------------------------------------------
// Depth guard
// ---------------------------------------------------------------------------

/**
 * Maximum recursion depth for {@link replaceAll}/{@link replaceRepeated}'s
 * tree walk. See the module doc comment's "Depth safety" section for why
 * only these two functions (not the re-exported matcher primitives) need
 * this cap. Chosen to be far higher than any realistic compiled expression
 * tree, while staying comfortably under typical JS engine stack limits —
 * mirrors `semantic-ir::limits::MAX_IR_DEPTH`'s "two orders of magnitude
 * below the empirical overflow point" rationale, scaled down for JS's
 * generally shallower default call stacks and this walk's heavier
 * per-frame cost (each frame also holds a `rules` scan).
 */
export const MAX_TERM_DEPTH = 512;

/** Returned by {@link replaceAll}/{@link replaceRepeated} when the walk's
 * recursion depth would exceed {@link MAX_TERM_DEPTH}. */
export interface DepthLimitError {
  readonly kind: "depth-limit";
  readonly maxDepth: number;
}

export function isDepthLimitError(value: unknown): value is DepthLimitError {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as { kind?: unknown }).kind === "depth-limit"
  );
}

/**
 * Local, broadened counterpart to `cas-pattern-matching`'s own
 * `isRewriteCycleError` (whose parameter type is `IRNode | RewriteCycleError`
 * — too narrow for this module's three-way `IRNode | RewriteCycleError |
 * DepthLimitError` return unions). Behaviorally identical structural check.
 */
export function isRewriteCycleError(value: unknown): value is RewriteCycleError {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as { kind?: unknown }).kind === "rewrite-cycle"
  );
}

function isErrorResult(
  value: IRNode | RewriteCycleError | DepthLimitError,
): value is RewriteCycleError | DepthLimitError {
  return isRewriteCycleError(value) || isDepthLimitError(value);
}

// ---------------------------------------------------------------------------
// replaceAll — `expr /. rules`, one pass
// ---------------------------------------------------------------------------

/**
 * `expr /. rules` — Wolfram's `ReplaceAll`, one pass over the whole tree.
 *
 * Walks `expr` **bottom-up** (post-order: a node's `head` and every `args`
 * element are visited — and possibly replaced — before the node itself is
 * tried against `rules`). This matches `cas-pattern-matching`'s own
 * `rewrite()` traversal order (see this package's PR, which also corrects
 * SIR23 spec §"Matcher semantics" point 1 — that prose said "top-down",
 * which never matched what `rewrite()` actually does).
 *
 * At each subtree (after its children are already finalized), `rules` are
 * tried **in order**; the first structural match wins, and its substituted
 * replacement takes that subtree's place. Unlike {@link replaceRepeated},
 * each subtree is visited and tried **exactly once** — a freshly-substituted
 * replacement is not re-walked or retried against `rules` at the same
 * position, matching Wolfram's `/.` (single-pass) contract exactly.
 *
 * Always terminates in a single bounded walk over `expr`'s existing node
 * count — there is no fixed-point loop, so (unlike {@link replaceRepeated})
 * there is no `maxIterations` parameter and no `RewriteCycleError` outcome.
 * The only failure mode is {@link MAX_TERM_DEPTH}.
 *
 * @example
 * ```ts
 * // x_ + 0  ->  x_   applied once, everywhere in the tree
 * const xPat = named("x", blank());
 * const r = rule(apply(ADD, [xPat, int(0)]), xPat);
 * const expr = apply(ADD, [apply(ADD, [sym("z"), int(0)]), int(0)]);
 * replaceAll(expr, [r]); // => sym("z")  (both `+ 0`s fire, one pass each)
 * ```
 */
export function replaceAll(
  expr: IRNode,
  rules: readonly IRNode[],
): IRNode | DepthLimitError {
  return walkOnce(expr, rules, 0);
}

function walkOnce(
  node: IRNode,
  rules: readonly IRNode[],
  depth: number,
): IRNode | DepthLimitError {
  if (depth > MAX_TERM_DEPTH) {
    return { kind: "depth-limit", maxDepth: MAX_TERM_DEPTH };
  }

  let current = node;
  if (node.kind === "apply") {
    const newHead = walkOnce(node.head, rules, depth + 1);
    if (isDepthLimitError(newHead)) return newHead;
    const newArgs: IRNode[] = [];
    for (const arg of node.args) {
      const nextArg = walkOnce(arg, rules, depth + 1);
      if (isDepthLimitError(nextArg)) return nextArg;
      newArgs.push(nextArg);
    }
    current = apply(newHead, newArgs);
  }

  for (const candidateRule of rules) {
    const replacement = applyRule(candidateRule, current);
    if (replacement !== null) {
      return replacement; // first match wins; no retry at this position
    }
  }
  return current;
}

// ---------------------------------------------------------------------------
// replaceRepeated — `expr //. rules`, fixed point
// ---------------------------------------------------------------------------

/**
 * `expr //. rules` — Wolfram's `ReplaceRepeated`, a fixed point.
 *
 * Like {@link replaceAll}, walks bottom-up, but at each subtree keeps
 * retrying `rules` until none fire, re-walking any fresh replacement (so
 * *its* sub-parts also converge) before moving up to the parent. This is
 * `cas-pattern-matching`'s own `rewrite()` algorithm — reimplemented here
 * (calling the same `applyRule`/`equals` primitives `rewrite()` itself
 * uses) rather than called directly, because `rewrite()` has no
 * recursion-depth parameter to hook a cap into (see the module doc
 * comment's "Depth safety" section). Otherwise the algorithm is identical:
 * bottom-up traversal, per-node local fixed point, and a **global**
 * `maxIterations` cap shared across the whole walk (not per-node) — the
 * same cap shape `rewrite()` uses, returning {@link RewriteCycleError} on
 * the same terms.
 *
 * SIR23 §"Matcher semantics" point 6 requires every backend to enforce an
 * iteration cap here ("an unbounded `//.` is a guaranteed non-terminating
 * program for some inputs"); `maxIterations` (default 100, matching
 * `rewrite()`'s own default) is that cap. {@link MAX_TERM_DEPTH} is the
 * *additional* recursion-depth cap this package adds beyond what
 * `rewrite()` itself enforces (see "Depth safety").
 *
 * **A caller-facing nuance worth calling out explicitly** (an existing
 * property of the ported algorithm, not something new this package
 * introduces): each time a rule fires, the local retry loop below recurses
 * into `walk(replacement, depth)` — one native JS stack frame per firing,
 * same as `rewrite()`'s own `cur = walk(replacement, ...)` — so `maxIterations`
 * itself bounds a SEPARATE recursion (retry count), distinct from
 * {@link MAX_TERM_DEPTH}'s tree-structural one. A caller that passes a very
 * large `maxIterations` (well beyond the "well-behaved rules converge in
 * 2-5 passes" norm this default assumes) could still exhaust the stack via
 * that path before `MAX_TERM_DEPTH` or `maxIterations` itself is reached.
 * Callers should keep `maxIterations` modest — SIR23's own point 6 already
 * places this choice on the backend/caller, not this package.
 *
 * @example
 * ```ts
 * // x_ + 0  ->  x_   applied repeatedly, to a fixed point
 * const xPat = named("x", blank());
 * const r = rule(apply(ADD, [xPat, int(0)]), xPat);
 * const expr = apply(ADD, [apply(ADD, [sym("z"), int(0)]), int(0)]);
 * replaceRepeated(expr, [r], 100); // => sym("z")
 * ```
 */
export function replaceRepeated(
  expr: IRNode,
  rules: readonly IRNode[],
  maxIterations = 100,
): IRNode | RewriteCycleError | DepthLimitError {
  let counter = 0;

  const walk = (
    node: IRNode,
    depth: number,
  ): IRNode | RewriteCycleError | DepthLimitError => {
    if (depth > MAX_TERM_DEPTH) {
      return { kind: "depth-limit", maxDepth: MAX_TERM_DEPTH };
    }

    let current = node;
    if (node.kind === "apply") {
      const newHead = walk(node.head, depth + 1);
      if (isErrorResult(newHead)) return newHead;
      const newArgs: IRNode[] = [];
      for (const arg of node.args) {
        const nextArg = walk(arg, depth + 1);
        if (isErrorResult(nextArg)) return nextArg;
        newArgs.push(nextArg);
      }
      current = apply(newHead, newArgs);
    }

    while (true) {
      let fired = false;
      for (const candidateRule of rules) {
        const replacement = applyRule(candidateRule, current);
        if (replacement !== null && !equals(replacement, current)) {
          counter += 1;
          if (counter > maxIterations) {
            return { kind: "rewrite-cycle", maxIterations };
          }
          // Re-walk the replacement at the SAME depth: this substitutes a
          // new value at the current position, it does not descend to a
          // child, so the depth budget for this position is unchanged.
          const walked = walk(replacement, depth);
          if (isErrorResult(walked)) return walked;
          current = walked;
          fired = true;
          break;
        }
      }
      if (!fired) return current;
    }
  };

  return walk(expr, 0);
}
