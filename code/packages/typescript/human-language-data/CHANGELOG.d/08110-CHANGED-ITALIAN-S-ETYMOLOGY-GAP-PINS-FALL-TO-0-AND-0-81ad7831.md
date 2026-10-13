### Changed — Italian's etymology gap pins fall to 0 and 0

`ETYMOLOGY_GAP_PINS.italian` in `tests/root-ledger.test.ts` is lowered from
745 word lessons without an `etymology_hook` and 745 without visible etymology
to **0** and **0**. The remaining 745 gap lessons in sequence order
(IT-C111-infermiera through IT-C259-storia) now say where their word comes
from, so every Italian word lesson does. Italian is the first track whose pins
are both zero.

- **Sources.** As in the first batch: EtymDB 2.1 (Fourrier & Sagot 2020,
  LREC; CC BY-SA 4.0), and Lewis & Short (Perseus `lexica`, CC BY-SA 4.0) for
  certain textbook Latin forms EtymDB lacks. Both were read locally; no dataset
  file is vendored. Each line is our own sentence and names only the source
  that supports it, or none.
- **Shape.** The same `## The word, taken apart` block (`## The phrase, taken
  apart` for multi-word headwords), typed `etymology` by the parser, after the
  introducing block, and a matching `etymology_hook` of 120 characters or fewer.
- **Answers.** 319 inherited, 126 learned borrowings, 63 borrowed from other
  languages, 221 built from parts, 2 named after a place or a god, and 14 that
  say the origin is uncertain. 61 words with no EtymDB row are traced through
  the sister words EtymDB records for the same etymon; 51 lines cite Lewis &
  Short. 41 EtymDB rows were rejected (homographs, wrong parents, junk suffix
  rows), 19 loans labelled inherited are given as borrowed, and 42 Latinisms
  labelled inherited are given as learned. The Italian changelog lists
  examples.
- **Ledger side effects.** 606 lessons gained a `roots:` slug, reusing the
  corpus's existing spelling of an etymon wherever one exists (`gratia`,
  `heri`, `tonare-latin`); the rest are new `<lemma>-latin` slugs.
  `check:root-slug-splits` reports no new split. No `duration.max_seconds`
  changed: the lessons compute at 74–109 s against a declared 200.
- **No code changed.** The measure in `src/etymology-coverage.ts` already counts
  a "taken apart" block as visible etymology.
