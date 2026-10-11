## Fixed — drivable lesson prose stops asking a driver to read or handle cards

Narration reads bare prose aloud as written, so a prose instruction to read
printed script, handle cards or cover the page reached a driver unhedged (issue
#12070, tenth pass): "Hear, picture the part, say, and read **おなか**." was
narrated word for word. Each such step now moves into a cue the narration
defers (`[YOU READ: …]`, `[YOU COVER: …]`, `[YOU CHECK: …]`, `[YOU FIND: …]`,
`[YOU WRITE: …]`: "once you have stopped driving — …"), in the authored order,
or is said for the ear and voice where the step was not about the page. Prose
that followed a new cue in the same paragraph now has a paragraph of its own.
The new prose check in human-language-data demands zero such spans in drivable
lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 24 drivable lessons; 11 `[YOU READ: …]`, 14 `[YOU COVER: …]` and 6
  `[YOU CHECK: …]` cues, and 5 ear-and-voice rewrites.
- Script recalls MR-R09-script-a..d-r3 and MR-R18-script-a..d-r4: "Close the
  earlier script pages and keep one blank line ready", "Keep every model closed
  and one blank line ready" and "Cover the answer." → COVER cues; "Compare:
  **धन्यवाद; ः आ भ**" / "Compare once: **…**" → `[YOU CHECK: your answer
  against **…**]`. MR-R18-script-warmup and MR-R08-runway-recall likewise
  ("Close every model" → `[YOU COVER: every model]`, then "Now find out what
  stayed.").
- MR-C01-practice, MR-C129-granthalay, MR-C135-pariksa, the MR-C161 notices and
  the MR-C62 reading lessons become READ cues; MR-R61 "Open Molesworth's
  *Dictionary* … and look up all ten:" → `[YOU CHECK: Molesworth's … for all
  ten]`.
- "Close the lessons before this one." (MR-R68, MR-R69) → "Leave the lessons
  before this one closed."; MR-R05 "Close the chapter." → "Leave the chapter
  closed."; MR-C38 / MR-C47 "Read those two pairs across" → "Compare those two
  pairs".
- Left alone: "Look up **पहिला** in Molesworth's 1857 dictionary and it is
  there" (a narrative conditional — the sentence says what is there), "Sort
  them into two classes before you say anything" (a spoken sort), "Listen
  before picking up the pencil" (a time, not a step). MR-R66 and MR-R67 are not
  drivable and keep their wording.
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - "Leave the lessons before this one closed." (MR-R68, MR-R69, the form
    described above) → MR-R68 "From memory alone, say the one word this chapter
    taught, …"; MR-R69 drops it in front of "[YOU WRITE: the grid from memory —
    …]". MR-R05 "Leave the chapter closed." is dropped: the next sentence,
    "Nothing here needs the page", carries it.
  - The notice lessons (MR-C161-jahirat, MR-C161-sucana, MR-C161-velapatrak)
    keep the notice in narrated prose and defer only the look: "[YOU READ: the
    advert]" then "The advert says **…** — …". The first pass had put the whole
    notice inside the deferred cue, so a listener heard the comment on a notice
    without the notice; that superseded form is the one described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (MR-C62-message,
    MR-C62-pahila-paath): "[YOU READ: the passage once without stopping; …]",
    "[YOU READ: the message again, …]".
  - A new cue followed by prose or by another cue in the same paragraph now ends
    its paragraph, so the book no longer runs "*Cover:* the page The next
    question …" together (MR-R09-script-a..d-r3, MR-R18-script-warmup).
