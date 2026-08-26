### Added - Spanish A1 vocabulary tranche 6

- Add thirty-five A1 vocabulary lessons across chapters 374-380, sequences
  7620-7960, one headword and one new atom per lesson. Spanish moves **549 ->
  584** distinct headwords at or below A1, against the HL09 3.1 floor of 600.
  Sixteen to go.
- Author entirely against the HL21 shards: seven new `chapters.d/`, seven new
  `curriculum.d/path/`, seven new `curriculum.d/extensions/`, seven targets in
  `core/book-generation.d/spanish.json`, and one appended `segments` line in
  each of the three A1 spine nodes used. The three monoliths are regenerated
  with `--unshard`, never hand-edited, and shard integrity is asserted on
  parsed counts rather than on bytes: 380 chapter shards against 380 monolith
  sections, 379 path, 374 extensions, 33 spine nodes.
- Screen every candidate three ways: against the headword list, against the
  **atom** ledger across all lesson types, and against the **root** ledger.
  The mechanical screen earned its place this time, catching two drops a
  careful hand pass had already cleared -- `el codo` against the very common
  `cómo`, and `la cima` against both `la cama` and `la cita`.
- Correct the near-duplicate rule rather than widening it. A same-length pair
  differing in one position is only a drop when the differing position is
  **not** the first: `el hombre`/`el hombro` and `la ropa`/`la roca` share
  their opening and are genuinely confusable, while `beso`/`peso` and
  `codo`/`todo` share nothing a learner keys on. Firing on a first-letter
  difference is the containment mistake in a new costume.
- Verify every etymology against current scholarship before writing the prose.
  Three independent passes killed or rewrote **eleven** planned hooks, four of
  them outright false. The largest: `la vela` was going to teach that the
  candle and the sail are one word about stretched cloth, and they are two
  words with opposite ancestries -- the candle is `velar` from *vigilare*, to
  keep watch, and only the sail is *velum*.
- Delete three etymologies that were being used as their own evidence:
  `el pulgar`'s "the strong one", which reads strength out of a proposed
  ancestor and offers the thumb's obvious strength back as proof of it;
  `el pimiento`'s "named for its colour", where the Academy's own gloss shows
  the route running through drug and condiment rather than paint; and
  `necesario`'s "what cannot be got round", which is the disputed analysis
  restated rather than support for it. Each lesson now says why the tempting
  version fails.
- Hold every content pin exactly. `ruleStatements` stays at 30 of 30,
  `paradigmTables` at 95, `lessonsWithFindings` at 121, `fullParadigmGrids` at
  22, banned words unchanged, and Spanish cross-chapter prose references at
  zero. **No pin was raised.** The derived bundle gate reports 284 lesson
  batches over 283 chapter bands, `BAND_SPLIT_SLACK` untouched at 1 and the
  256 kB backstop untouched.
- All seven chapters narrate as five-of-five drivable, and all 35 lessons
  resolve to `voice` in the modality manifest.
