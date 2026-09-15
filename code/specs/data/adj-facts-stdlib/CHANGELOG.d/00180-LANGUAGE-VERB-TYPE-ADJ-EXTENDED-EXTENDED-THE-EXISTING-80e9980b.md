- `language/verb-type.adj` (extended) -- extended the existing
  `verb_type(type, description)` table from 3 to 4 rows, adding
  stative_verb->describes_a_subjects_state_or_feeling, using the SAME
  already-cited Grammarly "Verbs: Definition and Examples" article. The
  new row is the article's own clean, standalone, single-fact defining
  sentence ("Stative verbs describe a subject's state or feeling..."),
  WebFetch-verified against the live page. Checked and rejected two other
  verb categories on the same page as extension candidates because
  neither has one clean standalone sentence: modal auxiliary verb
  (bundles the definition with a second sentence about not being the
  main verb), phrasal verb (definition bundled with its mechanism).
  Extended `facts_verbtype_e2e.rs` to 5 tests (added direct recall and
  reverse binding for the newly added row). No manifest change (same
  library, no new objective).
