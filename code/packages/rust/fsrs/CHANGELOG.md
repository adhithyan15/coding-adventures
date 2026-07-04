# Changelog — fsrs

## 0.1.0 — Unreleased

Initial release: a zero-dependency, forward-only FSRS-6 scheduler, created to
remove the third-party `fsrs` crate (and its `burn` tensor-framework dependency
tree) from the Engram stack as part of the Engram zero-dependency program
(`code/specs/engram-zero-dep-plan.md`, Phase B).

### Added

- `FSRS` scheduler: `new`, `next_states`, `memory_state_from_sm2`,
  `init_stability`, `parameters`.
- Free functions/constants matching the upstream surface Engram consumes:
  `current_retrievability`, `DEFAULT_PARAMETERS`, `FSRS6_DEFAULT_DECAY`,
  `FSRS5_DEFAULT_DECAY`.
- Types `MemoryState`, `ItemState`, `NextStates`, `FsrsError`.
- Faithful scalar transcription of the upstream 6.6.1 forward pass: power
  forgetting curve, initial stability/difficulty, mean reversion + linear
  damping difficulty update, the three stability regimes (after-success,
  after-failure, short-term), first-review seeding, parameter upgrade
  (`check_and_fill_parameters`) and clipping (`clip_parameters_in_place`).

### Verified

- A throwaway cross-check test (removed with the upstream dev-dependency) asserted
  **5,900+ comparisons** — `next_states` across a grid of retention × elapsed-days
  × random memory states, plus `memory_state_from_sm2` and
  `current_retrievability` over random inputs — all match the live upstream
  `fsrs` 6.6.1 within a `1e-4` relative tolerance. The exact upstream outputs for
  representative cases are frozen as unit-test snapshots so the numeric behaviour
  stays locked without the third-party crate.
