### Changed — French's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.french` in `tests/root-ledger.test.ts` is lowered from
1048 word lessons without an `etymology_hook` and 1049 without visible
etymology to **0** and **0**. The 1,045 generated word lessons
(FR-C49-bras through FR-C257-anniversaire) now say where their word comes
from, and seven earlier hand-written lessons gained the half they lacked, so
every French word lesson does.

- **Sources.** EtymDB 2.1 (Fourrier & Sagot 2020, LREC; CC BY-SA 4.0),
  cross-checked row by row, and Lewis & Short (Perseus `lexica`, CC BY-SA 4.0)
  for textbook Latin forms EtymDB lacks. Both were read locally; no dataset
  file is vendored. Each line is our own sentence and names only the source
  that supports it, or none.
- **Shape.** The same `## The word, taken apart` block as Italian
  (`## The phrase, taken apart` for multi-word headwords), typed `etymology` by
  the parser, after the introducing block, and a matching `etymology_hook` of
  120 characters or fewer.
- **Answers.** 388 inherited, 194 learned or semi-learned borrowings, 107 borrowed from other
  languages, 259 built from parts, 71 compounds and phrases, 8 named after a
  person or a thing, and 18 that say the origin is uncertain. 57 EtymDB rows
  were rejected (homographs, wrong parents, junk derivations) and 30 loans or
  Latinisms labelled inherited are given as borrowed, learned or semi-learned. The French
  changelog lists examples.
- **Ledger side effects.** 671 lessons gained or extended `roots:`, reusing the
  corpus's existing spelling of an etymon wherever one exists; 40 slugs are new
  `<lemma>-latin`. `check:root-slug-splits` reports no new split. Four
  hand-written lessons raise `duration.max_seconds` (FR-C07-dimanche 245,
  FR-C08-il-est 215, FR-C14-avoir-etre-langues 250, FR-C15-areal 275); the
  generated lessons compute at 73–117 s against a declared 200.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
