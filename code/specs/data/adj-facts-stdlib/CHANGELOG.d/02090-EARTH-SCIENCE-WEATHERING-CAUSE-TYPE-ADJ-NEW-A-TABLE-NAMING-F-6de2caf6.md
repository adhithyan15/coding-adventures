- `earth-science/weathering-cause-type.adj` (new) — a `table` naming five causes of weathering
  (heating and cooling, growth of foreign crystals, collision of rock pieces, exposure to acid,
  exposure to oxygen) and which of the two basic weathering types (physical or chemical) each one
  belongs to, quoted verbatim from the U.S. National Park Service's "Weathering and Erosion"
  article (Scotts Bluff National Monument geology-in-action page; curl- and WebFetch-verified
  against the raw page HTML, a new source never before cited in this stdlib):
  `weathering_cause_type(cause, type)`, heating_and_cooling/foreign_crystal_growth/rock_collision →
  physical, acid_exposure/oxygen_exposure → chemical. This is the SECOND instance of the
  "Earth-processes" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1/§5.2), after
  `geology/earth-layer-matter-behavior.adj`, and the first to ground it with a fresh source rather
  than composing two already-shipped tables — composition was confirmed exhaustively dry for this
  gap in the immediately-prior session. Grounds NGSS 4-ESS2-1 (grade 4, band 3-5) — its own
  performance expectation ("evidence of the effects of weathering or the rate of erosion") presumes
  a learner already knows weathering splits into named mechanisms, an axis nothing already shipped
  in this stdlib names (`geology/rock-types.adj` only gives rock FORMATION a combined phrase;
  `earth-science/metamorphism-cause.adj` covers deep METAMORPHISM causes, a different process).
  Runs the relation BACKWARD as a genuine one-to-many recall in both directions (three physical
  causes, two chemical causes). Honest abstention on `erosion` (the cited article's own structure
  treats it as a distinct, LATER process that moves weathering's products away, not a weathering
  type) and on `frost_wedging`/`crystal_wedging` (the article's own detailed breakdown folds that
  exact mechanism into "the growth of foreign crystals" — a separate row would double-count the
  same cause under a second name). New e2e test `facts_weatheringcausetype_e2e.rs`; new manifest
  objective `adj.science.3to5.weathering_cause_type`.

