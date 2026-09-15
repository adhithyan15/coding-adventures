- `meteorology/precipitation-minimum-diameter.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj` (`precip_form(precip, form)`, ONE defining physical form per
  precipitation type): a new `precipitation_min_diameter(precip, min_diameter_mm)` table names
  the numeric diameter threshold the SAME NOAA NWS Glossary states for a type, decoded from spans
  already sitting unused inside `precipitation-types.adj`'s own provenance block — no new
  WebFetch. Two rows: rain → 0.5, hail → 5, both read off the SAME already-quoted NWS Glossary
  spans that table's single-form schema had no room for. Deliberately narrow: only these two of
  the table's five precipitation types have a diameter figure in the already-cited spans. New e2e
  test file `facts_precipitationmindiameter_e2e.rs` (3 tests: both-term recall with citation,
  backward recall from a bound diameter, honest abstention on snow). No manifest objective,
  matching `precipitation-types.adj`'s own precedent of not having one.
