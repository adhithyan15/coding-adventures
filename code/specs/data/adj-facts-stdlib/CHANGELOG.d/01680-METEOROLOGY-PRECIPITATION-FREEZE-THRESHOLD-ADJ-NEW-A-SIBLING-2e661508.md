- `meteorology/precipitation-freeze-threshold.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj` (`precip_form(precip, form)`, ONE defining physical form per precipitation
  type — freezing_rain → glaze_of_ice). That table's own header already quotes, verbatim, an NWS sentence
  stating the temperature at or below which freezing rain refreezes on contact (32 degrees F) — a numeric
  figure the single `form` atom had no room for. New `precipitation_freeze_threshold_f(precip,
  temperature_f)` table decodes that clause as its own row: freezing_rain → 32. Honest abstention on rain,
  snow, sleet, and hail, whose cited spans state no numeric freeze threshold. New e2e test file
  `facts_precipitationfreezethreshold_e2e.rs` (3 tests: forward recall with citation, backward recall,
  honest abstention on rain). No manifest objective, matching `precipitation-types.adj`'s own precedent.
  Fourth slice from the meteorology/ domain sweep. (The remaining MODERATE candidate,
  `hurricane_damaged_component` off `hurricane-categories.adj`/`hurricane-category-home-damage.adj`, was
  re-inspected and confirmed a dead end: its five structural-effect spans require inconsistent
  interpretive judgment calls to decompose — some are clean comma/and-joined noun lists, others bundle a
  severity choice ("damage or removal") or use distinct verb phrases per row rather than a shared list —
  so no honest single-schema decomposition was possible.)
