- `physics/mirror-focal-length-sign.adj` (new) — a sibling to the already-shipped `lens-types.adj`
  (`optic_action(optic, action)`, whether each of four basic optical elements converges or diverges
  parallel light — convex_lens → converges_light, concave_lens → diverges_light, concave_mirror →
  converges_light, convex_mirror → diverges_light). That table's own header already quotes,
  verbatim, an OpenStax sentence for EACH of the two mirror rows that states the SIGN of the
  mirror's focal length: "The focal length f of a concave mirror is positive, since it is a
  converging mirror" and "The focal length and power of a convex mirror are negative, since it is a
  diverging mirror." New `mirror_focal_length_sign(mirror, sign)` table: concave_mirror → positive,
  convex_mirror → negative. Honestly narrower than its parent: honest abstention on `convex_lens`
  and `concave_lens`, whose already-cited spans describe ray behavior but never state a sign
  convention. New e2e test file `facts_mirrorfocallengthsign_e2e.rs` (3 tests: forward recall with
  citation, backward recall of the negative-sign mirror, honest abstention on convex_lens). No
  manifest objective, matching `lens-types.adj`'s own precedent. Third and final STRONG slice from
  the physics/ sweep, closing out simple-machine-function.adj (PR #11523),
  heat-transfer-example.adj (PR #11531), and this one.
