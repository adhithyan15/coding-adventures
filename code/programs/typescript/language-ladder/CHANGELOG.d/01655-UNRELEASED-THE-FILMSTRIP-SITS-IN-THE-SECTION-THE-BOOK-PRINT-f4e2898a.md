## Unreleased — the filmstrip sits in the section the book prints it in

- `filmstripSectionIndex(body, sections)` no longer restates the book's
  placement rule. It runs the book's own `filmstripBlockIndex`
  (human-language-data's new `strip-placement.ts`) on the book's own parse of
  the body (`parseBodyBlocks`), and maps the chosen block to a section.
- The old copy took the first Writing OR Script section, whichever came
  first; the book takes the first Writing block and falls back to Script only
  when there is none. 435 of the 795 writing lessons with a strip showed it
  higher than in print, under "Script you'll notice: د" rather
  than "Writing: د" (malayalam 57, kannada 54, hindi 47, sanskrit 43, telugu
  43, gujarati 41, marathi 39, tamil 37, urdu 18, persian 16, arabic 13,
  marwadi 10, punjabi 10, russian 5, bengali 1, japanese 1). They now match
  the book. No strip appears or disappears.
- `LessonSection` gains `blockIndex`, the index of the book block (the `## `
  heading, counted the way `parseBodyBlocks` counts it) a section renders;
  the preamble has none. The app's own `MODELLED_WRITING_STAGES` copy is
  gone.
- A new corpus test checks, for every writing lesson with a committed
  filmstrip, that the app's section is the block the book's `parseLesson`
  and `filmstripBlockIndex` choose, by index and by heading.
- Known gap, unchanged: FA-C03-chist, a word lesson with a declared strip of
  چ, still shows no strip in the app, which captions strips from the headword.
