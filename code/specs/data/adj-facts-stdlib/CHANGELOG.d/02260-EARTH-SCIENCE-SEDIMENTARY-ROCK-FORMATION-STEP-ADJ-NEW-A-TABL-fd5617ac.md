- `earth-science/sedimentary-rock-formation-step.adj` (new) — a `table` naming the three
  ordered stages of sedimentary rock formation and each stage's numbered position:
  `sedimentary_rock_formation_step(stage, step_number)`, `weathering` → 1, `erosion` → 2,
  `compaction` → 3. The THIRD instance of the "Earth-processes" Major Gap, after
  `geology/earth-layer-matter-behavior.adj` and `weathering-cause-type.adj`, and a genuinely
  NEW axis from every rock table already shipped in this stdlib: `geology/rock-type.adj` and
  `earth-science/rock-types.adj` each name WHAT a rock type forms FROM in one summary phrase;
  `weathering-cause-type.adj` sorts individual weathering causes into physical/chemical;
  `metamorphism-cause.adj` sorts metamorphism causes into a shared effect. None of them states
  the ORDER the process actually runs in — this table does, the same "named stage → a NUMBER
  marking its position" shape `water-cycle.adj` already established for a different Earth
  cycle. Sourced from National Geographic Education's "The Rock Cycle" article (curl-verified
  byte-for-byte against the raw page's own embedded article JSON before writing), the same
  source family already cited at `trust consensus` by `biology/consumer-trophic-level.adj`.
  Deliberately scoped to the sedimentary path only (not named "rock cycle" generally): the same
  page also describes igneous and metamorphic formation, but neither is a multi-stage,
  orderable sequence the way the sedimentary path's own three-sentence, hand-off-style
  paragraph is — only the sedimentary path gets a step count this table can honestly build.
  Honest abstention on `deposition` (a term OTHER K-8 sources use for a stage here, but never
  named by this source's own paragraph, which folds it into the erosion/compaction stages'
  prose instead) and `melting` (a real term the SAME cited page uses, but only for the separate
  igneous-rock path, not this sedimentary sequence). Runs the relation BACKWARD as a genuine
  reverse recall (step number → stage). New `sedimentary-rock-formation-step.query.adj` and
  `facts_sedimentaryrockformationstep_e2e.rs` (3 tests: forward recall + citation check,
  reverse recall, honest abstention on both excluded terms); new manifest objective
  `adj.science.3to5.sedimentary_rock_formation_step` added via a surgical text edit (verified
  with `git diff --stat`, JSON validated before write, and the parsed objective read back out
  to confirm correctness).

