### Fixed — a Writing block that is a dictation or a composition no longer prints a strip

- `figure-targets.ts`: the writing-stage rule (a block shows the learner a
  model only at `observe-trace`, `guided-copy` or `delayed-copy`) now
  applies to Writing and Script blocks too, not only to the fallback.
  `stripBlockIndex` is the first Writing block, else the first Script
  block, else the first modelled practice block, skipping any block that
  declares a no-model stage (dictation, either composition, timed
  production). The book prints a strip at the top of its block, above the
  task, so a dictation printed its answer above its cue: "How hola is
  written" above "Hear: OH-la. Write the Spanish greeting from that sound
  alone". None of these lessons has an answer-key block to move it to.
- 33 lessons lose their strip (829 -> 796, counted on top of the twelve
  Kannada and Malayalam strips, none of which declares a stage): arabic 1,
  french 2, german 2, gujarati 14, italian 1, kannada 1, latin 1,
  malayalam 1, persian 1, portuguese 1, sanskrit 4, spanish 3, telugu 1.
  Four Marathi letter lessons (MR-W03-ba, -lla, -va, -ya) keep theirs,
  moved up from "Writing — heard cue" to the Script block that shows the
  letter the dictation covers.
  Their 33 SVGs are deleted, the figure-hash owners regenerated (latin's
  owner goes, with its only strip), and 29 book chapters regenerated.
- `letter-anchoring.ts` no longer asks the strip placement whether a
  lesson writes a letter: a single-letter dictation (SA-S02-dictation)
  still writes it. The anchoring report is unchanged, lesson for lesson.
- Pins: filmstrip-target-counts arabic 22 -> 21, french 5 -> 3, german
  5 -> 3, gujarati 73 -> 59, italian 3 -> 2, kannada 57 -> 56, latin 1 -> 0,
  malayalam 60 -> 59, persian 27 -> 26, portuguese 3 -> 2, sanskrit
  55 -> 51, spanish 9 -> 6, telugu 46 -> 45; the real-corpus case's Gujarati
  sign table, Devanagari sign and word tables, word list and Latin table.
  The per-track count check now holds a pin of 0 for a track that draws
  nothing.
- New case `no-strip-above-a-dictation`: every no-model stage on a Writing
  block, the move to Script, a Script block that is itself a dictation,
  anchoring, the 33 real lessons and the four moves, and a converse guard: a
  lesson with no strip never says "follow the numbered strip" or "the strip
  shows" (with a control that strip lessons do).
