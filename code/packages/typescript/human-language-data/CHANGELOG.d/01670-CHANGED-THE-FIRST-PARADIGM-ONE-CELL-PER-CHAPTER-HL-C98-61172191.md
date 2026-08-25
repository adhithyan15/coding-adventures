### Changed - the first paradigm, one cell per chapter (HL-C98)

- Split `ES-C06-ar-presente` — which taught `hablo`, `hablas` and `habla` in a
  single lesson, behind a three-row table on first exposure, with pro-drop
  alongside — into five chapters: **15** *hablo* and pro-drop, **16** *hablas*,
  **17** *habla*, **18** a **review** chapter, **19** a **synthesis** chapter.
  `maxNewGrammarCellsPerLesson` is 1; this lesson taught three.
- Keep `ES-C06-ar-presente` as the review chapter so the 14 lessons that require
  `ES-GRAMMAR-AR-PRESENT-SINGULAR` keep resolving; the atom is now *earned* at
  the recap rather than asserted at the introduction. Its table is unchanged and
  finally legitimate.
- Add the corpus's **first `teaches_cells:` declarations** — coverage moves
  **0 → 3 of 231** against `spanish/grammar-cells.json`, whose
  `1SG → 2SG → 3SG` prerequisite chain already prescribed exactly this order.
- Add the book's first chapters that introduce **zero** new atoms. Chapter 19
  makes the register choice itself the communicative act: one conversation held
  twice, warmly then respectfully, where the only thing that changes is one
  letter on one verb.
- Renumber Spanish 50 → **54 chapters** (old 16–50 → 20–54). Lesson ids are
  stable slugs and deliberately do not renumber. Forward references **424 → 423**;
  fully drivable chapters **332 → 336**; chapter 18 is `sight`, because a
  paradigm table cannot be read aloud.

