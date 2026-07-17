# j-to-semantic-ir

J CST → narrow-waist Semantic IR. Task **MA-6e** — the last remaining
rollout item for J (see [MA06](../../../specs/MA06-j-language.md) §6/§7).
J is APL's ASCII-respelled descendant, and this crate is built directly on
[apl-to-semantic-ir](../apl-to-semantic-ir)'s design — same SIR22 base cut,
same SIR22 "APL addendum" (`Reduce`/`Scan`/`Shape`/`Reshape`/
`IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`), same lowering idioms. J adds
exactly one genuinely new production APL never had: **trains** (`(f g)`
hooks, `(f g h)`/`(n g h)` forks, `f@g` compose), lowering to nested
applications of the same node types rather than any new SIR node.

## Where this fits

```
J source
   │
   ▼  coding_adventures_j_parser::try_parse_j(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  j_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR22 + SIR22 addendum)
```

This crate depends only on `coding-adventures-j-parser` (the CST), **not**
`coding-adventures-j-runtime` (the tree-walking evaluator) — mirrors
`apl-to-semantic-ir`'s identical choice.

## Usage

```rust
use j_to_semantic_ir::compile_source;

let module = compile_source("a=.3+4\n", "demo")?;
```

## Scope (v0.1.0)

Everything `apl-to-semantic-ir` supports transfers directly (J's
`noun_expr`/`term`/`assignment` are structurally identical to APL's
`value_expr`/`term`/`assignment`, MA06 §3): number literals, stranded
literals, variables, parenthesised grouping, chained assignment, all 12
scalar dyadic atoms (unconditionally `ElementwiseOp`, no scalar/array
disambiguation needed), `$`/`i.`/`,`, `/`/`\` (reduce/scan, monadic-only),
and auto-print.

Two things are genuinely new relative to APL:

### `#` and `^` — no APL analogue

`array_runtime::ops::BinOp` (what the 12 shared atoms map onto) has no
`Pow` variant, and `#`'s monadic (tally)/dyadic (replicate) meanings are
unrelated structural-array operations — `j-runtime::eval::JFn` categorises
both as bespoke non-scalar verbs, and this lowerer mirrors that exactly:

| Verb | Monadic | Dyadic |
|---|---|---|
| `#` | `BuiltinCall("tally", [target])` | `BuiltinCall("replicate", [counts, data])` |
| `^` | `BuiltinCall("exp", [target])` | `Expr::ElementwiseOp { op: Pow, .. }` |

`^`'s dyadic form reuses `ElementwiseOpKind::Pow` (already in SIR22, added
for MATLAB's `.^`, unused by APL) even though `^` is still classified as a
bespoke non-scalar verb (not folded into the same bucket as the 12 shared
atoms) — this is what correctly excludes `^` from reduce/scan eligibility,
matching `j-runtime::eval::require_scalar_binop`'s real restriction, so
this frontend's accepted surface stays in lockstep with the reference
interpreter (the point of the oracle-testing convention HML01's
Verification section describes).

### Trains: `(f g)` hooks, `(f g h)`/`(n g h)` forks, `f@g` compose

No new SIR node — each combinator lowers to nested `ElementwiseOp`/
`BuiltinCall`/etc. applications, per MA06 §5's own explicit instruction.
See `src/lower.rs`'s module doc comment for the exact formulas and the
4+-tooth peel-from-the-left folding rule (`(a b c d)` = `(a (b c d))`).

**A dedicated, much smaller depth guard.** A hook or verb-left fork
duplicates its noun operand(s) in the emitted `Expr` tree (this lowerer
builds owned expression trees, not values — unlike a real interpreter,
which evaluates an operand once and cheaply reuses the resulting value,
this lowerer must `.clone()` an already-lowered subtree to use it twice).
That duplication compounds through three distinct mechanisms that all
share one counter and cap: folding a wide single train, descending into
an explicitly nested parenthesised sub-train, *and* a chain of separately
parenthesised hooks joined by ordinary application (`(f g)(h i)...base`)
— `N` combinator levels, reached through any mixture of the three, bound
the worst case at `2^N` duplicated copies of whatever sits at the bottom,
entirely independent of (and far tighter than) the general
expression-depth guard. `MAX_TRAIN_COMBINATOR_DEPTH` (`12`) bounds this;
a security review caught an earlier draft accumulating it correctly for
the first two mechanisms but resetting it to `0` per application link for
the third, letting a chain of small (individually within-cap) hooks
bypass the cap entirely — confirmed to blow up to hundreds of megabytes
from an under-100-byte source before the fix. See `src/lower.rs`'s module
doc comment's "Why trains get their own, much smaller depth guard"
section for the full reasoning and all three mechanisms.

## Well-known `BuiltinCall` names

`+` (conjugate) is a pass-through no-op, exactly like APL's. `-`/`*`/`%`/
`<.`/`>.` map onto `"neg"`/`"sign"`/`"recip"`/`"floor"`/`"ceil"` — the
*exact* names `apl-to-semantic-ir` already introduced. `#`/`^` introduce
`"tally"`/`"replicate"`/`"exp"`, new to this crate.

### Testing

- `tests/test_lower.rs` — 43 tests covering every dyadic atom, monadic
  atoms (valid + rejected comparisons), `$`/`i.`/`,`/`#`/`^` (monadic and
  dyadic), reduce/scan (valid + rejected dyadic use + rejected non-scalar
  verbs), compose (monadic + dyadic), hooks (monadic + dyadic + rejected
  bare-noun tooth), forks (verb-left and leading-noun, monadic + dyadic +
  rejected bare-noun in a non-leading position), 4+-tooth train folding,
  the combinator-depth cap (a rejected too-deep single train, a rejected
  too-deep *chain* of separately-parenthesised hooks — the security-review
  regression — two accepted within-cap cases, and a long chain of
  non-duplicating verbs confirming the cap tracks actual duplication risk
  rather than raw chain length), stranded/underscore-negative literals,
  parenthesised grouping, chained assignment,
  first-occurrence-vs-reassignment, undefined-variable rejection,
  parse-error propagation, and a full multi-line program that validates
  via `semantic_ir::validate`.
- `tests/test_validator.rs` — mirrors `apl-to-semantic-ir`'s own
  capability-rejection pattern: `semantic-ir-to-javascript` accepts
  base-cut modules (including hook/fork-using ones, which are ordinary
  nested base-cut applications) and rejects `Reduce`-using ones via its
  dedicated tree-walk (not the plain feature-flag check, which can no
  longer distinguish the two).
