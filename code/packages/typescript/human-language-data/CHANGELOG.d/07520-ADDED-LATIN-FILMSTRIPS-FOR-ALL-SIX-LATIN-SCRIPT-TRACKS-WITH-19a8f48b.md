### Added Latin filmstrips for all six Latin-script tracks, with the one-storey a

- The Latin inventory's font is now `_fonts/LatinPrint-Subset.ttf`, a
  renamed subset of SIL Global's literacy typeface Andika 7.000 whose a is the
  one-storey a the Grundschrift-App and UJIpenchars2's writers teach. The
  books' body text is unchanged (Latin Modern Roman); every Latin source note
  says the strip shows the handwriting model's letter, not the book's type.
- `data/scripts/latin.json`: 31 cited rows (was 18): a d p q t y H, and new
  precomposed rows á é í ó ú ü (63 rows, was 57). Every Latin variation now
  names the new font.
- `DERIVED_FILMSTRIP_SCRIPTS` gains french, italian, portuguese and latin
  (all `"latin"`). 22 lessons gain a strip: FR-W01-salut-observe,
  -guided-copy, -delayed-copy, -dictation, FR-W04-quatre-lignes (parce que);
  GE-W01-hallo-guided-copy, -delayed-copy, -dictation;
  IT-W01-ciao-guided-copy, -delayed-copy, -dictation; LA-W04-quattuor-versus
  (quia); PT-W01-ola-guided-copy, -delayed-copy, -dictation;
  ES-W00-hola-observe, -guided-copy, -delayed-copy, -dictation, ES-W01-acento
  (á é í ó ú), ES-W01-frase-propia (buenos días, exactly ten pieces) and
  ES-W02-cuatro-lineas-ayer. The four earlier Latin strips are redrawn on the
  new outline. None of the new lessons disclaims its stroke order.
- Still undrawn: FR-W01-accents, FR-W02-cedille, FR-W03-trema, FR-C10-oe,
  GE-W02-umlauts and the salvē lessons (uncited marks), ES-C03-como-acento and
  ES-W03-question-span (punctuation inside a word), ES-W01-tilde-diacritica
  (a slash), ES-W02-enye-formas and GE-W03-capitalization (past
  `MAX_SEQUENCE_PIECES`).
- Tests: the real-corpus case pins all 26 Latin strips and the new words;
  the Latin figure-targets case switches on all six tracks; every track in
  the corpus is now on, so the switched-off example is a track the allowlist
  does not name (greek); `filmstrip-target-counts` spanish 2 -> 9, german
  2 -> 5, new french 5, italian 3, portuguese 3, latin 1; `latin.evidence.ts`
  pins the new font, 63 rows and 31 cited glyphs.
