## Handwriting: the gap named, not filled — HL-C41 (2026-08-06)

- **No Telugu handwriting was authored, and that is the deliberate outcome.** The
  track still teaches zero letter formation: `data/scripts/telugu.json` has 0 of 455
  entries with a `strokeOrder`, and there are no `type: writing` lessons. Tamil has
  11/11 and Devanagari 28/28, so this remains a real gap in three of twenty tracks
  (Kannada 0/455 and Malayalam 0/468 are identical).
- **The blocker is provenance.** `strokes.ts` admits a letter only with a citation
  and a URL for its stroke ORDER — the pen path's shape is checked against the
  vendored font, but no font records the order, so it must trace to a published
  source. Not one such source could be opened for a single Telugu letter. Zero
  letters were authored rather than any uncited ones. The full search record is in
  the [`BACKLOG.d/`](../BACKLOG.d/) history, *Findings from HL-C41*.
- **"Telugu is written without lifting the hand" is a simplification.** Telugu's
  roundness does make many letters loop-continuous, which is genuinely teachable, but
  the published statement about Telugu stroke direction is that the order is *not*
  uniform — clockwise for some letters, counter-clockwise for others — and the
  `talakattu` tick crowning most consonants is described as its own mark. So
  `penLifts` stays **absent** for every Telugu entry, which means NOT VERIFIED.
- **`telugu.json` now states the rule it is governed by.** Its `notes` record that
  only base consonants and vowel signs are ever authored (a syllable's figure is
  assembled from its parts), that `penLifts` absent means NOT VERIFIED and never
  none, and that it must never be inferred from `strokeOrder.length`. The rule is
  expanded in [`data/scripts/README.md`](../data/scripts/README.md). Authoring 455
  syllables was never the work; authoring ~36 base shapes is.
- No lesson content changed. Chapter counts, book output, and the track's 78%
  drivable figure are untouched.

