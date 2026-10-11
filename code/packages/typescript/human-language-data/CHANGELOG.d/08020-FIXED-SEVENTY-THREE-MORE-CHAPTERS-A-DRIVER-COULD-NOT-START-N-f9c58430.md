### Fixed — seventy-three more chapters a driver could not start now open on a lesson they can do, and five open after a reorder

Chapters with a drivable prefix of 0 fall from **263 to 185**. The by-content
count falls from **80 to 7**, and `tests/drivable-prefix.test.ts` lowers its
ceiling to 7. The by-type count falls from **183 to 178** through five
reorders. Core-drivable lessons rise from 27,669 to **27,743**. No chapter's
prefix fell. No classifier rule changed: the sight cues, the eyes-or-hands
detectors, the prompt and heading gates and the linearisable table width
(`maxLinearisableTableColumns: 3`) are as they were. All 80 by-content openers
were read, and 73 were fixed:

- **Wide tables folded to three labelled columns (19 lessons).** Each table
  had a column that was derivable from another, or that folded into one. The
  first case is a bare numeral beside its word (Italian, Portuguese and Latin
  teens and units, Telugu digits, Sanskrit stems). The second is "planet-god"
  beside "day of Mars" (Spanish, Italian, Latin, Hindi, Malayalam weekdays),
  a Latin source beside its Romance descendant, or an ending beside the form
  it ends ("-e: je parle"). Every value the table carried is still printed.
  The narrator now reads these tables instead of saying "Come back and look at
  it", and the refusal pin in `tests/narration.test.ts` falls from 44 to 25.
  The French -er grid keeps its person column, so `info-dump` still flags
  FR-C05-parler as a full paradigm grid. An earlier draft dropped that column,
  and the info-dump test caught it.
- **Said for the ear (43 lessons).** Some phrases referred to the page
  but were never about it: "Before the table:", "all three rows of the
  table", "the whole first column", "opening a column this book has left
  empty", "can you see the seam?", "left you at the table" (a breakfast
  table), and "You can see the adjective half". Some points are as audible as
  they are visible: "Put the word beside the number it comes from",
  "Notice the front of it", and "Say the eight commands in a row: *di, haz,
  ve…*". A pointing gesture became a description. Glosses that contain
  *see* or *look at* are quoted ("I see the house", "to look at, to watch").
- **Deferred as an authored cue (11 lessons).** Eight are written-accent,
  silent-h and letter-shape observations (ES-C57/C63/C65, ES-C424, IT-C14,
  BN-C18, SA-C61, PA-R160). Each is now `[YOU LOOK: …]` and is followed by
  the fact said aloud. Three more defer a hand or reading step: ES-C422's writing warm-up
  becomes `[YOU WRITE: …]`, and GU-C38's R4 written recall becomes
  `[YOU WRITE: …]`. TA-C71's "say *vaṇḍi*, then read **மேசை**" is split
  into a spoken recall and a `[YOU READ: …]`.

The seven left at 0 by content were each read and are honest:

- PA-R168 to PA-R171: every step of these date-card reviews handles a printed
  card.
- ES-C07-repaso-tres-familias: an eighteen-form -ar/-er/-ir grid whose drills
  run "down a column, then across a row".
- GU-C25-savaar and UR-C16-practice: reading practice in all but `type`.

Of the 13 reorder candidates, five chapters were reordered. Every gate stayed
green: script closure, glyph budget, continuity, reinforcement, prerequisites
and the track pins.

- Telugu ch3: TE-C03-elaa moved ahead of the ౩ writing lesson.
- Telugu ch5: TE-C05-maatlaadu moved ahead of the త ladder.
- Telugu ch6: TE-C06-dative-ku also lost its "can you see the seam?" cue and
  moved ahead of the ౦ lesson.
- Urdu ch7: UR-C07-sochna moved ahead of آ.
- Urdu ch8: UR-C08-lena moved ahead of و.

The other eight were rejected:

- Already startable: Arabic 3, Latin 48 and Tamil 8, after the previous pass.
- Arabic 45: AR-R45 is "The last page", and AR-W04 opens on "The last lesson
  asked…".
- Gujarati 2: every later word lesson depends on, or prints a letter from, the
  GU-W02 series that opens the chapter.
- Hindi 21: छह needs the छ that the opener teaches.
- Malayalam 4: there is no free sequence slot, and പോകുക's lesson prints വ
  from the opener.
- Punjabi 18: moving the oral R4 shortens the chapter-3 R4 distance from 132
  to 126 lessons, costs one R3 window, and breaks 21 Punjabi reinforcement and
  session-map pins.
