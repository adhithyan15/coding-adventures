- `language/verb-type.adj` (new) -- a new `verb_type(type, description)` table names three
  verb types and what each actually is
  (action_verb->physical_action_or_activity_that_can_be_seen_or_heard,
  linking_verb->connects_the_subject_to_other_words_in_the_sentence,
  auxiliary_verb->changes_another_verbs_tense_voice_or_mood), quoted verbatim from
  Grammarly's "Verbs: Definition and Examples" article -- `trust consensus`, the same
  source family already used by `sentence-type.adj`/`part-of-speech.adj`/`noun-type.adj`.
  Grounds CCSS L.1.1.e. Picked using the mandatory full-tree-grep-before-scoping discipline
  -- `grep -rilE "\bverb_type\b|\baction_verb\b|\blinking_verb\b|\bauxiliary_verb\b|
  \bhelping_verb\b|\btransitive_verb\b" code/specs/data/adj-facts-stdlib/` found ZERO hits,
  confirming a completely fresh topic before this file was written. WebFetch-verified
  before writing (twice). Honest abstention on "transitive_verb" (a real verb category the
  same page also covers, but only through worked examples and category description rather
  than a single formal definition sentence the way it does for the three tabled here). New
  manifest objective `adj.literacy.k2.verb_type` (band K-2, `recall` competency, `ccss.ela`
  coverage root). New e2e test `facts_verbtype_e2e.rs` (3 tests: direct recall, reverse
  binding, honest abstention on an untabled verb type).
