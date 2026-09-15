- `physics/band-secondary-use.adj` (new) — a sibling to the already-shipped `em-spectrum.adj`
  (`band_use(band, application)`, ONE representative everyday use per EM band): a new
  `band_secondary_use(band, application)` table names a SECOND everyday use the SAME already-cited
  NASA "Imagine the Universe!" page states for a band, decoded from text already sitting unused
  inside `em-spectrum.adj`'s own provenance block — no new WebFetch. Two rows: microwave →
  astronomy, x_ray → airport_security. Honest abstention on the other five bands (radio, infrared,
  visible, ultraviolet, gamma_ray), whose own cited spans name only one use each. New e2e test file
  `facts_bandsecondaryuse_e2e.rs` (3 tests: forward recall of both with citation, backward recall
  from a bound application, honest abstention on radio). No manifest objective, matching
  `em-spectrum.adj`'s own precedent of not having one. Third slice from the physics/ sweep tranche.
