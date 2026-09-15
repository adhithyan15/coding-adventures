- `meteorology/precipitation-alternate-form.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj` (`precip_form(precip, form)`, ONE defining physical form per precipitation
  type — hail → balls_of_ice) and `precipitation-minimum-diameter.adj`. That SAME already-quoted NWS
  Glossary hail sentence lists TWO descriptive terms joined by "or" — a fact the single `form` atom folded
  into one label, keeping only the second term. New `precipitation_alternate_form(precip, form)` table
  decodes the first listed term as its own row: hail → irregular_pellets. Honest abstention on rain, snow,
  sleet, and freezing_rain, whose cited spans each name only one descriptive term with no listed
  alternative. New e2e test file `facts_precipitationalternateform_e2e.rs` (3 tests: forward recall with
  citation, backward recall, honest abstention on rain). No manifest objective, matching
  `precipitation-types.adj`'s own precedent. Second slice from the meteorology/ domain sweep.
