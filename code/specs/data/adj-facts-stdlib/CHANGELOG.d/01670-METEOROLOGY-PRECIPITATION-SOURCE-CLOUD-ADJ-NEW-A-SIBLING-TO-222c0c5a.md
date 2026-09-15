- `meteorology/precipitation-source-cloud.adj` (new) — a sibling to the already-shipped
  `precipitation-types.adj`, `precipitation-minimum-diameter.adj`, and
  `precipitation-alternate-form.adj`. That SAME already-quoted NWS Glossary hail sentence also names the
  originating cloud type — a fact none of those three siblings had a column for. New
  `precipitation_source_cloud(precip, cloud)` table decodes that clause as its own row: hail →
  cumulonimbus. Honest abstention on rain, snow, sleet, and freezing_rain, whose cited spans name no
  originating cloud. New e2e test file `facts_precipitationsourcecloud_e2e.rs` (3 tests: forward recall
  with citation, backward recall, honest abstention on rain). No manifest objective, matching
  `precipitation-types.adj`'s own precedent. Third slice from the meteorology/ domain sweep.
