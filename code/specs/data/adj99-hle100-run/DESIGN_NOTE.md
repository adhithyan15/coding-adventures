# ADJ99 — Design note: the CAS as capitalized, correctable intelligence

Forward-looking synthesis the run points toward (not new experimental data). Captured so the reasoning
isn't lost; evidence references ADJ93 (spider-split) and this run (ADJ99).

## The cost-structure inversion

Today every query pays full frontier-model inference. The CAS architecture converts that into:

  one-time capital cost (build CAS for source S, with the STRONG model + adversarial verification)
  + cheap marginal cost per query (a WEAK model reasons over the already-built, already-verified CAS).

This is the economics of **indexing** (build the index once, queries are cheap) and **compilation**
(compile once, run many). The expensive intelligence is *capitalized into a reusable asset* instead of
re-spent on every query. Per-query LLM inference has no accumulation; the CAS accumulates.

## Three separable roles (don't conflate them)

| role | who | when | cost |
|---|---|---|---|
| 1. **CAS builder** — source → verified facts w/ provenance | STRONG model (Opus) + adversarial inspection + human spot-check | once **per source** | capitalized / amortized |
| 2. **Query translator** — question → which facts + what computation/program | WEAK model (Haiku) | per query | cheap, marginal |
| 3. **Executor** — run the program over the facts | a computer (CAS/CAS-solver/program) | per query | ~free, exact |

The weak model only does role 2 — the part it is actually good at (translate messy input → a plan over
already-clean facts). It never does the expensive one-time extraction (role 1) nor the cascade-prone
computation (role 3).

## Why this is the killer feature: an EDITABLE asset, not an EDITABLE model

Correcting a CAS fact is durable, global, and deterministic: a human edits **one fact in the store**,
and **every future reasoning run over that source inherits the fix** — and if role 3 is deterministic
execution, the downstream answers *recompute correctly by construction*, not "hopefully." Contrast with
correcting a model (fine-tune/prompt): probabilistic, leaky, non-transferable. The CAS makes knowledge
editable the way a **database** is editable, not the way a neural net is. Human as **editor of the
asset**, not author of each answer — this is what dissolves the knowledge-acquisition bottleneck.

## The integrity inversion (the honest edge)

The same propagation that makes corrections powerful makes **un-caught build-time errors** powerful:
a fact Opus mis-extracts and the adversarial pass misses is now baked into the asset and poisons every
future query. The cheaper you make reasoning, the more total correctness rests on **build-time
integrity**. So spend the verification budget where it's amortized: strongest model + multi-perspective
adversarial inspection + human spot-check **at build time**; be cheap at query time. (ADJ99 evidence:
52% of fw-haiku failures traced to bad CAS facts — exactly the class build-time inspection must catch.)

## What ADJ99 did and did NOT test (important)

- ADJ99's four arms were `plain-haiku`, `plain-opus`, `fw-haiku` (**all-Haiku** framework), `fw-opus`
  (**all-Opus** framework). **It did not test Opus-builds-CAS → Haiku-reasons.** So the negative headline
  ("Haiku+framework < Opus") indicts the *wrong configuration* — a cheap model doing the expensive
  one-time job — **not** the division-of-labor architecture.
- ADJ93 (spider-split) *did* test it and supported it: Opus-spider→Haiku-reason ≈ Opus-spider→Opus-reason
  on retrieval-bound items (the *builder* is the bottleneck; cheap reasoning over a good CAS ≈ frontier).
- ADJ99 corroborates directionally: fw-opus's wins were **CAS-building wins** (retrieving the
  authoritative source — Jane Street solution, Godot source, Nagano paper), not reasoning wins.
- **Untested empirically anywhere yet: reuse.** Every run rebuilt a per-query CAS. The amortization
  claim is an economic argument, not a measurement.

## The experiment that would prove the whole thesis in one artifact

One rich source → build the CAS **once** with Opus + adversarial verification → answer **N different
questions** over it with Haiku (role 2) + execution (role 3). Then a human corrects **one** fact.
Measure: (a) marginal cost per query vs all-Opus; (b) does the single correction **propagate
deterministically** to all affected answers; (c) locus-coverage + correction-propagation (not accuracy).
A successful demo = *one correction, N corrected answers, at 1/Nth the marginal cost* — amortization +
correctability + model-split, shown together.

## Addendum — cross-experiment reconciliation (ADJ52 run-3 / run-5 + ADJ-CAS, run after ADJ99)

Three later experiments let us decompose what ADJ99 measured as one blob ("the framework"):

- **The cited-reasoning *discipline*** (decompose → rulebook → cited conclusion) is a **within-model
  defensibility gain** — large at weak models, small at the frontier (which is already disciplined):
  - ADJ99 within-model: fw-haiku 2.68 > plain-haiku 2.14; fw-opus 3.61 ≈ plain-opus 3.72.
  - **ADJ52 run-5** (Haiku vs Haiku, no engine): framework **wins the blind judge 60-37**, +2 correct,
    and **0 cases where defensibility preference overrode a correct answer**. This is the right
    comparison ADJ99 buried by anchoring fw-haiku against plain-**Opus** (a *cross-model* gap) instead
    of against plain-haiku. The framework's value is a within-model lift, largest where the model is weak.
- **The numeric *engine*** (saturated Bayesian posteriors) is a defensibility **loss**: ADJ52 **run-3**
  (frontier *with* engine) **lost the blind comparison 39-60** — punished as false precision (28 losses
  were "right-but-overconfident"). ADJ99's framework had **no engine** (just a cited chain), which is
  exactly why fw-opus came out *neutral* at the frontier rather than negative — nothing dragged it down.
- **The engine's defensibility-killer is itself a correctable CAS error.** The ADJ-CAS
  edit-override-propagate demo (meningitis corpus) shows the over-saturated 0.9999 posterior is correlated
  facts mis-weighted as independent; a human override caps the correlated cluster → **0.9999 → 0.7709**,
  propagating, with the dispositive case regression-checked unchanged (1.0000). So you keep the engine
  (deterministic execution — the "execute, don't reason" half) and fix its false precision by **editing
  the CAS facts**.

**Unified picture:** discipline (within-model defensibility) + engine/execution (deterministic numbers)
+ editable CAS (fixes the engine's false precision) + correctability (the governance payoff). ADJ99's
"neutral at the frontier" is consistent with all of it: it ran a cheap-model-builds-its-own-CAS,
no-engine configuration — neither the division-of-labor (Opus builds, Haiku reasons) nor the execution
layer that the rest of the program is built around. The clean next isolation: **frontier model with the
same no-engine discipline arm** — if it also wins the blind comparison, "the engine was the loss factor"
is nailed.
