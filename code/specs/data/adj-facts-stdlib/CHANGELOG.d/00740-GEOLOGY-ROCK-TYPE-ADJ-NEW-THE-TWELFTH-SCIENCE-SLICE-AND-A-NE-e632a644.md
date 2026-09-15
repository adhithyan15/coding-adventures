- `geology/rock-type.adj` (new) -- the TWELFTH science slice, and a new file in the ALREADY-
  SHIPPED `geology/` directory (alongside `earth-layers.adj` and `mineral-hardness.adj`). A new
  `rock_type(rock, formation_process)` table names the three basic classes geologists sort ALL
  rocks into and HOW each class forms (igneous -> crystallized_molten_rock, sedimentary ->
  deposited_weathered_material, metamorphic -> heat_and_pressure_transformation). UNLIKE
  `earth-layers.adj` (four rows sharing one USGS publication), this table's three rows each cite
  a DIFFERENT USGS FAQ page ("What are igneous/sedimentary/metamorphic rocks?"), so it uses the
  multi-source pattern `ocean-observing-instruments.adj`/`fable-moral.adj` established: the
  table-level citation carries the primary (igneous) source, and the other two rows' own
  distinct citations are documented in the file's header prose. All three quotes WebFetch-
  verified before writing. `trust authoritative` -- every row's own source page is a primary
  U.S. government (USGS, .gov) source. Honest abstention on "coal" (a real rock, but not one of
  the three rock-type classes tabled here). New manifest objective `adj.science.3to5.rock_type`
  (band 3-5, `recall` competency, `ngss` coverage root). New e2e test `facts_rocktype_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled rock).
