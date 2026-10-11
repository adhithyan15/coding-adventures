### Fixed — counts of one read naturally, and point cues say "Point to" once

- **Narration chapter header.** `renderChapterNarrationText` worded every
  branch for "many". A one-lesson chapter said "1 lesson. All 1 can be done
  entirely by ear." (241 chapters) and, when that lesson needs eyes or hands,
  "1 lesson. The first lesson already needs your eyes or your hands, so save
  this one …" (60 chapters), though there is no "first" of one. They now say
  "1 lesson. It can be done entirely by ear." and "1 lesson. It needs your
  eyes or your hands, so save it for when you have stopped." Two-lesson
  chapters that are drivable throughout say "Both can be done entirely by
  ear." instead of "All 2 can …" (70 chapters). The header comment tabulates
  every branch by count. Regenerated: 383 narration `.txt` scripts (371 header
  lines, 12 for the point cues below), 12 narration `.json` chapters and the
  narration hash shards.
- **Book chapter header.** The "Hands-free start:" line in each book's
  `chapter-modalities.tex` came from a template that printed "first 1 of 2
  lessons", "all 1 lesson" and "none of the 1 lesson". A new exported
  `handsFreeStart(drivablePrefix, lessonCount)` in `book.ts` words the small
  counts for what they are: "its only lesson" (249 chapters), "none (1
  lesson)" (52), "both lessons" (75), "neither of the 2 lessons" (23), "the
  first of N lessons" (193), and "the first K of N lessons" gains its article
  (214); "all N lessons" and "none of the N lessons" stand for N > 2. That is
  806 chapter headers across all 23 books. `chapter-modalities.tex` is a
  compile-only projection, so no committed `.tex` changes for this.
- **Point cues.** The book prints `[YOU POINT: …]` as "*Point to:* …" and the
  narration as "point: …", so a cue written `[YOU POINT: to the right edge]`
  printed "Point to: to the right edge", and `[YOU POINT: at …]` printed
  "Point to: at …". The fix is in the 15 source lessons (Hindi 1, Persian 6,
  Tamil 1, Urdu 7), not the renderer: a renderer that stripped a leading "to"
  would also strip one the author meant, and the narration's "point: to …"
  would need a second rule of its own. Tamil's "[YOU POINT: in each one, the
  letter carrying the புள்ளி]" is reordered to name the letter first.
  A new corpus test in `cue-action-classification.test.ts` fails on any POINT
  cue whose content opens with to, at, toward(s), in or on. Regenerated for
  the 15 lessons: 12 book chapter `.tex` files, their `core/generated-book-hashes`
  and `core/lesson-modality` source hashes (modality is unchanged).
- **Tests.** Narration: one-, two- and three-lesson headers, drivable and not.
  Book: every row of the `handsFreeStart` table, and a one-lesson chapter that
  prints neither "all 1" nor "first 1"; `book-cli.test.ts`'s staged
  one-lesson book now expects "its only lesson". Corpus: the POINT-cue
  preposition check.
