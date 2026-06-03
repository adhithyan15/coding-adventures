# Changelog — adj52-experiment

All notable changes to the ADJ52 experiment runner.

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
