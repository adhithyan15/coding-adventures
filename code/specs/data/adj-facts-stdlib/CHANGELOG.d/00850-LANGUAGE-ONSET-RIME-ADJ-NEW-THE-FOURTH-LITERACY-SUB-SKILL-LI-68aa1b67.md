- `language/onset-rime.adj` (new) -- the FOURTH literacy sub-skill library in the ADJ stdlib,
  deliberately different in shape from the three prior ones: `word-families.adj` derives RHYMING
  (RF.K.2.a), `syllable-count.adj` recalls a SYLLABLE COUNT (RF.K.2.b), and `initial-sound.adj`
  recalls a BEGINNING sound (RF.K.2.d) -- this one recalls how a single-syllable word splits into
  its ONSET (sound(s) before the vowel) and RIME (the vowel and everything after) as a pure
  lookup, `onset_rime(word, onset, rime)`, a THREE-column table (the shape
  `metrology/si-base-units.adj` already established). Quoted verbatim from Reading Rockets'
  "Tuning In to the Sounds in Words" article, WebFetch-verified TWICE for consistency before
  writing: "sleep could be broken into /sl/ and /eep/" and "Here are two ways to break up the
  word blast: Onset (bl) – Rime (ast)". Deliberately scoped to ONLY these two words the cited
  page splits explicitly -- honestly narrow (mirroring `syllable-count.adj`'s and
  `initial-sound.adj`'s precedent) rather than inventing a split for an uncited word. Grounds
  CCSS RF.K.2.c. New manifest objective `adj.literacy.k2.onset_rime` (band K-2, `recall`
  competency). New e2e test `facts_onsetrime_e2e.rs` (3 tests: direct segmenting recall, reverse
  blending recall, honest abstention on an unshipped word).
