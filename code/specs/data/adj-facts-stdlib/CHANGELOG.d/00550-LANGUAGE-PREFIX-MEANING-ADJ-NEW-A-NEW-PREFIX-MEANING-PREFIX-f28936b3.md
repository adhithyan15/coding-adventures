- `language/prefix-meaning.adj` (new) -- a new `prefix_meaning(prefix, meaning)` table names
  three common English prefixes and what each actually means (un_->negation_or_absence,
  re_->doing_again, dis_->negation_or_reversal), quoted verbatim from Grammarly's "Prefixes:
  Definition and Examples" article -- `trust consensus`, the same source family already used by
  `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/`possessive-noun.adj`/
  `simile-meaning.adj`. Introduces a NEW atom-label convention for this stdlib -- a TRAILING
  UNDERSCORE marks that an atom is a prefix attaching to the front of a word (ADJ atom labels
  cannot contain hyphens), distinct from the underscore-joined multi-word-phrase convention
  `idiom-meaning.adj`/`simile-meaning.adj` already established. Grounds CCSS L.4.4.b. Picked
  using the mandatory full-tree-grep-before-scoping discipline -- `grep -rilE "\bprefix\b|
  \bsuffix\b" code/specs/data/adj-facts-stdlib/` found only `metric-prefixes.adj` (a genuinely
  DIFFERENT topic -- metric UNIT prefixes like kilo-/centi-, not word-morphology) plus incidental
  "suffix" prose mentions in `past-tense-ed-sound.adj`/`plural-s-sound.adj`, confirming a
  completely fresh word-morphology topic before this file was written. WebFetch-verified before
  writing. Honest abstention on "over_" (a real, well-known prefix, but not one of these three
  tabled here). New manifest objective `adj.literacy.3to5.prefix_meaning` (band 3-5, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_prefixmeaning_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled prefix).
