### Changed — Latin's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.latin` in `tests/root-ledger.test.ts` is lowered from
1095 word lessons without an `etymology_hook` and 1105 without visible
etymology to **0** and **0**. The 1,090 generated word lessons
(LA-C63-bracchium through LA-C280-vallum) now say where their word comes
from. The five Chapter 1 greetings gained the hook they lacked, and fifteen
hand-written lessons gained a short taken-apart block, so every Latin word
lesson does.

- **Sources.** EtymDB 2.1 (Fourrier & Sagot 2020, LREC; CC BY-SA 4.0),
  checked row by row, and Lewis & Short (Perseus `lexica`, CC BY-SA 4.0),
  without its outdated comparisons. Both were read locally, and no dataset
  file is vendored. Each line is our own sentence. A source tag follows only
  a fact that the source's entry holds, and a script checked every tag.
- **Shape.** The same `## The word, taken apart` block as French and Italian
  (`## The phrase, taken apart` for a multi-word headword), typed `etymology`
  by the parser, with a matching `etymology_hook` of 120 characters or fewer.
- **Answers.** 287 inherited from Proto-Indo-European or Proto-Italic,
  every reconstruction starred and hedged. 67 borrowed from Greek, 9 from
  another language (Celtic, Sabellic, Punic, Germanic, Etruscan), 544 built
  inside Latin, and 183 that say the origin is uncertain. 54 source rows were
  rejected or relabelled (homographs, Greek loans marked as inherited, junk
  suffix rows). The Latin changelog lists examples.
- **Ledger side effects.** All 1,090 lessons gained `roots:`. 578 of the
  1,344 slugs already existed, and 766 are new `<lemma>-latin` (plus
  `latin-nam-for`, kept apart from Hindi's bare `nam`).
  `check:root-slug-splits` reports no new split.
- **Durations.** Eleven hand-written lessons raise `duration.max_seconds` to
  their computed length, all under 300 s: LA-C05-dies-lunae-veneris 278,
  LA-C06-niger-albus 291, LA-C12-medius-dies-nox 270, LA-C16-undecim-viginti
  252, LA-C19-quid-agis 205, LA-C19-ut-vales 221, LA-C20-quid-tibi-nomen 252,
  LA-C22-tu-vos 279, LA-C30-bonam-noctem 279, LA-C34-bonum-vesperum 271 and
  LA-C36-salve-post-meridiem 255. The generated lessons compute at 76–127 s
  against a declared 200.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
