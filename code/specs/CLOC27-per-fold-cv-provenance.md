# CLOC27 — Per-Fold CV Provenance Through the SIMPLE Bridge

## What this spec locks down

"A Closure-Compiler clone **with tracing**" is the project's headline
differentiator: every transformation the optimizer makes should be auditable
back to the source bytes it came from, via the correlation vector (CV). The
constant-fold pass already does its part — each fold calls `fork_cv` /
`stamp_literal_cv` to record that a new literal *derives from* its input nodes.
But today that lineage **dead-ends at the bridge boundary**: the typed-AST nodes
the bridge produces carry `cv: None`, so a folded `3` from `"abc".length` has a
CV that derives from *nothing* — there is no link back to the `"abc".length`
source span.

This spec defines how to close that gap on the SIMPLE pipeline, and pins the
behaviour with a golden trace test. It builds directly on
[CLOC03](CLOC03-correlation-vector-plumbing.md) (the CV plumbing) and is the
implementation counterpart to the characterization test added for the gap
(`closurec/tests/cv_fold_provenance_gap.rs`, the "cv-fold-gap" finding).

The invariant it must satisfy is CLOC03's, applied to the optimizer:

> Every literal the optimizer emits is reachable, through the CV log, from at
> least one root `Origin` describing the source token(s) it came from. A fold
> that produces a literal with no path to a source origin is a bug.

## The gap, concretely

For the input `report("abc".length);` at `--compilation_level SIMPLE
--correlation_vector`:

```
                         ┌─────────────────────────── today: cv: None ───────────┐
  "abc".length    ──parse──►  StringLiteral{cv:None} . length   ──fold──►  3{cv: derive(None)}
  (source bytes)                                                            └─ origin: nothing
```

The folded `3` *does* get a fresh CvId, and the fold *does* record a
contribution ("constant-fold rewrote `"abc".length` → `3`"). But because the
`StringLiteral` it derived from had `cv: None`, the chain has no root — the
sidecar shows only coarse lex/file/pass-summary origins, never the `"abc".length`
span. Assertion (2) of the cv-fold-gap characterization test pins exactly this:
every entry's `origin.source` is lex/file-level (`lexer_token`, `input_file`,
`js_output_file`, `concatenated_combined_source`); none ties the `3` to its
source.

## Key finding: the CvIds already exist — this is propagation, not minting

Investigation of the parser pipeline (origin/main, 2026-06) established that
**per-token CvIds are already minted** and then **discarded** before they can
reach the bridge:

1. `tokenize_javascript_with_cv` (CLOC03) already mints a CvId per token, each
   with a root `Origin{source, location}`. This is the `cv` on each `CvToken`.
2. `parse_javascript_with_cv` (`javascript-parser/src/lib.rs`) **strips** those
   ids: `let tokens = cv_tokens.into_iter().map(|t| t.token).collect();` — only
   the bare `lexer::token::Token` (which has no `cv` field) flows into the
   parser, so the `GrammarASTNode` the bridge walks has lost every token CvId.
   The `token_cv_ids` are kept in a *separate* vector, used only to compute the
   program-root CV by `merge`.
3. The SIMPLE optimization path (`closurec/src/run.rs`, `transform_source_with_cv`)
   parses with `parse_javascript_typed`, which has **no CV plumbing at all** —
   so even the separate `token_cv_ids` vector does not exist on that path.

So the leaf literals never had a chance to carry provenance: the ids exist, but
they are thrown away before the bridge and the SIMPLE path does not request them.
The fix is to **propagate the existing per-token CvId onto the leaf literal** —
not to mint anything new in the bridge.

This is important because it avoids the naive alternative — threading a mutable
`CVLog` (or a position→id map) as a parameter through all ~51 `convert_*` bridge
functions and their ~183 internal call sites. The CvId rides on the token
itself, so only the single leaf-factory function needs to change.

## Where the leaves are born

Every leaf literal in the typed AST is created in exactly one bridge function:

```
fn convert_primary_token(t: &Token, ctx: &GrammarASTNode) -> Result<Expression, BridgeError>
```

(`javascript-parser/src/bridge.rs`). It is the sole factory for `NullLiteral`,
`UndefinedLiteral`, `BooleanLiteral`, `BigIntLiteral`, `NumericLiteral`,
`StringLiteral`, and `Identifier` — eight `cv: None` return sites. It already
receives the token `t` (with `line`/`column`) and the node `ctx`. If `t` carries
its CvId, this one function is the only place that needs to read it.

## Design

```
  source ─► tokenize_with_cv ─► token carries cv ─► parser (cv rides on token)
                                                          │
                                                          ▼
   typed AST leaf {cv: Some(token_cv)} ◄── convert_primary_token stamps t.cv
                                                          │
                                          fold: derive(leaf.cv, …) ─► folded literal
                                                          │
                                          sidecar: folded 3 ──ancestors──► "abc".length token Origin
```

### D1 — `lexer::token::Token` carries an optional CvId

Add `pub cv: Option<CvId>` (where `CvId = String`, re-exported from
`correlation-vector` / `javascript-ast`) to `lexer::token::Token`
(`lexer/src/token.rs`). Default `None`. There are ~49 struct-literal
construction sites across the lexer and parser; each gains `cv: None` (or the
struct derives `Default` and call sites adopt `..Default::default()` where
ergonomic). The non-CV tokenizer leaves it `None`; only the CV tokenizer sets it.

This is deliberately a plain field on the existing token type rather than a
parallel side-channel: the token is the natural carrier of its own provenance,
and keeping the id *on* the token is what lets the bridge stay oblivious to CV
plumbing (no threaded context).

### D2 — stop stripping the CvId before the parser

In `parse_javascript_with_cv` (and a new typed variant, D3), set each token's
`cv` from its `CvToken.cv` instead of discarding it:

```rust
let tokens: Vec<Token> = cv_tokens
    .into_iter()
    .map(|t| Token { cv: Some(t.cv), ..t.token })
    .collect();
```

The parser does not inspect `cv`; it copies tokens into `GrammarASTNode`
children unchanged, so the id arrives at the bridge intact.

### D3 — a CV-carrying typed parse entry for the SIMPLE path

Add `parse_javascript_typed_with_cv(source, source_file, version, cv: &mut CVLog)`
mirroring `parse_javascript_typed` but routing through the CV tokenizer (D2). The
existing `parse_javascript_typed` stays as the zero-overhead default.

### D4 — stamp the leaf in `convert_primary_token`

Replace the eight `cv: None` literal returns with `cv: t.cv.clone()`. When the
token carries no id (non-CV path), this is `None` — **byte-identical to today**,
so every existing test passes unchanged. When CV is on, the leaf now carries its
source token's CvId, whose `Origin` is the source span.

### D5 — wire the run's CVLog on the SIMPLE path

In `closurec/src/run.rs` (`transform_source_with_cv`, ~line 511), when
`--correlation_vector` is set, parse via `parse_javascript_typed_with_cv` with
the run's real (enabled) `CVLog`. The fold pass already runs against that log;
its `fork_cv` now `derive`s the folded literal from a leaf id that has a real
source `Origin`, so the chain reaches the source. The non-CV path is unchanged.

## Soundness & zero-behaviour-change

* **No new minting in the bridge.** The bridge stamps an id that already exists;
  it never creates CV state. The bridge stays a pure `GrammarASTNode → Program`
  transform.
* **The disabled path is identical to today.** A token with `cv: None` yields a
  leaf with `cv: None`, exactly as now. The whole feature is gated behind the
  CV tokenizer being used, which only happens under `--correlation_vector`.
* **No emitter change.** CV ids never appear in emitted JS; they live only in the
  sidecar. Output bytes are unaffected on every path.

## Test plan

1. **Golden trace test** (`closurec/tests/cv_fold_trace.rs`, new): run
   `report("abc".length);` at `SIMPLE --correlation_vector`; assert the folded
   `3`'s CV entry has an ancestor whose `origin.source` is the per-token source
   (the source file), and whose `location` is the `"abc".length` span — i.e. the
   `3` is traceable to the bytes it came from.
2. **Flip the gap characterization** (`cv_fold_provenance_gap.rs`): assertion (2)
   currently requires *all* origins be lex/file-level. Once D1–D5 land, the
   folded literal's ancestor origin is a real source span, so the gap no longer
   holds — update the test to assert the **presence** of per-token lineage on the
   folded node instead of its absence. That test flipping is the signal tracing
   became real.
3. **CV-summary count tests:** the four existing `cv-summary` count assertions
   gain leaf-origin entries; update the expected counts for the new blast radius.
4. **Regression:** the full `lexer`, `javascript-parser`, and `closurec` suites
   pass unchanged on the non-CV path (D4 disabled path is byte-identical).

## PR breakdown (leaf-first, each independently green)

| PR  | Scope | Risk |
|-----|-------|------|
| P1  | D1 — `Token.cv` field + ~49 ctor sites (`cv: None`), no readers yet | mechanical, no behaviour change |
| P2  | D2+D3 — stop stripping; `parse_javascript_typed_with_cv` | additive, default path untouched |
| P3  | D4 — stamp `t.cv` in `convert_primary_token` | 1 function, disabled path identical |
| P4  | D5 — wire run CVLog on SIMPLE cv-on path | gated by `--correlation_vector` |
| P5  | Tests — golden trace + flip gap test + cv-summary counts | test-only |

Each PR is small, leaves the build green, and changes no output bytes on the
default (non-CV) path. P1–P3 touch `lexer` / `javascript-parser`; P4–P5 touch
`closurec`.

## Out of scope

* Per-node CVs for **non-leaf** nodes (binary expressions, calls). Folds derive
  from their operands' ids, so leaf provenance already gives folded results a
  real chain; richer interior-node spans are a later refinement.
* ADVANCED-only passes (rename-globals/properties, tree-shaking) recording CV
  contributions — they run on the same typed AST and inherit leaf ids, but their
  own contribution records are a follow-up.
* The property-key raw-value decode bug (tracked separately) is orthogonal.
