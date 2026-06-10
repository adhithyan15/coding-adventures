# E2 correctability run — pre-registration (locked before the localize pass)

> Implements [`PAPER1-E2-correctability-study.md`](../../papers/PAPER1-E2-correctability-study.md)
> (work item W2, the paper's **headline**). Pre-registered before any auditor call is made.
> The thesis goal-shift: from **correctness** to **correctability** — when the model is wrong,
> how cheaply can a reviewer *localize* the error, *fix* it with one local edit, and have the
> fix *persist + propagate*. Framework (byte-provenance trail + editable CAS) vs plain prose.

## 0. What is new here vs ADJ96

ADJ96 ran this protocol on 6 reasoning items and the blind auditor saw the **raw** framework
trail (citation-shaped) vs raw prose. That is exactly the **format confound** the ADJ99 rescore
(PR #5261) later proved: a regex separates the arms with 100% accuracy, so a "blind" judge is
not blind, and any delta is contaminated by format. **E2 fixes this reflexively** — it applies
the ADJ99 measurement-validity lesson to its own headline experiment. This is itself a
contribution (Threats-to-Validity): the correctability result survives format normalization.

## 1. Design

**Paired-solution comparison.** For each item we use the two solutions ADJ99 already generated
with the *same solver model* on the *same question*:
- **plain arm** — free-prose answer + work (`plain-{scale}`).
- **framework arm** — byte-provenance trail: sourced facts + a cited stepwise reasoning chain
  (`fw-{scale}`).

**Treatment vs confound — the central distinction.**
- The **treatment** (what the framework genuinely does) is **decomposing the reasoning into
  discrete, individually-checkable premises/steps**, so a reviewer can point at one line.
- The **confound** (what ADJ99 caught) is **citation chrome** — `RETRIEVED FACTS (CAS):` headers,
  `[cites: n]` markers, `(src: URL)` parentheticals — which merely *looks* rigorous and let the
  judge identify the arm.

We **strip the chrome and keep the decomposition** (reusing the ADJ99 `normalize()` function), then
**measure the residual leak** (§4). If a regex can still separate arms it can only be via the
*structure that is the treatment*, not chrome — and we report that number openly rather than
claim a perfectly blind judge.

**Wrong-answer selection.** Only items where the chosen arm's solution is **incorrect** (per the
ADJ99 accuracy flag) — there must be a real error to localize. Primary stratum: items where
**both** arms are wrong (a matched within-item pair), so the comparison is the *artifact*, not
which arm happened to be right.

## 2. Arms, models, items

- **Solver scale (the error source):** primary = **Haiku** (the cheap scale, where correctability
  matters most); robustness subset = **Opus** (tests model-independence of the localize delta).
- **Auditor / oracle:** **Opus**, the *same* reviewer on both arms (so the delta is the artifact).
  Cross-judge robustness: a non-Opus second auditor (Sonnet) on a subset (shared with E4/W5).
- **Items (n ≥ 30 wrong solutions):**
  - **ADJ99 both-arms-wrong**, stratified across all 8 categories (Math, Physics, Chemistry,
    Biology/Medicine, CS/AI, Engineering, Humanities/SocSci, Other). Target **4 per category = 32
    items × 2 arms = 64 cells** for the primary (Haiku) run.
  - **ADJ96 5 reasoning items** (the original set, 1 oracle-correct item dropped) re-run through the
    *same normalized pipeline* for continuity with the published 5/5-vs-2/5 result.
  - **Opus-scale robustness:** 2 per category = 16 items × 2 arms = 32 cells.

## 3. The correction protocol (per cell = one wrong solution)

1. **Blind localize** (no gold, normalized envelope only): the Opus auditor names the single
   most-likely-flawed step/quantity, or "could not localize". Identical prompt for both arms
   (the ADJ96 auditor prompt). This is the only artifact the auditor sees.
2. **Oracle + score** (gold-aware, run *after* the blind audit): Opus + gold + the *raw* solution
   establishes the true error locus for *that* solution, then judges the blind audit as
   **hit / partial / miss / n-a** against it, and flags **auditor_fooled** (auditor declared the
   wrong solution correct, or affirmed a false load-bearing premise).
3. **Fix-locality** (mechanical, no model): framework fix = the minimal CAS-fact override / clause
   edit at the located locus (1 local edit); prose fix = "not localizable → rewrite derivation".
   Measured structurally + via the worked CAS examples (§5), not by an LLM.
4. **Persist + propagate** (no answer-time model call): re-derive from the corrected CAS; count
   sibling cases corrected by the single override. Sourced from the already-built CAS families
   (§5), reported as the derive-once panel.

## 4. Metrics (pre-registered)

- **`localize_rate`** = (hit + 0.5·partial) / n_wrong, per arm. **Primary, RQ1.** Report the
  framework−plain delta with a bootstrap 95% CI (10k resamples over items).
- **`auditor_fooled_rate`** = fraction where the auditor affirmed/over-trusted a false premise,
  per arm. (The ADJ96 qualitative failure made quantitative.)
- **`fix_locality`** = fraction of fixes that are a single local override/edit (framework) vs
  require derivation rewrite (prose). **RQ2.**
- **`propagate_yield`** = mean fraction of sibling cases corrected per single override, with
  **answer-time model calls = 0** as the headline derive-once number. **RQ3.**
- **`leak_accuracy`** = accuracy of the best deterministic regex arm-classifier on the
  *normalized* artifacts. Reported as a validity guard (target: well below the 100% ADJ99 saw;
  any residual is attributable to the decomposition treatment, not chrome).

## 5. Fix + propagate sources (already on disk — aggregated, not re-run)

- `code/specs/data/adj52/cas/` — meningitis CSF-correlation override: one edit corrects the
  pre-culture case (P 0.9999→0.7709) and is regression-safe on the culture-positive sibling.
- `code/specs/data/adj101-defensibility-100crossdomain/run100/cas/` — TAX rulebook → compiled
  program library → held-out inputs run with **zero answer-time model calls** (derive-once).

## 6. Hypotheses & the honest null

- **H1:** `localize_rate_fw > localize_rate_plain`, surviving normalization (CI excludes 0).
- **H2:** framework fixes are single local edits; prose has no localized handle.
- **H3:** one override propagates to K siblings at zero answer-time model cost; prose = 0.
- **Honest null we must be able to report:** if normalization erases the localize delta (the
  advantage was *only* chrome), or if prose corrections are just as cheap/persistent, the
  correctability thesis is weakened. The design can show this — that is why the leak-check and
  the both-arms-wrong matched pairing are pre-registered.

## 7. Threats specific to this run

- **Prose strawman** — the plain arm is best-effort same-model prose with work shown (not terse).
- **Reasoning confound** — fw and plain are *different* reasonings; mitigated by same solver, large
  n, matched both-wrong pairing, and reporting per-item paired deltas.
- **Gold-is-wrong items** — dropped and logged (cf. ADJ96 `integral` 5482).
- **Auditor ceiling** — Opus auditor is competent-but-not-omniscient; report the Opus-solver
  subset where the auditor cannot simply re-derive.

## 8. Outputs

`items_e2.json` (private map: cell → arm/scale/gold/raw) · `cells/cell_NNNN.json` (blind, what the
auditor sees) · `leak_check.json` · `localize_results.json` (raw audits+scores) ·
`fix_propagate.json` · `aggregate.json` (CIs) · `FINDINGS.md`. One-command reproduce documented in
FINDINGS.
