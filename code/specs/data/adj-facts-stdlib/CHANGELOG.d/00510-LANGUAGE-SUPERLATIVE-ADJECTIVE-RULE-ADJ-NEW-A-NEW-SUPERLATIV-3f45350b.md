- `language/superlative-adjective-rule.adj` (new) -- a new `superlative_adjective_rule(rule,
  description)` table names three common English superlative-adjective formation rules and what
  each actually requires (one_syllable_adjective->add_est_suffix,
  one_syllable_consonant_vowel_consonant->double_final_consonant_before_est,
  adjective_ending_in_y->change_y_to_i_before_est), quoted verbatim from Grammarly's "What Are
  Superlative Adjectives? Definition and Examples" article -- `trust consensus`, the same source
  family already used by `sentence-type.adj`/`part-of-speech.adj`/`contraction.adj`/
  `possessive-noun.adj`/`simile-meaning.adj`/`prefix-meaning.adj`/`capitalization-rule.adj`.
  Grounds CCSS L.4.1.a. Picked using the mandatory full-tree-grep-before-scoping discipline --
  `grep -rilE "\bsuperlative\b|superlative_adjective_rule|\bcomparative_adjective\b"
  code/specs/data/adj-facts-stdlib/` found only an incidental unrelated match in
  `meteorology/hurricane-categories.adj`'s header prose, confirming a completely fresh topic
  before this file was written. WebFetch-verified before writing (twice, across two cycles of this
  loop -- the second pass specifically re-confirmed exact wording after an initial fetch surfaced
  a bullet-list fragment for a DIFFERENT, ultimately rejected rule about long adjectives). Honest
  abstention on "three_or_more_syllable_adjective" (a real rule the same article covers -- longer
  adjectives use "most" instead of "-est" -- but whose own supporting text on the page is a
  bullet-list fragment rather than a clean quotable sentence, and which is not one of these three
  tabled here). New manifest objective `adj.literacy.3to5.superlative_adjective_rule` (band 3-5,
  `recall` competency, `ccss.ela` coverage root). New e2e test `facts_superlativeadjectiverule_e2e.rs`
  (3 tests: direct recall, reverse binding, honest abstention on an untabled rule).
