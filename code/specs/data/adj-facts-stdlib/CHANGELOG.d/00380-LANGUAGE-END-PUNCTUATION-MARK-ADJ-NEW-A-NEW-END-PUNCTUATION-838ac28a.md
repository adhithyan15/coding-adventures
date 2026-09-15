- `language/end-punctuation-mark.adj` (new) -- a new `end_punctuation_mark(mark,
  description)` table names three marks that end a sentence and what each actually does
  (period->ends_a_declarative_sentence, question_mark->communicates_that_a_sentence_is_a_
  question, exclamation_point->makes_sentences_exciting), quoted verbatim from Grammarly's
  "Punctuation: The Best Guide to Using Punctuation Marks" article -- `trust consensus`,
  the same tier the sibling `noun-type.adj`/`verb-type.adj`/`pronoun-type.adj`/
  `preposition-type.adj`/`conjunction-type.adj`/`determiner-type.adj` (the other libraries
  in this directory) already use for their Grammarly citations. Grounds CCSS L.K.2.b.
  Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "end_punctuation|punctuation_mark|question_mark|exclamation_point|declarative_sentence"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice -- the second pass
  specifically re-confirmed each sentence's exact wording via the surrounding paragraph).
  Honest abstention on "comma" (a real mark the same page also covers, but one that belongs
  to a DIFFERENT category -- a mid-sentence pause mark, not an end-of-sentence mark like
  the three tabled here). New manifest objective `adj.literacy.k2.end_punctuation_mark`
  (band K-2, `recall` competency, `ccss.ela` coverage root). New e2e test
  `facts_endpunctuationmark_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled punctuation mark).
