- `language/phoneme-segmentation.adj` (new) — a new ELEVENTH literacy sub-skill library, the
  exact OPPOSITE direction from `phoneme-blending.adj`: `phoneme_segmentation(word, sound_one,
  sound_two, sound_three)`. Where `phoneme-blending.adj` composes three sounds into a word, this
  decomposes a word into its three sounds. One row: (feet, f, ee, t). Discovered via a fresh
  WebFetch of the same already-vetted Reading Rockets "In Practice" page's "Segmenting sounds in
  a syllable" section, not yet explored by any prior slice. WebFetch-verified THREE separate
  times for consistency before writing, all byte-identical. New e2e test file
  `facts_phonemesegmentation_e2e.rs` (3 tests: direct recall of the three sounds, reverse
  binding into the word, honest abstention on an untabled word). New manifest objective
  `adj.literacy.k2.phoneme_segmentation` (170 -> 171 total).
