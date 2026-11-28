# Changelog

## Unreleased

- Add a pure evacuation planner over the admitted immutable page map. Produce
  deterministic source/destination pairs, honor target and excluded-block
  reserve boundaries, and return zero plans on every refusal.
- Add a strict model-first shared vector, exhaustive reachable-map/layout/mask
  comparisons, bounded fixture and history fuzzers, and three named mutation
  controls.
- Add `PlanDrain` for readable retirement. It shares the bounded copy
  enumeration, accepts an excluded source and empty or all-live targets, and
  leaves erase eligibility to the caller. Add its strict canonical vector,
  ledger/zero-budget composition and three clean-controlled mutations.
