- `physics/energy-conversion-example.adj` (new) — a `table` naming four everyday processes and
  which form of energy goes IN and which comes OUT of each, grounding the U.S. EIA's own "law of
  conservation of energy" statement that energy is never created or destroyed, only changed from
  one form into another: `energy_conversion_example(process, energy_in, energy_out)`,
  wood_burning_in_fireplace → chemical/thermal, car_engine_burning_gasoline → chemical/mechanical,
  solar_photovoltaic_cell → radiant/electrical, bicycle_going_downhill → gravitational/motion. This
  is the FIRST genuine instance of the "matter/energy systems" Major Gap (ADJ-STDLIB-COVERAGE.md
  §5.1) — a prior cycle's only prior touch, the already-shipped `physics/energy-forms.adj` and
  `physics/energy-sources.adj`, named energy FORMS and SOURCES but never a CONVERSION between
  forms, so this gap had zero real instances (unlike its sibling K-8-science gaps, each of which
  already had one). Two of the four rows decode a clause already sitting unused inside
  `energy-forms.adj`'s own already-cited EIA "Forms of energy" page ("chemical energy is converted
  to thermal energy when people burn wood in a fireplace"; "the gravitational energy is converting
  to motion energy" on a bicycle going downhill); the other two come from that page's sibling
  "Laws of energy" page — a genuinely NEW fetch this cycle, not previously cited anywhere in this
  stdlib — under its own "Energy is neither created nor destroyed" heading ("A car engine burns
  gasoline, converting the chemical energy in gasoline into mechanical energy. Solar photovoltaic
  cells change radiant energy from the sun into electrical energy."). Honest abstention documented
  on a near-collision deliberately avoided: the "Forms of energy" page's fireplace sentence also
  mentions "burn gasoline in a car's engine" as a second chemical→thermal example, but the "Laws of
  energy" page gives car engines their own dedicated chemical→MECHANICAL sentence instead — both
  physically true of a real engine (motion output plus waste heat), but a single `process` key
  cannot carry two different outputs without contradiction, so this table keeps
  `car_engine_burning_gasoline` mapped to the "Laws of energy" page's dedicated sentence and scopes
  the fireplace clause to `wood_burning_in_fireplace` only. Also honestly abstains on any process
  not one of these four (e.g. a toaster). `trust authoritative` (eia.gov, the same tier
  `energy-forms.adj`/`energy-sources.adj`/`energy-form-family.adj` already use); the "Laws of
  energy" page's own footer credits its content to "National Energy Education Development Project
  (public domain)", reproduced transparently in the header rather than hidden, without changing the
  trust tier (the citable, re-fetchable artifact is the EIA .gov page itself). Added manifest
  objective `adj.science.3to5.energy_conversion_example` (recall, NGSS, band 3-5, matching NGSS
  4-PS3-4's own grade-4 energy-conversion standard). New e2e test `facts_energyconversionexample_e2e.rs`
  (3 tests: direct recall across all four rows, reverse one-to-many recall on the shared
  `chemical` energy_in, and honest abstention on `toaster`).
