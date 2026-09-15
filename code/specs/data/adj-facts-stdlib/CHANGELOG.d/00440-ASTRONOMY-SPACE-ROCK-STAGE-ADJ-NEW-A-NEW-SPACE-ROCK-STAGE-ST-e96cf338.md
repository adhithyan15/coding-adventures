- `astronomy/space-rock-stage.adj` (new) -- a new `space_rock_stage(stage, description)` table
  names three stages a single rocky object passes through, not three different kinds of object
  (meteoroid->still_a_rock_in_space,
  meteor->called_a_fireball_or_shooting_star_when_it_burns_up_in_the_atmosphere,
  meteorite->survives_the_atmosphere_and_hits_the_ground), quoted verbatim from NASA Science's
  "Meteors & Meteorites" page -- `trust authoritative`, the same tier the sibling
  `comet-part.adj`/`solar-eclipse-type.adj` (the other libraries in this directory) already use
  for their NASA citations. Grounds NGSS 3-5 space-systems standards. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "meteoroid|meteorite|meteor_type|
  space_rock_type" code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely
  fresh topic before this file was written. WebFetch-verified before writing (twice -- the
  second pass specifically re-checked the meteoroid sentence's exact wording against its
  surrounding paragraph, since a first-pass fetch can silently paraphrase a short or
  awkwardly-worded source sentence). Honest abstention on "asteroid" (a real object the same
  page mentions in passing -- "Meteoroids range in size from dust grains to small asteroids" --
  but never defines in a sentence of its own on this page, unlike the three stages tabled here).
  New manifest objective `adj.science.3to5.space_rock_stage` (band 3-5, `recall` competency,
  `ngss` coverage root). New e2e test `facts_spacerockstage_e2e.rs` (3 tests: direct recall,
  reverse binding, honest abstention on an untabled term).
