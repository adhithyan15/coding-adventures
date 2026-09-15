- `physics/energy-form-family.adj` (new) — a sibling to the already-shipped `energy-forms.adj`
  (`energy_form_token(form, token)`, ONE defining token per named energy form): a new
  `energy_form_family(form, family)` table names which of the EIA page's two families —
  potential or kinetic — each of the same eight forms belongs to. `energy-forms.adj`'s own
  header already summarized this split, but it was re-verified LIVE via WebFetch this cycle
  against the same already-cited EIA "Forms of energy" page: "Many forms of energy exist, but
  energy is either potential energy or kinetic energy," with chemical/mechanical/nuclear/
  gravitational listed under "Potential energy" and radiant/thermal/motion/electrical under
  "Kinetic energy" — byte-identical to the existing header's claim. Honest abstention on `sound`
  (a real form the same EIA page also lists under Kinetic, but not one of `energy-forms.adj`'s
  eight tabled forms). New e2e test file `facts_energyformfamily_e2e.rs` (3 tests: forward recall
  of all eight with citation, backward recall of all four kinetic forms, honest abstention on
  sound). No manifest objective, matching `energy-forms.adj`'s own precedent of not having one.
  Fourth and LAST slice from the physics/ sweep tranche — physics/ (21 files) is now fully
  exhausted except a weak, not-recommended `circuit-parts.adj` grab-bag.
