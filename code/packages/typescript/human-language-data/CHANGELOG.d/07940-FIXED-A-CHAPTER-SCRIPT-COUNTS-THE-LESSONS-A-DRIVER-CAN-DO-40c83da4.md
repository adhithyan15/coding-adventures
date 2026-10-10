### Fixed — a chapter script counts the lessons a driver can do

Each chapter's narration script opens by saying how far a driver gets. That
count used the lesson's FULL modality, while the modality summary and the
book's "Hands-free start" line use its CORE (the lesson with its detachable
writing and letters sections set aside). The two disagreed whenever a lesson
was voice apart from such a section. After the writing ramp put a one-line
trace block into lesson one of six tracks, eight chapter-1 scripts (Punjabi,
Gujarati, Sanskrit, Malayalam, French, Spanish, plus Tamil and German) said
"The first lesson already needs your eyes or your hands, so save this one for
when you have stopped". In fact the lesson could be done in the car.

What changes in `src/narration.ts`:

- `LessonNarration` gains `coreModality`. `narrateChapter` counts
  `drivablePrefix` on it, the same rule as `drivablePrefix` in `modality.ts`.
  It also adds `prefixWithPartsSetAside`: how many of those lessons are
  `pen`/`sight` in full.
- `renderChapterNarrationText` adds one sentence when that number is above
  zero, worded for the count, with an updated truth table in the comment.
  For example: "Of those 3, one has a part that needs your eyes or your hands;
  that part waits until you have stopped." It also drops "entirely" from "can
  be done entirely by ear".
- The lesson notice now agrees with the header. A lesson whose core is voice
  used to open with "this one needs your hands, so it is not a driving lesson.
  You can listen to all of it now and come back to the parts that need looking
  at …". That named no section, because a `## Writing — …` block with no
  bracketed cue was never listed. It now opens "you can do this one in the car,
  but part of it needs your hands" (or eyes). It names every non-voice
  detachable section to leave until the driver has stopped. The needs sentence
  becomes "For that part you will want …".
- The section itself is still read in place, in body order. Hands-on cues
  inside it are still deferred the way #17100 made them.

What changes in `src/modality.ts`:

- An authored override that raises a lesson with no detachable section now
  raises its core too. The cap only ever lowered before, so
  `PA-R171-date-repair-again` kept a `voice` core and counted as drivable.
  Its author marks it `pen` ("this task is not drivable"), and the cue
  detector reads its prose as voice. This restores the documented invariant
  "core equals modality when nothing is detachable". It moves seven Punjabi
  lesson-modality rows (PA-R168…R171), and only PA-R171 changes from a
  `voice` core. No book, and no chapter header, changes because of it.

Regenerated output: 152 chapter headers change across 22 tracks (arabic 11,
bengali 14, chinese 1, french 2, german 1, gujarati 20, hindi 11, italian 1,
kannada 9, latin 1, malayalam 9, marathi 15, marwadi 1, persian 4,
portuguese 1, punjabi 13, russian 4, sanskrit 7, spanish 3, tamil 9,
telugu 6, urdu 9). 115 of them used to say "the first lesson already needs
your eyes or your hands". Their JSON `drivablePrefix` and hash shards move
with them. 617 lesson notices change wording. One-lesson chapters already
said "It can be done entirely by ear." (241 of them) and still do.

Spec: HL08 records the core-based header, the notice wording and the new
override rule 6. Tests: new narration cases cover each row of the header's
set-aside table and the core-voice notice. New modality cases cover a raised
override with and without a detachable section.

The spoken notice also names the sections to leave until the driver has
stopped at the END of its sentence ("… but leave this section until you have
stopped: Writing — let your finger meet one shape."). Section titles carry
their own dashes, colons and commas, and read aloud mid-sentence ("leave the
section called Writing — let your finger meet one shape until you have
stopped") the listener could not hear where the title ended.

The notice's promise, "I will say so again when we reach it", is now kept:
`renderLessonNarrationText` speaks `STOP_GUARD` ("[once you have stopped
driving — this part needs your eyes or your hands; if you are driving, skip
ahead to the next part]") at the top of every section the notice named,
before any of its content. A security review caught that without it a lesson
newly counted as drivable played its hands-on writing instructions straight
through at the wheel; the promise was already unkept on main for lessons that
were not drivable.
