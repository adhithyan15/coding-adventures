- `language/phoneme-addition.adj` (new) — a new TENTH literacy sub-skill library, the narrowest
  sibling of `phoneme-blending.adj`: `phoneme_addition(sound_one, sound_two, word)`. Same
  direction as `phoneme-blending.adj` (sounds combining INTO a word), but for exactly TWO sounds
  rather than three, a distinct arity the cited page treats as its own named skill ("Adding
  sounds," distinct from "Blending sounds"). One row: (i, s, ice). Discovered via the SAME
  already-vetted Reading Rockets "In Practice" page's "Adding sounds" section, explicitly flagged
  as a future candidate in `phoneme-blending.adj`'s own header when that library shipped.
  WebFetch-verified THREE separate times across this session for consistency, all byte-identical.
  New e2e test file `facts_phonemeaddition_e2e.rs` (3 tests: direct recall, reverse binding into
  both sounds, honest abstention on an untabled addition). New manifest objective
  `adj.literacy.k2.phoneme_addition` (170 -> 171 total).
