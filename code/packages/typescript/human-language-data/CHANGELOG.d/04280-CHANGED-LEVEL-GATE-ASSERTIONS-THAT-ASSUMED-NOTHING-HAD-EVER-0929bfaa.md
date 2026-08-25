### Changed - level-gate assertions that assumed nothing had ever been attained

- Spanish attained pre-A1 (HL09 §3.1) when its reinforcement tail closed, which
  falsified four assertions in `tests/level-gate.test.ts` that read a null
  `attained` or a zero `tracksWithAnyLevel`. Each is rewritten to state the
  claim its test is named for rather than the value that claim happened to have:
  the TOUCHES-vs-ATTAINED gap is asserted as a gap, "overstating" is asserted as
  touching higher than attained, the closed criterion is asserted as the gate's
  own `attained` verdict plus an explicit anti-vacuity guard on the per-blocker
  loop, and the authored-but-unrealized rung is scoped to B1-and-above.
- The etymology-waiver counterfactual now compares whole verdicts rather than
  bare shortfalls. A blocker's shortfall is scoped to the level a track is in
  progress at, so once the waiver can carry a track over a rung the two runs
  report shortfalls for two different levels and `unwaived >= waived` stops
  being meaningful. The invariant asserted instead — the waiver can only ever
  leave a track at the same rung or a higher one — holds at every level, and a
  track that loses a rung under the rename now counts as the strongest form of
  the bite the test is looking for.
