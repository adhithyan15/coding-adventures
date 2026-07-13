# @coding-adventures/sir-runtime-symbolic

The **symbolic-expression + pattern/rewrite runtime** imported by
Semantic-IR-emitted TypeScript/JavaScript for the CAS math-language domain
(Wolfram/Macsyma/Maxima), per
[`code/specs/SIR23-symbolic-pattern-semantic-ir.md`](../../../specs/SIR23-symbolic-pattern-semantic-ir.md).

## What it is

SIR23 adds five new `Expr` variants to Semantic IR — `SymApply`,
`SymPatternBlank`, `SymPatternNamed`, `SymRule`, `SymReplaceAll` — so a
compiled Wolfram/Macsyma program can still pattern-match and rewrite terms
**at runtime**, not just lower a precomputed answer. This package is what
those nodes compile down to: it's bound as `__SirSym` at the emitted call
sites (`__SirSym.apply(...)`, `__SirSym.replaceAll(...)`,
`__SirSym.replaceRepeated(...)`).

It is intentionally thin — the term-tree type and the structural matcher
algorithm already exist as two separate published packages:

| Package | What it provides |
|---|---|
| `@coding-adventures/symbolic-ir` | The `IRNode` term-tree type (`Symbol`/`Integer`/`Rational`/`Float`/`String`/`Apply`) |
| `@coding-adventures/cas-pattern-matching` | A faithful TypeScript port of the Rust `cas-pattern-matching` crate's five-case structural matcher (`Blank()`/`Blank(T)`/`Pattern(name, inner)`/compound/structural-equality), `substitute`, and `applyRule` |

This package re-exports those primitives unchanged and adds exactly the two
things neither sibling provides:

1. **`replaceAll`/`replaceRepeated`** — the `/.`/`//.` tree-wide replacement
   operators SIR23 requires. `cas-pattern-matching` ships `rewrite()`, which
   already *is* `replaceRepeated`'s algorithm (bottom-up, per-node fixed
   point, global iteration cap) — but has no equivalent of Wolfram's `/.`
   (try each rule once per subtree, no retry, no fixed point). `replaceAll`
   is genuinely new code here, not a port of anything upstream.
2. **An explicit recursion-depth cap** (`MAX_TERM_DEPTH`, default 512) on
   the full-tree walk both operations perform. `cas-pattern-matching`'s own
   `rewrite()` has no depth guard at all; since this package runs on
   compiled, potentially deeply-nested runtime expressions (not just
   short, hand-authored rule literals), it adds one rather than carrying
   that gap forward — see the module doc comment in `src/index.ts` for why
   the re-exported matcher primitives *don't* need the same treatment.

## How emitted code uses it

```ts
import * as __SirSym from "@coding-adventures/sir-runtime-symbolic";

// x_ + 0 -> x_   (a rule compiled from Wolfram `x_ + 0 -> x_`)
const xPat = __SirSym.named("x", __SirSym.blank());
const dropAddZero = __SirSym.rule(
  __SirSym.apply(ADD, [xPat, int(0)]),
  xPat,
);

const result = __SirSym.replaceAll(expr, [dropAddZero]);
if (__SirSym.isDepthLimitError(result)) {
  throw new Error(`expression too deep: exceeds ${result.maxDepth} levels`);
}
```

## Current scope: matching and substitution only, no evaluator

`SymRule { delayed }` faithfully carries Wolfram's `Rule` (`->`) vs.
`RuleDelayed` (`:>`) distinction through the data model — but this package
has **no general expression evaluator** yet (wiring the `cas-*` algorithm
surface, e.g. `Expand`/`Factor`/`Solve`, into a JS runtime is separate,
later work per the SIR23 spec's own "Explicitly out of scope" section). So
today, `rule` and `ruleDelayed` match and substitute **identically** — the
distinction (evaluate the RHS once at construction vs. fresh per match) only
starts to matter once a real evaluator exists to do that evaluating. The
`delayed` flag round-trips through the data so that future work has a clean
seam to extend; see `tests/sir-runtime-symbolic.test.ts`'s
`rule vs ruleDelayed` suite for the locked-in current contract.

## Where it fits

`wolfram-to-semantic-ir` / `macsyma-to-semantic-ir` / `maxima-to-semantic-ir`
→ `semantic-ir` (SIR23 `Expr` variants) → `semantic-ir-to-typescript` /
`semantic-ir-to-javascript` → emitted code that imports this package. See
[`code/specs/HML01-math-to-semantic-ir.md`](../../../specs/HML01-math-to-semantic-ir.md).

## Development

```sh
npm install
npx tsc --noEmit
npx vitest run
```
