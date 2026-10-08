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

- **Count:** 11 drivable lessons; 12 `[YOU READ: …]` cues and 3 ear-and-voice
  rewrites.
- Warm-ups GU-C123-makhi, GU-C138-tarvu, GU-C141-jagvu, GU-C97-phagan ("Read on
  without stopping: **…**", "Read the line once, at speaking pace: **…**") and
  GU-C31-maaf ("Read the first word slowly.") become READ cues.
- GU-C37-kyaan: the prose "Then read the word for **city**" in front of an
  existing `[YOU READ: **શહેર**]` loses its "read"; "Read that row aloud before
  you go on." becomes a READ cue.
- Reading lessons GU-C44-lines, GU-C44-pehli-vanchan, GU-C44-words: the reading
  and the elliptical "Again — …" / "Now again, …" steps become READ cues; "look
  at the whole word" inside the cue is "take in the whole word".
- GU-C20-hear-mandir "Sort three heard cards" → "Sort three heard words";
  GU-C39-nav "Read that line twice" → "Go over that line twice" (an etymology
  line, not script).
- Left alone: "Do not read the Gujarati shape yet" (a request not to look),
  "Keep **તે** on the page" (keep it in mind), "Now look back over the chapter"
  (a review).
