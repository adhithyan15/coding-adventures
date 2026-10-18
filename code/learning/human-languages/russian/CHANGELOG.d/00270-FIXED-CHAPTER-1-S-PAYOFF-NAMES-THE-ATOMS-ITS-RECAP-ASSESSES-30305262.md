## Fixed — chapter 1's payoff names the atoms its recap assesses

Chapter 1 was migrated to schema v2 (#12078), but `chapters.d/0001.json` kept
the schema-v1 placeholder `payoff.assesses: []` and a note waiting "until
migration", so the representativeness gate scored the recap 0/18 (#12088).

- **RU-C01-practice** now also declares `RU-PRAGMATICS-GREETING-REGISTER` in
  `requires`, `practises` and the assessments of "The exchange" and
  "Wrap-up Recall" — both already make the reader choose between
  здравствуйте and привет. No prose changed.
- The payoff names all eight atoms of the chapter's spoken layer: the six
  words, the register, and the courtesy dialogue. That is 8/18 (0.44), still
  below the 0.5 floor, and the new note says why: the other ten atoms are
  Cyrillic letters that RU-W01 to RU-W05 (sequences 40-64) introduce after the
  recap (sequence 35), so the recap cannot assess them. Closing the gap needs
  a terminal checkpoint after the writing runway.
- Book chapter 1, narration ch01, the lesson's modality owner and both hash
  manifests regenerated (source hashes only).

