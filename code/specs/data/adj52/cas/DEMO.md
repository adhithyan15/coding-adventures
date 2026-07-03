# CAS edit-override-propagate loop — "fix the fact, not the weight"

The premise of the whole program: build expert systems whose errors trace to
**editable knowledge**, not to weights buried inside a model. When the system is
wrong, the auditable trail localizes the error to a specific claim in the
content-addressed store (CAS); a human **edits that claim**; the fix is local,
versioned, attributed, propagating, and regression-checked. You cannot do any of
that to a neural net — there is no "this belief" to open up and correct.

This directory demonstrates the full loop on **real data**: ADJ55's grounded
bacterial-meningitis corpus and ADJ56's *documented* CSF over-saturation bug.

## The loop, demonstrated (`python demo.py`)

1. **Error, visible in the trail.** `corpus/eval.py` is a deterministic sequential
   Bayesian update that prints every likelihood ratio it multiplies and its source.
   On a **pre-culture** case (CSF chemistry back, Gram stain + culture still
   pending) it climbs to **P = 0.9999** — indefensible certainty, because the
   confirmatory test isn't back yet. You can *see* why: f2–f5 (neutrophilic
   pleocytosis, low glucose, high protein, high lactate) are one correlated CSF
   signal being multiplied as four independent ones.

2. **Localized to specific claims.** The trail points at nodes **f3, f4, f5** — the
   correlated CSF-chemistry findings — as the over-counting source.

3. **Human edits the facts.** `override.py` applies a versioned, attributed,
   *cited* override (`overrides/meningitis-csf-correlation.json`): cap f3/f4/f5 to
   `lr = 1.0` (subsumed under f2 as the single CSF-chemistry signal). **The base
   corpus stays immutable** — the edit is an additive human layer, and each touched
   node records `provenance.override` (who, when, why, source, prior value), so the
   edit itself is auditable and `eval.py`'s trail shows it.

4. **Re-run — the fix propagates.** Pre-culture case: **0.9999 → 0.7709** —
   calibrated "high suspicion, await the confirmatory test," not false certainty.

5. **Regression check — nothing else broke.** The **culture-positive** case
   (Gram + culture positive, genuinely dispositive): **1.0000 → 1.0000**,
   unchanged. The edit corrected the over-confident case without damaging the case
   that should be certain.

| case | base P | edited P | outcome |
|---|---:|---:|---|
| pre-culture (Gram/culture pending) | 0.9999 | **0.7709** | de-saturated — false certainty fixed |
| culture-positive (regression) | 1.0000 | 1.0000 | unchanged — no regression |

## Why this is the point

Every step here is a **file and a number you can inspect, edit, and diff** — not a
weight. The error was *localizable* (to f3/f4/f5), the fix was *local* (three
LR edits), *attributed and reversible* (the override JSON is the human layer, the
base corpus untouched), *propagating* (re-runs everywhere those claims are used),
and *regression-checked* (the dispositive case stayed correct). That is an expert
system you can **correct and audit**, versus a model you can only retrain and hope.

## Honest scope

- This override encodes **correlation knowledge by capping LRs** — a legitimate
  human judgment, and the right *mechanic* to demonstrate. The *principled*
  representation of the same knowledge is the ADJ53 `mechanism` construct (group
  the correlated manifestations under one latent cause); the override layer and the
  mechanism construct are complementary — the override is how a human fixes a
  specific corpus entry today, with full provenance.
- For localization→edit to be one click in production, the trail must reference
  **stable claim IDs** (here we used `eval.py`'s per-node id trail). And edits to a
  *shared* claim must always run the regression set (`demo.py` does this for the two
  meningitis cases) so fixing one case can't silently break another.

## Files

- `override.py` — apply a versioned/attributed/cited human override layer to a base
  corpus → effective corpus (base immutable; each edit logged in `provenance.override`).
- `overrides/meningitis-csf-correlation.json` — the human edit (cap the correlated CSF cluster).
- `cases/meningitis-preculture.json` — the pre-culture decision-point case.
- `demo.py` — reproduces the whole loop + the regression table.
- `effective-meningitis.json` — the derived human-edited corpus (shows the override provenance inline).
