### Added — lesson prose may not contradict a letter's pen-lift record

- **New test `tests/pen-lift-prose.test.ts`.** It reads every single-letter
  writing lesson (`writingLetterOf`) whose letter or sign has a verified
  `penLifts` in its script inventory (753 lessons: 657 letters, 96 signs)
  and enforces two rules:
  1. **Whole-letter counts agree with the record.** Six forms the corpus
     uses are read exactly: the step-list label (`**Pen lifts: N.**`), a
     count opening a sentence ("Five pen lifts."), strokes and lifts
     together ("Six strokes, five lifts.", which must also agree with each
     other), "counting the N lifts", "N pen-down runs" (N − 1 lifts), and
     a self-check question ("How many pen lifts? (**Two.**)"). 225 such
     counts are checked.
  2. **Zero means zero.** Where the record draws the letter in one unbroken
     stroke, the block that prints its filmstrip (`stripBlockIndex`) may
     not instruct a lift: no sentence opening "Lift", no numbered step
     "lift, then …", no ", then lift" that goes on to more ink. A lift that
     ends the letter (", then lift.") is allowed. 252 lessons are checked.
  Floors on the lesson, count and zero-lift totals keep it from passing
  vacuously, and small inline cases pin what each reader accepts and
  refuses ("one stroke, no lift" inside a Chinese character is not read as
  the character's count). Run against the corpus before this change, the
  two rules found 13 lessons; the audit that prompted it found 33 across
  six tracks, fixed in the Telugu, Marathi, Marwadi, Urdu, Japanese and
  Russian lessons.
- **Regenerated** from the corrected lessons: book chapters and their hash
  owners, narration and its hash owners, and lesson-modality records for
  the six tracks. Figures do not change.
