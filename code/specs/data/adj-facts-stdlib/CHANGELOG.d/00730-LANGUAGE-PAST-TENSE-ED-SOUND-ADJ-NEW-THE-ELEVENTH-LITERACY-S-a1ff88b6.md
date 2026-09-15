- `language/past-tense-ed-sound.adj` (new) -- the ELEVENTH literacy sub-skill library, and a
  genuinely new phonics pattern beyond CCSS RF.K.2's five parts and the spelling/whole-text/
  vocabulary slices already shipped: the regular -ed past-tense suffix is spelled the same way
  every time, but PRONOUNCED one of three different ways depending on the final sound of the
  base verb. A new `past_tense_ed_sound(word, sound)` table names three worked examples (walked
  -> t_sound, lived -> d_sound, wanted -> id_sound), quoted verbatim from 7ESL's "Pronunciation
  of ED: Past Tense Pronunciation for Regular Verbs" article, `trust consensus` -- a general
  ESL-learning site, the same tier this stdlib already reserves for its other non-.gov language
  sources (Wikipedia's Greek-alphabet/Morse-code entries). WebFetch-verified before writing.
  Honest abstention on "played" (also /d/-sounded, but not one of the three tabled example
  words). New manifest objective `adj.literacy.k2.past_tense_ed_sound` (band K-2, `recall`
  competency, `ccss.ela` coverage root). New e2e test `facts_pasttenseedsound_e2e.rs` (3 tests:
  direct recall, reverse binding, honest abstention on an untabled word).
