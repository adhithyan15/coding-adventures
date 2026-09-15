- `physics/optic-focal-point-location.adj` (new) — a sibling to the already-shipped `lens-types.adj`
  (`optic_action(optic, action)`, whether each of four basic optical elements converges or diverges
  parallel light — convex_lens → converges_light, concave_lens → diverges_light, concave_mirror →
  converges_light, convex_mirror → diverges_light). That table's own header already quotes, verbatim,
  an OpenStax sentence for THREE of the four rows that states WHERE the focal point sits — a fact the
  action-only schema had no room for. New `optic_focal_point_location(optic, location)` table:
  convex_lens → opposite_side_of_the_lens, concave_mirror → same_side, convex_mirror →
  behind_the_mirror. Genuinely new information beyond the parent's converge/diverge action: it
  distinguishes a REAL focal point (convex_lens, concave_mirror) from a VIRTUAL one (convex_mirror),
  the core distinction for reasoning about real vs. virtual image formation. Honestly narrower than
  its parent: honest abstention on concave_lens, whose already-cited span describes ray divergence but
  never states a focal-point location. New e2e test file `facts_opticfocalpointlocation_e2e.rs`
  (3 tests: forward recall with citation, backward recall of the behind-the-mirror element, honest
  abstention on concave_lens). No manifest objective, matching `lens-types.adj`'s own precedent. Third
  and final slice from the direct re-examination of the physics/ MODERATE candidates the sweep had
  deprioritized — closes out that batch (the fourth, `circuit_part_input_energy`, was confirmed a
  genuine dead end rather than shipped).
