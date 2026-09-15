- `language/part-of-speech.adj` (extended) -- extended the existing
  `part_of_speech(word, category)` table from 3 to 5 rows, adding
  quietly->adverb and against->preposition, using the SAME already-cited
  Grammarly "The 8 Parts of Speech" article (which covers all 8 parts of
  speech; only 3 were originally tabled). Both new rows are the article's
  own clean, standalone, single-fact example sentences ("I entered the
  room quietly." / "I left my bike leaning against the garage."),
  WebFetch-verified against the live page including the surrounding
  sentences to confirm neither is folded into a longer bundled passage.
  Checked and rejected the remaining three parts of speech on the same
  page as extension candidates because none has one clean standalone
  sentence: pronoun (bundles two separate quoted sentences with framing),
  conjunction (one sentence covers two conjunctions -- "and" and "but" --
  bundled together), interjection (each example is bundled with its own
  punctuation demonstration). Extended `facts_partofspeech_e2e.rs` to 5
  tests (added direct recall and reverse binding for the two newly added
  rows). No manifest change (same library, no new objective).
