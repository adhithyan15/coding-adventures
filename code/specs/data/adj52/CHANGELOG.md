# Changelog — adj52-experiment

All notable changes to the ADJ52 experiment runner.

## [0.4.0] — 2026-06-04

### Added

- **ADJ56 — cross-domain stress test + two honest failure modes.**
  - **Generic, domain-agnostic tooling:** `corpus/build.py` (assembler) and
    `corpus/eval.py` (deterministic evaluator) — adding a domain is now ground →
    build → eval, no per-domain code.
  - **Two new grounded corpora** built case-blind by the forward spider:
    `streptococcal_pharyngitis` (9/9 — Centor/McIsaac + RADT) and
    `bacterial_meningitis` (9/9 — CSF parameters). With PE (12/12) the method now
    spans three sub-domains.
  - **PE stress test (n=4):** the framework's *correctness* edge appeared in 1/4
    cases (the Wells-0 trap); on clear cases all three arms agreed and its value
    reverts to auditability. PMC11999957 was robust but not representative.
  - **Two named failure modes:** (1) *population extrapolation* — the strep corpus
    over-applied a child prior + age LR to a 62-day-old infant; (2) *correlated
    over-saturation* — the meningitis CSF parameters, all grounded individually,
    multiply to P=1.0000 because they are manifestations of one process (the
    ADJ53/ADJ54 `mechanism` problem). Grounding magnitudes (ADJ55) is necessary but
    not sufficient; correlation-aware combination (ADJ54) is still required.
  - Spec: [ADJ56](../../ADJ56-cross-domain-stress-test.md). Artifacts in `corpus/`
    and `provenance/{strep,meningitis,pe}/`.

## [0.3.1] — 2026-06-04

### Added

- **ADJ55 three-arm comparison (the control arm §5 owed).** Ran plain frontier Claude
  (no framework) on the PMC11999957 PE case alongside the grounded corpus and the
  ungrounded deriver. Result: plain Claude put PE at **3–5%** and was "comfortable not
  imaging" a patient who had a PE — the same Wells-0/D-dimer trap; only the grounded
  corpus (0.28 → 0.89) caught it. On this case the framework's edge was **correctness**,
  not merely auditability. Writeup + verbatim run in `provenance/pe/arms/`; ADJ55 §5/§6
  updated. **n = 1 — existence proof, not a rate** (rule-out + high-Wells cases still owed).

## [0.3.0] — 2026-06-04

### Added

- **ADJ55 — provenance-first corpus construction (MYCIN-2026).** The byte-provenance
  invariant — *every magnitude must point at a datum* — built into a forward
  corpus-construction pipeline, proven end-to-end on pulmonary embolism.
  - **`provenance/` — the byte-provenance method.** A recursive crawler ("spider")
    that, per quantitative claim, follows citations to primary data and grades the
    magnitude `grounded` / `direction_only` / `fabricated`. Backward audit
    (`provenance/spider.workflow.js`) on case-5 urology: **0/19 grounded**, the
    decisive clause `fabricated`. Forward construction
    (`provenance/pe/ground.workflow.js`) on PE: **12/12 grounded** to PIOPED II /
    Christopher / D-dimer meta-analyses.
  - **`corpus/` — the grounded knowledge base (the product).** Canonical
    `corpus/pulmonary_embolism/corpus.json`: every LR carries a byte-anchored
    provenance chain; ungroundable links are explicit data-gaps. `build_corpus.py`
    assembles it; `eval_case.py` runs a deterministic sequential Bayesian update where
    every multiplier prints its source.
  - **End-to-end proof (PMC11999957, Wells-0 patient who had PE):** grounded corpus →
    0.28 pretest → 0.89 after CTPA (correct, fully auditable); ungrounded invent-LRs
    deriver → 0.01 "excluded" (missed a real PE). Byte provenance is the variable that
    flips it. See [ADJ55](../../ADJ55-provenance-first-corpus.md).
  - **`provenance/` (case-5 tree experiment):** a tree-JSON rulebook + direct evaluator
    (`build_tree.py` / `eval_tree.py`) reproducing the engine exactly, with
    `grounded_only` mode dissolving case-5's confident-wrong 0.99 to a base-rate
    differential once fabricated LRs are stripped.

## [0.2.0] — 2026-06-03

### Added

- **ADJ54 H2 — open-question discounting (hold residual uncertainty).** Per
  query, when a decision-relevant `uncertain { … }` marker bears on *that*
  conclusion and is still unobserved, the runner now reports a tempered
  VOI-band posterior alongside the raw one: it builds the band
  `{posterior} ∪ {sigmoid(logit + Δ)}` over the open uncertainty's outcomes
  and prints its midpoint as the *calibrated* confidence, with the band
  shown. The engine must not assert ~99% while recommending the confirmatory
  test that would resolve it.
  - **Anti-entropy guarantee:** the RAW posterior (`P = …`) is unchanged and
    remains what callers rank on; the tempered `Reported (H2 …)` value is
    reporting-only. H2 therefore cannot reorder the differential — zero
    correctness regression by construction.
- **Calibration-regression harness** (`calibration/score.py`) — deterministic,
  offline, per-case scorer + gate (`score` / `diff`). Ranks the differential
  on the raw posterior, scores calibration (Brier, log-loss, ECE, saturation,
  confidently-wrong) on the reported posterior. The `diff` gate fails on ANY
  per-case regression regardless of the aggregate. Frozen 30-case golden
  corpus in `calibration/corpus.json` (artifacts in `cases/case-N/`).
- **Failure-enriched diagnostic pipeline** (`pipeline-diagnostic.workflow.js`)
  — re-run of the ADJ52 pipeline seeded toward the run-3 failure specialties,
  macOS paths, persisting every per-case artifact (rulebook, program, engine
  output, ground truth, judge rationale) for root-cause. Run record:
  `runs/run-4-diagnostic-30case-full.json`; root-cause: `calibration/rootcause-results.json`.

### Result

- H2 gated baseline → H2: **0 regressions**, top-1 accuracy unchanged (0.833),
  saturation 17→12, log-loss −13%; confidently-wrong unchanged at 5 (those are
  H1/H3-driven, not H2). See [ADJ54](../../ADJ54-calibration-regression-harness.md).

## [0.1.0] — 2026-06-03

### Added

- **Counterfactual / VOI / kickback runner panel.** Extends the ADJ51
  runner so that, per query, it surfaces the engine's
  `LRAggregateResult.uncertainties` (which the ADJ51 runner discarded):
  - counterfactual sensitivity — for each candidate value of an
    unresolved `uncertain { … }` marker, the posterior the answer would
    move to if that value were observed, flagged when it flips the
    decision threshold;
  - kickback — via `LRAggregateResult::suggest_kickback`, escalates when
    the plausible posterior band straddles the decision threshold and
    lists the uncertainties to resolve ranked by value-of-information;
  - source disagreement — via `source_disagreements`, surfaces evidence
    whose LR the rulebook's sources disagree on.
- Strict superset of the ADJ51 runner: validated to reproduce ADJ51
  experiment 2's posteriors (e.g. `diagnosis(underlying_pulmonary_pathology)`
  = 82.4%, logodds +1.5436) with the panel gracefully omitted when no
  `uncertain` markers are present.
- `fixtures/uncertainty-demo/` — a minimal rulebook + vignette with one
  open (not-yet-performed biopsy) uncertainty so a bare `cargo run`
  demonstrates the panel.

### Notes

- Consumes only existing `logic-engine` 0.6.0 public APIs
  (`lr_aggregate`, `sigmoid`, `source_disagreements`,
  `LRAggregateResult::suggest_kickback`); no shared-crate edits, so the
  change stays isolated.
