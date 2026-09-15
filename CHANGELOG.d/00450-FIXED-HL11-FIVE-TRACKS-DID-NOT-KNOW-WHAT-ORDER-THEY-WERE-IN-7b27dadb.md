### Fixed — HL11: five tracks did not know what order they were in
- Hindi, Telugu, Kannada, Malayalam and Sanskrit each carried ~30 lessons with
  **no `sequence` at all** — every word lesson of their first five chapters. So
  the corpus believed their words came *after* their script lessons, which is
  the opposite of what their books print, and the only reason the books read
  correctly is that those chapters are hand-typed LaTeX.
- HL09 §4 has required `sequence` since it was written, and everything measured
  since — closure, continuity, the ramp windows, the drivable prefix, the app's
  reading order and the narration export — is a claim about order. For these
  five tracks those claims were being made against an order nobody had declared.
- The order is **recovered, not invented**: each book's own
  `\label{lesson:...}` sequence is the only place it was ever written down.
  Corpus lessons without a declared order: **477 → 322**.
- The knock-on numbers are the point. Forward prerequisites **225 → 143** and
  forward reviews **267 → 168**: most were artifacts of an undeclared order, not
  real gaps in the ramp. Tracks with unordered lessons **17 → 12**.
- Hindi also loses the title of steepest lesson in the corpus. `HI-W01` still
  shows twelve glyphs at once, but Hindi's *words* now come before it, so it is
  no longer the first place those glyphs appear. Marathi inherits the record
  with the same twelve — and Marathi still has no declared order, which is why.
- One matching bug caught before it landed: `HI-W04-ra-sa-mera-naam` ends with
  "naam" and was matched to the lesson `naam`, which jumped it ahead of its own
  prerequisite. Labels are now compared against a lesson id's whole tail.
- All five books rebuilt: exit 0, **zero** warnings — 183, 169, 167, 192 and 100
  pages.

