### Added — HL11: Tamil's Script Drizzle, One Letter at a Time
- Nine new Tamil lessons, each teaching **exactly one letter**: வ, ண, ன, ந, ற,
  க, ம, the puḷḷi, and the i-sign. Every one carries the letter's components,
  its full pen path, its pen-lift count and its citation — all read from
  `data/scripts/tamil.json`, none of it asserted by hand.
- This is the thing HL11 was written for and the corpus did not have. Tamil had
  a writing strand of 24 lessons, but every one of them is word-shaped — *write
  வணக்கம்*, *read peyar* — putting four to sixteen letters in front of the
  reader at once, with the first at sequence 270. A strand, and no drizzle.
- Each segment sits immediately **before** the word-writing lesson that uses its
  letter, so the reader meets a letter alone and then meets it inside a word.
  Tamil closure violations fall 50 → 42.
- **The drizzle costs the driving edition nothing.** `drivableLessons`,
  `drivablePercent`, `drivablePrefixTotal` and `fullyDrivableChapters` are all
  unchanged: no existing lesson became undrivable and no chapter lost a lesson
  from the prefix a commuter can do. That is HL11's own falsification test.
- It only passes because of where they sit. An earlier revision placed them in
  chapters 1–3, where the payoff is soonest, and two things broke: those
  chapters are **handwritten and protected from generation**, so the segments
  existed in the corpus and never reached the page; and one landed as chapter
  3's first lesson, leaving that chapter impossible to begin in the car —
  caught by `unstartableChapters` moving 173 → 174.
- First use of HL-C41's block-level modality by real content:
  `lessonsWithWritingSegments` moves 0 → 9 after existing only in fixtures.
- The book was rebuilt: 259 pages, exit 0, **zero** overfull, underfull or
  missing-character warnings. Fixed one page defect found by reading the PDF —
  the renderer has no Markdown ordered-list conversion, so numbered stroke steps
  collapsed into a run-on paragraph, which for a pen path is not cosmetic.

