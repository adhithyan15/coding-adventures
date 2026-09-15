- `geology/earth-layer-matter-behavior.adj` (new) — a `rule` composing the already-shipped
  `has_state(layer, state)` table (`geology/earth-layers.adj`, USGS) with the already-shipped
  `matter_state(state, property)` table (`chemistry/states-of-matter.adj`, NASA GRC) to DERIVE
  `earth_layer_matter_behavior(layer, behavior)` — WHY each of Earth's internal layers behaves the
  way it does, not just WHAT state it is in. Neither already-shipped table states outright "the
  outer core takes the shape of its container", but Earth-layers' specific state assignment run
  through states-of-matter's general chemistry principle for that state answers it directly:
  outer_core (liquid) → takes_shape_of_container, inner_core (solid) → fixed_shape. This is the
  THIRD `rule`-based CAUSAL-EXPLANATION library in this loop's science curriculum sweep, following
  the same "compose two independently-citable facts into a derived, dual-cited conclusion"
  discipline `physics/heat-causes-phase-change.adj` and `physics/force-causes-acceleration.adj`
  already established — the first CROSS-DIRECTORY instance of that specific discipline (geology +
  chemistry, not two files in the same subject directory), reusing the SAME cross-directory import
  shape `earth-science/season-start-month-number.adj` already proved out. Zero new WebFetch: both
  composed tables were already shipped and already cited. Honest abstention on `crust` (`rigid`)
  and `mantle` (`semi_solid`) — neither is a literal match for any of `matter_state`'s three keyed
  states (solid/liquid/gas), so forcing either to join would assert a behavior neither already-cited
  source states. Grounds NGSS MS-ESS2-1 ("the flow of energy that drives" Earth's material cycling
  — the liquid outer core's ability to flow is exactly why it can convect and generate Earth's
  magnetic field). New manifest objective `adj.science.6to8.earth_layer_matter_behavior` (band 6-8,
  `infer` competency, matching `force-causes-acceleration.adj`'s own precedent for a rule-derived
  fact). New e2e test file `facts_earthlayermatterbehavior_e2e.rs` (3 tests: derivation with dual
  citations, reverse binding, honest abstention). Empirically verified the composition in a scratch
  dir against the real built CLI before writing the shipped files. 116th content slice overall.
