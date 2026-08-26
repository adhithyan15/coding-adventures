### Added - Spanish A1 vocabulary tranche 4

- Add thirty-five A1 vocabulary lessons across chapters 360-366, sequences
  6920-7260, one headword and one new atom per lesson. Spanish moves **479 ->
  514** distinct headwords at or below A1, against the HL09 3.1 floor of 600.
- Author entirely against the HL21 shards: seven new `chapters.d/`, seven new
  `curriculum.d/path/`, seven new `curriculum.d/extensions/`, seven targets in
  `core/book-generation.d/spanish.json`, and one appended `segments` line in
  each of the three A1 spine nodes used. The three monoliths are regenerated
  with `--unshard`, never hand-edited.
- Hold every content pin exactly: `ruleStatements` stays at 30 of 30,
  `paradigmTables` at 95, `lessonsWithFindings` at 121, banned words unchanged,
  and Spanish cross-chapter prose references at zero.
