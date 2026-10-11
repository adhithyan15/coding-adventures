### Fixed — twenty-three more wide tables are read aloud, and the full-stop lesson says its spacing in words

The narrator's table refusals fall from **25 to 2**. The previous pass folded
the wide tables that opened chapters; this one read the other 25, and 23 of
them are now three LABELLED columns that keep every value they had. A column
was merged only where it was derivable from, or an annotation on, its
neighbour. Examples: a numeral beside its word ("**sei** (6) ← *sex*"), a Latin
source beside its descendant, a gloss beside its infinitive ("**tener** (to
have)"), a romanization in the Read cell beside its script, a stem beside its
verb ("hacer: har-"), and sister-language cognates sharing one cell ("*eraḍu* /
*reṇḍu* / *raṇṭu*"). `maxLinearisableTableColumns` stays 3, and no classifier
rule changed.

The two that are left are refused on purpose. Spanish chapter 46's
eighteen-form -ar/-er/-ir grid and chapter 124's six-row future/conditional
frame are paradigm grids that the eye needs, and `info-dump` flags both as full
grids. Folding either would hide the grid from that gate.

- **Lessons now drivable at their core:** 17 more (27,743 → 27,760). The
  chapters' summed drivable prefix rises from 26,587 to 26,607. No lesson got
  less drivable. Three of those lessons also had a page reference that was
  never about the page, and it is now said for the ear: FR-C01-nuit ("once you
  know the rules"), PT-C08-meio-dia-meia-noite ("And now take **noite**") and
  ES-C10-futuro-simple ("on the same ending as *hablaré*").
- **ES-C424-punto**, the full-stop lesson: by ear, its spacing table read
  "correct: Mi familia. wrong: Mi familia." The difference was whitespace, so
  the line meant nothing aloud. Each cell now says the difference in words
  ("no space before the point, one space after it", "a point after the title"),
  and the printed example stays beside it. The warm-up no longer points at "the
  last sentence you read". It names the small dot that ends a written sentence.
- **Tests:** `tests/narration.test.ts` pins 2 refusals. A test in
  `tests/drivable-instructions.test.ts` plants a "Type the word for …" question
  in every 250th core-drivable lesson. With 17 more such lessons, the sample
  moved onto PA-C34-ji-aaian-nu, whose planting site (its Wrap-up Recall) is a
  section the lesson's notice sets aside. The gate rightly does not scan that
  section. The test now skips set-aside sites, as its sibling deferral test
  already did, and requires that more than 10 lessons still demonstrate the
  gate. The `BY_CONTENT_CEILING` (7) is unchanged. None of these lessons opens
  a chapter.

A fourth reference that was never about the page is reworded too: ES-C07-repaso-comer's
"Read the middle two columns down" became "Set the two endings in each row side by side",
and its folded headers read *-ar* / *-er*.

PA-R172-form-order-again (#17162) merged with a modality record derived before the
driver-safety rules and announced itself as drivable; regenerated, it is `pen`, as its own
authored reason says ("inspect and copy printed Gurmukhi form labels"). The by-content
ceiling in `tests/drivable-prefix.test.ts` rises 7 -> 8 for it, with the reason recorded —
a correction, not new debt. This also brings main's `check:modality` back to green.

