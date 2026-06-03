# ADJ53 — Latent-Mechanism Modeling + Recursive Source-Tree Derivation

> **Headline.** ADJ52's hands-off runs exposed the real defect: the
> engine aggregates evidence as independent log-likelihood-ratios
> (Naive Bayes), but clinical findings are highly correlated — multiple
> manifestations of one underlying mechanism. Summing their marginal
> LRs double-counts, saturates the posterior to ~100%, and de-calibrates
> a strong reasoner. The softmax "coherent differential" patch was
> reverted because it normalized already-inflated scores. The correct
> fix is structural: model the **latent mechanism** that the correlated
> findings share (a QMR-DT-style disease → mechanism → findings
> Bayesian network), and **extract that structure from the literature
> itself** via a recursive, Google-like source decomposer that keeps
> byte-provenance all the way down.

## The defect, precisely

`logit(D) = logit_prior + Σ log LRᵢ` is exact only if the findings are
conditionally independent given D. They are not: "markedly elevated CK,"
"CK rises on exertion," "exertional cramps," "second wind" are one
glycogenolytic block seen four ways. Summing four marginal LRs counts
one signal four times → false certainty, and exaggerated gaps that
confidently exclude the correct sibling (ADJ52 run 2, case-2: 100%
Killian-Jamieson, 0% Zenker). The independence assumption is invisible
because it is baked into the formalism (one `contributes` edge per
finding) — neither the language nor the deriver can currently say "these
share a cause."

## Fix part 1 — model the latent mechanism (language + engine)

Introduce a mechanism (latent) node between disease and findings:

```
disease  →  mechanism M  →  { finding_a, finding_b, finding_c }
```

Conditioning on M makes the findings conditionally independent **given
M**. Observing many manifestations gives strong evidence for *M*; M
updates D **once** via P(D|M). The findings no longer each hammer D —
the over-counting is gone. Combine manifestations with a **noisy-OR /
saturating** rule, not a sum: presence of the mechanism contributes a
bounded amount however many of its manifestations are observed.

### adj-lang extension

New clause kind (subject to refinement during implementation):

```
% M is a latent mechanism that bears on the conclusion with one bounded LR;
% any of its manifestations being observed fires the mechanism ONCE.
mechanism glycogenolytic_block for diagnosis(mcardle_disease_gsd_v)
  contributes 12.0
  manifested_by creatine_kinase(markedly_elevated)
  manifested_by ck_rise(post_exertion)
  manifested_by symptom(exertional_cramps)
  manifested_by symptom(second_wind)
  source "GeneReviews NBK1344 (these are joint consequences of the enzyme block)"
  trust authoritative
```

Semantics: if ≥1 `manifested_by` term is observed, the mechanism fires
and contributes `log(12.0)` to the conclusion **once** (noisy-OR
strength optionally scaling sub-additively with the count, capped at the
declared LR). The individual manifestations do **not** also fire
independent `contributes` edges for that conclusion — the mechanism
subsumes them. This is the minimal construct that kills the
double-count; richer multi-level mechanisms are a follow-up.

Implementation: extend `code/grammars/adj_lang.tokens` + `.grammar`
(keyword `mechanism`, `manifested_by`), regenerate via grammar-tools
(never hand-edit `_grammar.rs`), extend the adj-lang AST + lowerer, and
implement the noisy-OR aggregation (in the ADJ52 runner first to avoid a
shared-crate change; promote into `logic-engine` once stable).

## Fix part 2 — recursive, Google-like source-tree derivation

The deriver stops being "emit flat LR edges" and becomes a recursive
indexer/decomposer that **extracts the causal structure from the
sources**:

1. **Index** a source (paper / guideline) — fetch it.
2. **Decompose** it into a **shape-preserving tree** (NOT flat JSON):
   claims nest under the sections/mechanisms/assertions they belong to,
   so the document's structure (a mechanism and its consequences;
   a finding and its qualifiers) is preserved, not flattened into a
   list. Every node carries its source byte-span.
3. **Follow the citations** the source itself makes — for each claim,
   find the references it rests on, fetch and decompose those, and
   **recurse**, keeping the provenance tree growing downward, until a
   stopping criterion (depth bound, or claims bottom out in
   primary/authoritative sources).
4. The mechanism structure (part 1) is **read off this tree**: when a
   source says "M causes A, B, C," that subtree becomes a `mechanism`
   clause whose `manifested_by` set is exactly {A, B, C}, provenanced to
   the span.

This is the ADJ40/41/51 recursive-source-decomposition + indexed-corpus
vision made the deriver's core loop. The tree shape is what lets the
causal/mechanism structure survive into the rulebook instead of being
flattened away.

### Faithfulness check (byte-provenance catches the correlation)

Extend the byte-provenance contract from "every clause cites a span" to
**"the rulebook's independence structure must match the sources'."** A
verifier flags: if findings A, B, C are encoded as *independent*
`contributes` edges for D, but a cited source describes them as one
syndrome / shared mechanism, that is a provenance violation — collapse
them under a `mechanism` clause or cite a source for their independence.
This is the rationale↔clause verifier (ADJ51) extended to *structure*.

## What this fixes and what it doesn't

- **Fixes:** the over-counting / saturation (the dominant calibration
  bug), via correct structure read from the literature; and the audit
  trail gets *richer* — it now shows "CK, cramps, second-wind count once
  because Source X says they share a cause," which a holistic LLM cannot
  show its work on.
- **Does not, by itself, fix:** the parameters (conditional
  probabilities) are still mostly LLM-estimated — the literature gives
  the graph, not the joint tables. But over the *right* structure the
  answer is far less parameter-sensitive (noisy-OR over a mechanism
  tolerates rough magnitudes where a product of independent LRs does
  not).

## The experiment this enables: top-down descent

**The goal is NOT to beat the frontier base model.** Bottom-up (start
with small models) failed — nothing worked. ADJ53 starts at the **top**:
establish that the full machinery (recursive source-tree derivation +
latent-mechanism rulebook + deterministic engine + audit trail) produces
**correct, defensible, well-calibrated** output with a frontier model
doing ingestion/derivation. Then **step the model down** the capability
ladder (frontier → mid → small → tiny) doing the framework's LLM roles,
and measure **where it breaks** — at what model scale does the
ingester/deriver stop producing a faithful IR / rulebook, and how far
does the framework's structure let a weak model punch above its raw
weight (the ADJ17 effect, now with correct probabilistic modeling).

Success metrics are the **descent curve**, not a blind win:
- correctness + calibration as a function of answerer/deriver model size;
- the breaking point (the smallest model that still yields a defensible
  verdict with the framework vs. raw);
- auditability and faithfulness (does the rulebook structure match the
  sources) at each rung.

## Implementation sequence

1. **adj-lang `mechanism` construct** — grammar + tokens + regen + AST +
   lowerer; noisy-OR aggregation in the ADJ52 runner. Unit-tested on a
   McArdle-shaped fixture (correlated findings → one mechanism update →
   no saturation).
2. **Recursive tree-shaped deriver** — shape-preserving decomposition +
   citation-following recursion with depth bound; mechanism structure
   read off the tree; byte-provenance per node.
3. **Structure-faithfulness verifier** — flag independent encodings that
   contradict a source's shared-cause description.
4. **Descent harness** — the ADJ52 pipeline parameterized by the model
   used for ingestion/derivation; sweep model size; report the curve.

## Status

- 2026-06-03: spec authored on branch `adj52-blind-cross-arm-experiment`
  after reverting the softmax calibration patch. Implementation starts
  with the adj-lang `mechanism` construct.

## See also

- [ADJ52](ADJ52-autonomous-blind-cross-arm-experiment-loop.md) — the
  hands-off loop + the runs that exposed the Naive-Bayes defect
  (`code/specs/data/adj52/runs/`).
- [ADJ14](ADJ14-probabilistic-ir-semantics.md) / LP19e — the LR
  aggregation this corrects.
- [ADJ40](ADJ40-recursive-source-decomposition.md) /
  [ADJ41](ADJ41-decomposed-source-ir-store.md) — the recursive
  source-decomposition + indexed-corpus vision the tree deriver realizes.
- [ADJ17](ADJ17-adversarial-rulebook-empirical-results.md) — the
  small-model "punch above weight" effect the descent experiment revisits.
