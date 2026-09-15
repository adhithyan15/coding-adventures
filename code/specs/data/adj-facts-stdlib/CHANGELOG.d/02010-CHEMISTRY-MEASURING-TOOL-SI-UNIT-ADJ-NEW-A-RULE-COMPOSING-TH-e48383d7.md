- `chemistry/measuring-tool-si-unit.adj` (new) — a `rule` composing the already-shipped
  `measuring_tool(tool, quantity)` table (`chemistry/measuring-tools.adj`, Chemistry LibreTexts)
  with the already-shipped `si_base_unit(quantity, unit, symbol)` table
  (`metrology/si-base-units.adj`, NIST) to DERIVE `measuring_tool_si_unit(tool, unit, symbol)` —
  WHICH internationally-standardized SI unit a lab measuring tool's own reading is expressed in,
  not just which quantity it measures. Neither already-shipped table states outright "a
  thermometer's reading is standardized in kelvin", but measuring-tools' specific tool→quantity
  assignment run through si-base-units' general metrology principle for that quantity answers it
  directly: ruler → meter ("m"), balance → kilogram ("kg"), thermometer → kelvin ("K"). This is
  the FOURTH `rule`-based CAUSAL-COMPOSITION library in this loop's science curriculum sweep,
  following the same "compose two independently-citable facts into a derived, dual-cited
  conclusion" discipline `physics/heat-causes-phase-change.adj`,
  `physics/force-causes-acceleration.adj`, and `geology/earth-layer-matter-behavior.adj` already
  established — the SECOND cross-directory instance of that discipline (chemistry + metrology,
  not two files in the same subject directory), reusing the SAME cross-directory import/
  query-placement shape `earth-layer-matter-behavior.adj` already proved out, and the FIRST to
  ground the "observation and measurement" Major Gap (ADJ-STDLIB-COVERAGE.md §5.1) rather than an
  Earth-processes or physics-mechanics one. Zero new WebFetch: both composed tables were already
  shipped and already cited (and predate this composition — `si-base-units.adj` shipped in PR
  #8786, well before `measuring-tools.adj` in PR #10479, so the pairing had simply gone
  unnoticed, not been tried and rejected). Honest abstention on `graduated_cylinder` (`volume`) —
  volume is not one of `si_base_unit`'s seven keyed BASE quantities; it is a DERIVED SI unit (m³),
  independently confirmed by the already-shipped sibling `metrology/si-derived-units.adj`
  (`si_derived_unit(volume, "m3")`), so forcing the join would conflate a base quantity with a
  derived one — a real metrology distinction the cited NIST source itself draws, and exactly the
  kind of invented mapping this stdlib's honest-abstention discipline forbids. A scoping pass this
  cycle surveyed ~349 cross-table atom collisions across all 310 shipped `table` declarations in
  adj-facts-stdlib (script-assisted, the same census method the prior cycle used) looking for a
  genuine general-principle + specific-instance pair addressing one of the STILL FULLY UNTOUCHED
  causal-composition gaps (observation/measurement, experiments, models, matter/energy systems,
  heredity, ecosystems, engineering design); several near-misses were found and rejected as too
  thin or coincidental (e.g. `biology/genetic-code` → `biology/amino-acids` is a real literal-key
  join but is a naming/decode BRIDGE, not a causal explanation; `physics/energy-sources`
  and `physics/energy-form-family` both use the atom `nuclear` but in two DIFFERENT senses — an
  energy SOURCE vs. an energy FORM — composing them would conflate the two; `biology/animal-
  habitat` and `biology/animal-survival-adaptation` share only ONE overlapping animal
  (`polar_bear`), too thin a join to generalize from) before `measuring-tools.adj` +
  `si-base-units.adj` surfaced as the clean match. Grounds NGSS science-practice observation/
  measurement (a tool's raw reading is only useful to another scientist once tied to the ONE unit
  everyone has agreed to measure that quantity in — the entire point of the SI system). New
  manifest objective `adj.science.6to8.measuring_tool_si_unit` (band 6-8, `infer` competency,
  matching the three prior rule-derived facts' own precedent). New e2e test file
  `facts_measuringtoolsiunit_e2e.rs` (3 tests: derivation with dual citations, reverse binding,
  honest abstention). Empirically verified the composition in a scratch dir against the real built
  CLI before writing the shipped files. 117th content slice overall.
