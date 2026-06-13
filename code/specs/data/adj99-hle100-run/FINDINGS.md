# ADJ99 — HLE-100 run: FINDINGS

> **⚠ CORRECTION (rescore).** The 0–5 "defensibility" numbers in this document measure **traceability
> / citation density**, not defensibility. The judge rubric rewarded the cited-chain *format*, and a
> one-line regex identifies the arm from that format with **100% accuracy** — so the "blind" judge was
> not blind. The headline means below (2.14 / 3.72 / 2.68 / 3.61) and the H1/H2 verdicts should be read
> as **traceability**, and superseded by the construct-valid rescore in
> [`analysis/rescore/RESCORE_FINDINGS.md`](analysis/rescore/RESCORE_FINDINGS.md). Under a rubric that
> scores whether the **load-bearing premise is exposed and flagged as fallible** (format-normalized so
> style can't leak the arm), the framework's advantage **grows**, and **H2 flips from FALSIFIED to TRUE**
> (fw-opus +0.45 over plain-opus, vs −0.11 here). The **audit-trail metrics in §"Audit-trail /
> correctable-CAS" below are unaffected** — they were always the construct-valid evidence and remain the
> primary result.

**Status: COMPLETE.** 100 stratified HLE items, 4 arms each, run in 20 batches of 5.
Raw data: `batches/batch_00.json … batch_19.json`; machine summary: `aggregate.json`.
Defensibility = blind Opus judge, 0–5 (grounded / auditable / traceable reasoning, **independent of
correctness**). Accuracy = strict blind judge, secondary (not the target). Gold never shown to any
solver or auditor.

## Headline table (n = 100)

| arm | mean defensibility | def ≥ 4 | correct | partial | incorrect |
|---|---|---|---|---|---|
| plain-haiku | **2.14** | 14/100 | 4 | 7 | 89 |
| **plain-opus** | **3.72** | 60/100 | 27 | 8 | 63 |
| fw-haiku | **2.68** | 25/99 | 7 | 5 | 87 |
| fw-opus | **3.61** | 56/98 | 27 | 6 | 65 |

(n_scored excludes [agent-error] cells; 5 such cells across 400 total, ~1.25%.)

## Pre-registered hypotheses — both FALSIFIED

**H1 — "Haiku + framework reaches plain-Opus defensibility": FALSE.**
fw-haiku 2.68 sits **~1.05 below** plain-opus 3.72 — about midway between plain-haiku (2.14) and
plain-opus. The framework reliably **lifts the cheap model's defensibility floor** (2.14 → 2.68,
+0.54; def-≥4 count nearly doubles, 14 → 25) but does **not** close the gap to the frontier baseline.

**H2 — "Opus + framework is slightly MORE defensible than plain-Opus": FALSE (marginally lower).**
fw-opus 3.61 vs plain-opus 3.72. The provenance scaffolding adds **nothing** on top of Opus's native
reasoning and is a hair lower — Opus already reasons in a grounded, auditable way unaided; forcing it
through spider→CAS→cited-chain occasionally *constrains* it (e.g. retrieved facts can mislead, and the
provenance gate truncates). Net: at the frontier the framework is defensibility-neutral.

**Robust finding:** *the byte-provenance framework raises a cheap model's defensibility but not to the
frontier; on the frontier model it is neutral.* Defensibility tracks the underlying model, not the
scaffold.

## Accuracy (secondary; not the target)

- Framework gives the **cheap model a small accuracy lift** (plain-haiku 4 → fw-haiku 7 correct) via
  open-book retrieval; for **Opus it is exactly neutral** (27 = 27 correct).
- This 100-item set is **hard**: even plain-Opus is only 27% correct. Many items are self-contained
  derivations where retrieval has nothing external to ground to (framework neutral-to-harmful), while
  lookup/recall items are where retrieval wins (see texture below).
- **Caveat (ADJ95 lesson):** accuracy here is a strict blind-Opus judge, which can be noisy on
  free-form numeric/short answers. **Raw answers + gold are preserved per item** in `batches/` for
  deterministic re-grading; treat accuracy as approximate.

## Audit-trail / correctable-CAS — the durable POSITIVE result (this run's actual goal)

For every `fw-haiku` trail, a fresh **same-model adversarial Haiku** and a fresh **cross-model
adversarial Opus** tried to localize the flaw and flag whether it traced to a CAS fact extracted
incorrectly.

| metric | value |
|---|---|
| cross-model **Opus** localized a flaw | **90/100 (90%)** |
| same-model **Haiku** localized a flaw | 82/100 (82%) |
| **either** auditor localized a flaw | **95/100** |
| flaw traced to a **CAS extraction error** (correctable by override) | **52/100 (52%)** |
| distinct CAS facts flagged for correction | **118** |
| auditor self-failures (agent-errors) | same-Haiku 6, cross-Opus 2 |

This is the property the run was built to demonstrate, and it held steadily across all 100 items:
**a cheap model's reasoning, run through byte provenance, is auditable — a reviewer (or a stronger
model) can pinpoint where it went wrong in ~9/10 cases, and over half the time the error is a specific
fixable CAS fact** (the "override a bad fact so future runs get it right" deliverable). Concrete
examples in the raw data: a quote misattributed to Chagall (pulled from Instagram; correct = Rouault);
a perovskite mode-count fact mislabeled as olivine; a resistivity constant the model "corrected" to a
wrong value; fabricated reachable-state counts in a DP problem; a wrong-formula mass attached to a
molecular target.

**Secondary finding — auditing wants a different, stronger model.** Cross-model Opus beat same-model
Haiku at localization (90 vs 82) and self-failed far less (2 vs 6). Haiku reviewing Haiku shares its
own blind spots — in at least one case (a Thompson-group scl problem) same-Haiku rubber-stamped
"no flaw" on a wrong answer that cross-Opus caught. Bears on the "who builds/audits the CAS" question:
put the bigger model on the audit, not (only) the solve.

**Auditors are not infallible.** On the Crayola/Rhodamine-B item, fw-haiku honestly abstained (no
reliable public source) and *both* auditors flagged the abstention as an error and pushed toward a
confident answer ("Razzmatazz") that was itself wrong (gold = "Razzle Dazzle Rose"). The adversarial
critic can over-penalize honest uncertainty and advocate a confidently-incorrect fix. Logged in the
corpus as a counterexample.

## When the framework helped vs hurt (texture)

- **Lookup / known-artifact items → framework (esp. fw-opus) wins** by retrieving the literal source:
  Jane Street "Well Well Well" puzzle (got 360 from the official solution while plain arms computed
  11/946/1204); exact Godot 4.3 analyzer error string from GitHub source; Nagano et al. hyperbolic
  wrapped-normal density (derived with the `k` term; plain-opus didn't); the SQ-lower-bound paper.
- **Self-contained derivations → framework neutral-to-harmful:** balloon ODE (plain arms nailed the
  closed form; fw-haiku retrieved generic ODE facts and drifted); retrieval has nothing to ground to.

## Caveats
- 5 arm agent-errors / 400 cells (~1.25%), excluded from means (resilience `safe()` wrapper).
- Batch 13's first attempt hit a transient rate-limit (7/20 cells errored) and was **re-run**; the
  clean re-run (2/20) is what's saved (degraded version backed up off-tree).
- N=1 sample per arm per item; defensibility is a single blind-judge score. Effects reported are the
  large, stable ones (the ~1.0-point fw-haiku↔plain-opus gap; the ~90% audit-localization rate); do
  not over-read small deltas (e.g. the 3.61 vs 3.72 fw-opus↔plain-opus difference is within noise).

## Bottom line
The cheap-model-reaches-frontier-defensibility thesis did **not** hold, and the framework did **not**
boost the frontier model's defensibility. What the run *did* establish, robustly, is the
auditability/correctable-CAS property: byte provenance makes a cheap model's reasoning **diagnosable**
(flaw localized ~90% of the time) and **correctable at the fact level** (~52% of errors trace to a
specific CAS fact you can override) — with a stronger cross-model auditor doing the localization best.

## Addendum — what "defensibility" meant here, and the measurement caveat

**What we *mean* by defensible** (the notion the program targets): a decision whose **locus of
contingency is exposed** — every load-bearing claim is attributed to a source or a *stated assumption* —
so that when the decision is wrong, you can point to the specific premise that produced the error,
override it, and re-derive. Defensibility is about the **auditability and correctability of the decision
process**, explicitly decoupled from correctness of the outcome (humans make defensible-but-wrong
calls constantly; what makes them professional is the exposed, correctable locus).

**What the run actually measured** (verbatim rubric handed to the blind Opus judge):

> Score DEFENSIBILITY 0-5 — how grounded, auditable, and traceable the reasoning is, INDEPENDENT of
> whether the final answer is correct: 0=bare assertion … 4=well-grounded, nearly every claim
> traceable to a cited source or the givens; 5=fully auditable, every claim traceable, no unsupported
> leaps.

So the judge **was** given a definition — but it operationalizes defensibility as **citation/
traceability density**, a narrower proxy than "is the load-bearing locus surfaced as fallible." Three
caveats follow, and they should temper every defensibility number above:

1. **Format confound (the big one).** The judge was blind to the arm *label* but not to *style*: the
   `fw-*` arms emit a literal `RETRIEVED FACTS [n] … REASONING CHAIN … [cites: n]` structure while the
   `plain-*` arms emit free prose. The rubric rewards exactly that cited-chain *format*, so part of the
   measured fw advantage is "emitted a citation-shaped artifact," not "reasoned more soundly." A fair
   future run must **normalize format across arms** (or have the judge score a structure-neutral
   rendering) so style can't leak the arm.
2. **Rewards attribution, not soundness.** A confidently-wrong chain in which every step *cites a (bad)
   fact* scores 4-5. This is why provenance-completeness came out uncorrelated with correctness and why
   ~64-84% of def≥4 answers were wrong — the rubric grades whether claims are *attributed*, not whether
   the premise is *true* or its fallibility *flagged*.
3. **Not the correctability notion.** Citation density ≠ "the error locus is exposed and overridable."
   The audit-trail metrics (95% locus-localized, 52% fixable CAS fact) are a *closer* operationalization
   of what we mean and should be treated as the primary correctability evidence; the 0-5 score is a
   weaker, format-sensitive proxy. **Recommendation for the next run: a correctability-explicit rubric**
   ("can you name the single assumption/fact whose change would flip this answer?") plus format
   normalization.
