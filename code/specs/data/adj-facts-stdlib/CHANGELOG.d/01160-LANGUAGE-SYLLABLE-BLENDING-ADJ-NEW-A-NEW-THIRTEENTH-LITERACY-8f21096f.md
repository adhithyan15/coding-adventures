- `language/syllable-blending.adj` (new) — a new THIRTEENTH literacy sub-skill library, the exact
  OPPOSITE direction from `syllable-segmentation.adj`: `syllable_blending(syllable_one,
  syllable_two, word)` names the word formed by BLENDING two separate syllables together, the
  same "opposite direction, separate table" shape already proven by `phoneme-blending.adj` versus
  `phoneme-segmentation.adj`. One row: (lap, top, laptop). Discovered via a full inventory of
  Reading Rockets' "Phonological and Phonemic Awareness: In Practice" page's named sections, in
  its distinct "Blending syllables" section (not the "Segmenting syllables" section
  `syllable-segmentation.adj` cites). WebFetch-verified THREE separate times for consistency
  before writing, all byte-identical. New e2e test file `facts_syllableblending_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled pair). New manifest objective
  `adj.literacy.k2.syllable_blending` (173 -> 174 total).
