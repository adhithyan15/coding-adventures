- `language/author-purpose.adj` (new) -- a new `author_purpose(purpose,
  description)` table names the three classic reasons an author writes
  something (persuade->convince_the_reader_of_the_merits_of_a_particular_point_of_view,
  inform->enlighten_the_readership_about_a_real_world_topic,
  entertain->keep_things_as_interesting_as_possible), each quoted verbatim
  from its own standalone sentence in LiteracyIdeas' "The Author's
  Purpose: Ultimate Guide for Teachers and Students" article -- `trust
  consensus`, a MULTI-SOURCE-STYLE table (see `point-of-view.adj`,
  `comma-rule.adj`, `figurative-language-type.adj`). Picked after checking
  12 not-yet-reviewed literacy tables this window (clause-type.adj,
  comma-rule.adj, point-of-view.adj, sound-device-type.adj,
  figurative-language-type.adj, sentence-type.adj, idiom-meaning.adj,
  simile-meaning.adj, past-tense-ed-sound.adj, plural-s-sound.adj,
  silent-e-word.adj, r-controlled-vowel-word.adj -- none extendable, each
  already documenting in its own header exactly why its rejected
  candidates don't qualify), then researching author's purpose as a fresh
  topic (Grammarly has no dedicated page; pivoted through
  education.com/study.com/twinkl.com, which bundle purpose with genre
  examples in one sentence, before LiteracyIdeas' page succeeded with
  clean parallel single-fact sentences). Honest abstention on `describe`:
  a real fourth purpose the same article also names, but its defining
  sentence is framed as a photograph comparison rather than the same
  parallel "when an author's purpose is to X, they Y" pattern the three
  tabled here share, and the classic PIE mnemonic this table grounds names
  only these three as the canonical peer set. New manifest objective
  `adj.literacy.k2.author_purpose` (band K-2, `recall` competency,
  `ccss.ela` coverage root; 161 objectives total, up from 160). New e2e
  test `facts_authorpurpose_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention).
