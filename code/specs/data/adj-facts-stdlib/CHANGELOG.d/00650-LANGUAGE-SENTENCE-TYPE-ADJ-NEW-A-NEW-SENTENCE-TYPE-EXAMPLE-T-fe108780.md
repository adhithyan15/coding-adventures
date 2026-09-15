- `language/sentence-type.adj` (new) -- a new `sentence_type(example, type)` table names four
  example sentences and which of the four grammatical sentence types each one is (declarative,
  interrogative, imperative, exclamatory), quoted verbatim from Grammarly's "4 Types of
  Sentences to Know, With Examples" article -- `trust consensus`, the same tier this stdlib
  already reserves for other non-.gov language sources (7ESL, Speakspeak). Grounds CCSS
  L.1.1.j. Picked using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE
  "\bsentence.type|\bdeclarative\b|\binterrogative\b|\bimperative\b" code/specs/data/adj-facts-stdlib/`
  confirmed ZERO existing coverage before this file was written. WebFetch-verified before
  writing. Uses SHORT ATOM-STYLE labels for the `example` column (not full-sentence string
  literals), mirroring `vocabulary-in-context.adj`'s established discipline of avoiding the ADJ
  query-grammar limitation where a quoted-string literal works as a table row VALUE but not as
  a query ARGUMENT. Honest abstention on "the cat sat on the mat" (a real, well-formed
  declarative sentence, but not one this specific cited page names). New manifest objective
  `adj.literacy.k2.sentence_type` (band K-2, `recall` competency, `ccss.ela` coverage root).
  New e2e test `facts_sentencetype_e2e.rs` (3 tests: direct recall, reverse binding, honest
  abstention on an untabled sentence).
