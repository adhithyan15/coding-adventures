- `music/solfege-alt-name.adj` (new) — a sibling to the already-shipped `solfege.adj`
  (`solfege_degree(syllable, degree)`, ONE scale-degree number per syllable). That table's own
  `source` field already quotes, verbatim, an alternate spelling or name for two of the seven
  syllables — that the degree-only schema had no room for: do's cited span also states "(spelt doh
  in tonic sol-fa)" and ti's cited span also states "(or si)". New `solfege_alt_name(syllable,
  alt_name)` table: do → doh, ti → si. Honest abstention on the other five syllables (re, mi, fa,
  sol, la), whose own cited span states no alternate spelling or name. New e2e test file
  `facts_solfegealtname_e2e.rs` (3 tests: forward recall with citation, backward recall from a
  bound alternate, honest abstention on mi). No manifest objective, matching `solfege.adj`'s own
  precedent of not having one. First and only strong candidate from the music/ sweep tranche (the
  header's own scale-degree-name truth table — tonic, supertonic, mediant, etc. — was checked and
  confirmed to be non-verbatim author-compiled prose, not a quoted source span, so it's out of
  scope); music/ (1 table) is now effectively closed after this ships.
