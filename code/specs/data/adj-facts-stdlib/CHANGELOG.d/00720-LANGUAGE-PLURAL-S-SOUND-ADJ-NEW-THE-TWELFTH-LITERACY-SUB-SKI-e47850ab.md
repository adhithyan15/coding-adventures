- `language/plural-s-sound.adj` (new) -- the TWELFTH literacy sub-skill library, a sibling
  phonics pattern to `past-tense-ed-sound.adj`: the regular plural -s/-es suffix is pronounced
  one of three different ways depending on the final sound of the singular noun. A new
  `plural_s_sound(word, sound)` table names three worked examples (hats -> s_sound, dogs ->
  z_sound, boxes -> iz_sound), quoted verbatim from Speakspeak's "Pronunciation of 's' and 'es'
  plural endings" article, `trust consensus` -- the same tier as `past-tense-ed-sound.adj`'s
  7ESL citation. WebFetch-verified before writing. Honest abstention on "cats" (also
  /s/-sounded, but not one of the three tabled example words). New manifest objective
  `adj.literacy.k2.plural_s_sound` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_pluralssound_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled word). NOTE: this slice replaced a dropped "science 13th slice:
  water cycle stages" candidate after discovering `earth-science/water-cycle.adj` already
  tables `water_cycle_stage(stage, step_number)` covering the same ground -- see this
  directory's `README.md` for a fuller account of the duplication discovered this cycle
  (also `physics/simple-machines.adj` vs. a dropped teachengineering.org candidate, and
  `earth-science/rock-types.adj` vs. the already-merged `geology/rock-type.adj`).
