## 0.15.0 — jump to any concept (HL03 polish)

- **A "jump to concept" picker in the Learn nav.** Walking 186 concepts one
  Next-click at a time is a long way to get anywhere; the nav row is now
  `← Previous | [jump ▾] | Next →`, where the picker is a native `<select>` of
  the whole book-ordered spine (`1. courtesy · thanks`, `2. farewell`, …) with
  free keyboard type-ahead. Selecting one jumps the cursor straight there.
- All three controls now funnel through one `jumpToConcept(index)` — it clamps,
  resets the review draw (the covered set changed), persists the cursor (so the
  jump is where you resume next visit, via the existing `cursorstore`), and
  re-renders. No new persistence surface; it reuses the tested cursor save/clamp.
- **A slice was abandoned, honestly:** the planned "romanization under the review
  options" turned out un-grounded — only ~54 of ~700 lessons populate a
  `romanization` field, and the Indic vocabulary (where script-shape guessing is
  the real gap) carries its romanization inside the *gloss* text instead. Showing
  it would render inconsistent subtext for <8% of options, so it was dropped
  rather than faked; this jump-picker was built instead. 225 tests still pass;
  verified in a real browser.

