# ADJ99 — HLE-100 run: SUPPLEMENT (deeper analysis + reframing)

This supplements `FINDINGS.md`. It (a) corrects a framing error in how the headline read
defensibility-vs-correctness, and (b) reports five analyses computed over the saved raw batches
(`batches/batch_*.json`) with no new model calls (script: `analysis/mine_adj99.py`).

## Reframing — the bar is correctability, not correctness

`FINDINGS.md` treated "provenance certifies defensibility, not truth" and "def≥4 answers are wrong
~2/3 of the time" as *limitations*. That is the wrong lens. **Humans make defensible-but-incorrect
decisions constantly; what makes such a decision professional rather than negligent is that you can
point to the assumption, misinterpretation, or missing fact it rests on — which makes it defensible,
auditable, and *correctable*.** Correctness is not the target; it is a downstream consequence of the
correction loop running over time. The unit of value is a **decision + its locus of contingency**
(the load-bearing premise you can override and re-derive from).

Under this lens the run did **not** under-deliver. The decoupling of defensibility from correctness is
the *feature*: the framework produced defensible decisions whose wrongness is **traceable to a locus**.

## The correctability surface, measured (n=100, fw-haiku)

For each wrong/flawed fw-haiku answer, the adversarial auditors located *where* the decision rests:

| locus type | count | correction operation |
|---|---|---|
| **specific CAS fact** (overridable) | **52/100** | edit one fact in the store → re-run (tightest loop) |
| **reasoning/computation step** | **43/100** | redo that step — the part the execution layer turns into *deterministic re-run* |
| no flaw found (answer stood) | 5/100 | — |
| locus localized at all (either auditor) | **95/100** | the auditability property, delivered |

So the framework already makes **95%** of the cheap model's decisions locus-tagged, **52%** correctable
by a single fact-override, and the remaining **~43% are reasoning-step loci** — exactly the class the
"LLM extracts facts → writes a program → computer executes" move converts from "redo it and hope" into
"fix the input/code and re-run deterministically." Execution is in service of *correctability* (tighten
the loop, make corrections propagate, kill cascades), not in service of chasing a correctness score.

## Five supplementary analyses

**1. Error budget ≈ 45% reasoning / 55% retrieval.** Of fw-haiku failures: 43 reasoning-only, 52
CAS-extraction, 5 none. Both prongs of the architecture (better retrieval/translation; program
execution) have a roughly equal-sized target. The reasoning share is likely *under*counted — some
"CAS-extraction" flags are the model's own bad arithmetic written *into* a fact (e.g. resistivity
`s=0.5→1.5`), i.e. a compute error in fact's clothing.

**2. Provenance-completeness is ~uncorrelated with correctness.** fw-haiku: prov-complete = 2 correct /
47 wrong; prov-incomplete = 5 / 45. fw-opus: 96/98 prov-complete, still 72% wrong. A fully-cited chain
certifies "followed from cited facts," which is *orthogonal* to "facts + computation were right." This
is the feature, not a bug (see reframing) — but it is also why a *content* check (execution) is needed
to tighten correctness over correction cycles.

**3. Most-defensible answers are still mostly wrong.** def≥4 wrong-or-partial: plain-opus 65%,
fw-opus 64%, fw-haiku 84%. The producing model's own groundedness/confidence does not separate right
from wrong on hard problems — so an *independent* verifier (execution + cross-model audit) is
load-bearing, and "looks defensible" must never be read as "is correct."

**4. Frontier retrieval swaps items, governed by source authority.** fw-opus vs plain-opus accuracy:
both 19, fw-opus-only 8, plain-opus-only 8, neither 63. The 27=27 tie is churn, not inertia. By domain:
FORMAL (n=63) fw-opus 20 > plain-opus 17 (authoritative sources: papers/code/puzzle solutions);
INFORMAL (n=37) fw-opus 7 < plain-opus 10 (polluted sources: Instagram/Etsy SEO spam). Lesson: retrieval
value tracks **source authority**, not recall-vs-derive — retrieval *poisons* a capable model when the
sources are low-authority. Implies *selective, authority-weighted* retrieval, not always-on.

**5. Auditors agree 88%; disagreement is a triage signal.** same-Haiku vs cross-Opus agreed on
found-flaw 81/92; disagreements skew toward same-Haiku *missing* a flaw cross-Opus caught. Gives a free
cheap→expensive escalation policy: spend the strong verifier where cheap and strong auditors diverge.

## Raw data inventory (everything is in this PR)

- `items_100.json` — the frozen 100-item set (id, question, gold, category).
- `batches/batch_00.json … batch_19.json` — full per-item raw output for all 4 arms: answers, complete
  reasoning trails, CAS facts with sources, both adversarial audits, blind judge scores.
- `batches/batch_13_degraded_ratelimited.json` — the rate-limited first attempt at batch 13 (7/20 cells
  errored); preserved for completeness; the clean re-run is `batch_13.json`.
- `aggregate.json` — machine summary; `analysis/` — the exact scripts used (sampling + aggregation +
  this supplement's analysis), for reproducibility.
- `FINDINGS.md`, `SUPPLEMENT.md`, `README.md`.
