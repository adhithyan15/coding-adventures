- `language/past-tense-ed-sound-effect.adj` (new) — a sibling to the already-shipped
  `past-tense-ed-sound.adj` (`past_tense_ed_sound(word, sound)`, walked/lived/wanted →
  t_sound/d_sound/id_sound). That table's own header already reproduces, verbatim, the 7ESL rule for
  the /id/ sound in full — "Final -ed is pronounced /id/ after 'T', and 'D' sounds. The sound /id/ adds
  a whole syllable to a word." — but the word/sound schema had room only for the sound itself, not the
  second sentence's pronunciation-effect claim. New `past_tense_ed_sound_effect(sound, effect)` table
  decodes that second sentence as its own row: id_sound → adds_a_whole_syllable. Honest abstention on
  t_sound, a real already-tabled -ed sound whose own rule states no comparable effect. New e2e test file
  `facts_pasttenseedsoundeffect_e2e.rs` (3 tests: forward recall with citation, backward recall, honest
  abstention). No manifest objective, matching `past-tense-ed-sound.adj`'s own precedent. Fourth slice
  from the language/ domain cleanup sweep.
