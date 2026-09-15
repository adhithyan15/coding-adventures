- `language/diphthong-sound.adj` (new) — a `table` naming the two diphthong lessons of the
  University of Florida Literacy Institute (UFLI) Foundations Toolbox's "Diphthongs and Silent
  Letters Units (Lessons 95-98)" page and the single glided vowel sound each spelling represents:
  `diphthong_sound(diphthong, sound)`, `oi` → `oi_sound`, `oy` → `oi_sound`, `ou` → `ow_sound`,
  `ow` → `ow_sound`. This is the SECOND fresh-WebFetch-sourced literacy instance shipped under
  `wave2-k8-literacy-foundations`, and deliberately a genuinely different phonics angle from
  `digraph-sound.adj` (this loop's first) rather than an extension of it: a DIPHTHONG is one
  glided VOWEL sound, categorically distinct from a consonant digraph's one un-glided consonant
  sound. Confirmed via full-tree grep that no shipped table anywhere already names `diphthong` or
  tables `oi`/`oy`/`ou`/`ow` as a sound-mapping row value. Curl-fetched the raw UFLI page directly
  (not just an AI-summarized WebFetch) and confirmed byte-for-byte before writing this file, the
  same primary .edu source family and `trust authoritative` tier `digraph-sound.adj`/
  `r-controlled-vowel-word.adj` already established for this stdlib. The reverse direction is a
  genuine many-keys-to-one-sound shape — the source's own lesson 95 pairs BOTH `oi` and `oy` with
  the same `/oi/` sound, and lesson 96 pairs BOTH `ou` and `ow` with the same `/ow/` sound — the
  mirror image of `digraph-sound.adj`'s one-key-to-many-sounds `th` case. Lesson 97 ("Vowel Teams
  and Diphthongs Review") and lesson 98 ("kn /n/, wr /r/, mb /m/", the page's own separately
  categorized Silent Letters Unit) are deliberately NOT rows. Honest abstention on `au`/`aw`, a
  spelling many lay phonics materials also call a diphthong, but which UFLI's own broader scope
  and sequence tables under a DIFFERENT unit ("Other Vowel Teams", lesson 93) — this table stays
  scoped to exactly what the cited page itself calls a diphthong. Empirically verified against the
  real built CLI binary before writing the e2e test (forward on all four rows, both many-answer
  reverse cases, and the abstention all behave exactly as designed). New e2e test
  `facts_diphthongsound_e2e.rs` (5 tests); new manifest objective `adj.literacy.k2.diphthong_sound`
  (`recall` competency, matching this library's other plain-lookup literacy facts).

