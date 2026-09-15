- `physics/force-causes-acceleration.adj` (new) — the SECOND causal-explanation library in the
  ADJ stdlib's science domain, following `heat-causes-phase-change.adj`'s precedent. A `rule`
  composes Newton's second law's own general statement (from the already-shipped
  `newton-laws.adj`) with a specific force→example fact (from the already-shipped `forces.adj`)
  to DERIVE `force_causes_acceleration(force, example)` — a genuine CROSS-FILE composition
  reusing TWO already-verified NASA citations with zero new sourcing work. Grounds NGSS MS-PS2-2.
  Deliberately scoped to the second law only (F = m·a governs any force's relationship to
  acceleration) — the first law (inertia) and third law (action-reaction) describe different
  causal relationships, left for a later pass. New manifest objective
  `adj.science.6to8.force_causes_acceleration` (band 6-8, uses the `infer` competency for a
  rule-derived fact). New e2e test `facts_forcecausesacceleration_e2e.rs` (3 tests: direct
  derivation with dual citations, reverse binding, honest abstention on an untabled force).
