- `language/contraction.adj` (new) -- a new `contraction(word, expansion)` table names three
  contractions and the two-word phrase each stands for (dont->do_not, cant->can_not,
  wont->will_not), quoted verbatim from Grammarly's "What Are Contractions in Writing?
  Definition and Examples" article -- `trust consensus`, the same source family already used
  by `sentence-type.adj`/`part-of-speech.adj`. Grounds CCSS L.2.2.c. Picked using the mandatory
  full-tree-grep-before-scoping discipline -- `grep -rilE "\bcontraction\b|\bapostrophe\b"
  code/specs/data/adj-facts-stdlib/` found ZERO hits, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing. Uses apostrophe-free
  underscore-joined atom labels for both columns (ADJ atom labels cannot contain punctuation),
  mirroring `sentence-type.adj`'s established discipline for punctuation-bearing content; the
  `source` citation string still quotes the original punctuated text ("don't = do not") so the
  mapping stays independently checkable. Deliberately keeps the source's own two-word expansion
  "can not" for "can't" rather than silently "correcting" it to the more common single-word
  spelling "cannot". Honest abstention on "shouldnt" (a real contraction, "should not," but not
  one of these three tabled rows). New manifest objective `adj.literacy.k2.contraction` (band
  K-2, `recall` competency, `ccss.ela` coverage root). New e2e test `facts_contraction_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled contraction).
