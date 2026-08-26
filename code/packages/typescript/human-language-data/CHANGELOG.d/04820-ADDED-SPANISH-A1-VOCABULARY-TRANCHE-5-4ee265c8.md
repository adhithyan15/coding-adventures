### Added - Spanish A1 vocabulary tranche 5

- Add thirty-five A1 vocabulary lessons across chapters 367-373, sequences
  7270-7610, one headword and one new atom per lesson. Spanish moves **514 ->
  549** distinct headwords at or below A1, against the HL09 3.1 floor of 600.
- Author entirely against the HL21 shards: seven new `chapters.d/`, seven new
  `curriculum.d/path/`, seven new `curriculum.d/extensions/`, seven targets in
  `core/book-generation.d/spanish.json`, and one appended `segments` line in
  each of the three A1 spine nodes used. The three monoliths are regenerated
  with `--unshard`, never hand-edited.
- Screen every candidate three ways: against the headword list, against the
  **atom** ledger across all lesson types, and against the **root** ledger. The
  string matcher caught six of the drops and missed nine; the misses are listed
  in the Spanish changelog.
- Verify every etymology against current scholarship before writing the prose.
  Two independent passes killed or rewrote eleven planned hooks, including
  `la alfombra`, whose long-printed "the red one" story is superseded by the
  Academy's own `alhanbal`, and `el rio`, which now teaches that English
  *river* is not the same word.
- Hold every content pin exactly: `ruleStatements` stays at 30 of 30,
  `paradigmTables` at 95, `lessonsWithFindings` at 121, `fullParadigmGrids` at
  22, banned words unchanged, and Spanish cross-chapter prose references at
  zero. All seven chapters narrate as five-of-five drivable.

