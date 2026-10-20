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

- **Count:** 18 drivable lessons; 12 `[YOU READ: …]` and 2 `[YOU COVER: …]`
  cues, and 8 ear-and-voice rewrites.
- Warm-ups GE-C112-teppich and GE-C113-dusche ("Read *der Flughafen, …* aloud
  as one line") and the GE-C52 reading lessons ("Read down once", "Again — and
  this time notice …", "Read it once without stopping", "Now read it again, and
  find …") become READ cues; notices GE-C143-anzeige, GE-C143-oeffnungszeiten,
  GE-C143-quittung, GE-C143-termin move the notice into the cue.
- GE-C15-partizip-practice "Cover the right-hand side and rebuild it from the
  left. Then cover the left and work backwards" → two COVER cues, the question
  after them in its own paragraph.
- Ear rewrites: "Read across each row" → "Go across each row" (GE-C09), "Read
  that last row twice" / "Read the middle row again" → "Go over …" (GE-C13,
  GE-C16), "Read it against the English" → "Set it against the English"
  (GE-C14), "Read the English gloss aloud" → "Say the English gloss aloud"
  (GE-C15-gestern), "Look closely at **gewesen**" → "Listen closely to
  **gewesen**", "then read on" → "then go on" (GE-C33).
- GE-C27-schliessen "Close your hand, both ways" (a gesture the gesture check's
  vocabulary did not hold) → "Say *I close my hand* both ways".
- Left alone: "Read the German for its meaning rather than its parts"
  (interpretation), "Look what happens when you line the languages up" (the
  idiom), "so do not read them off the page" (a request not to look).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (GE-C143-anzeige, GE-C143-oeffnungszeiten,
    GE-C143-quittung, GE-C143-termin) keep the notice in narrated prose and
    defer only the look: "[YOU READ: the advert]" then "The advert says **…** —
    …". The first pass had put the whole notice inside the deferred cue, so a
    listener heard the comment on a notice without the notice; that superseded
    form is the one described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (GE-C52-erste-lesung,
    GE-C52-saetze): "[YOU READ: the passage again, and find the one line where
    the verb is last]", "[YOU READ: the six lines again, …]".
  - GE-C09-zahlmonate "Go across each row and hear that nothing rhymes." → "Say
    each row through and hear that nothing rhymes."

