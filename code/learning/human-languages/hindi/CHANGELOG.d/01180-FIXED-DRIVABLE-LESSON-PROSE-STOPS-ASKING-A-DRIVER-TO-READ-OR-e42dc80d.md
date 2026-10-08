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

- **Count:** 37 drivable lessons; 25 `[YOU READ: …]`, 2 `[YOU FIND: …]` and 1
  `[YOU COVER: …]` cues, and 12 ear-and-voice rewrites.
- Reading: HI-C01-practice "Read each aloud, then recall its meaning …",
  HI-C06-numbers-1-5 "Read them now; you'll draw them …", HI-C78-notices /
  HI-C78-pehla-paath, HI-C87, HI-C88, HI-C90-sintesis-dincharya,
  HI-C91-sintesis-sangya and the synthesis lessons' "Read aloud. Every word has
  been taught:" (HI-C82..C86) become READ cues ("Every word has been taught:"
  keeps its paragraph above the passage). Notices HI-C166 and HI-C167 (ten
  lessons) move the notice and its transliteration into the cue.
- HI-C92-achchha-buraa "Find it in the middle of the word." and HI-C88-aaunga
  "Find **ऊ** inside the word:" → FIND cues; HI-R94-met-once-and-left-there
  "Cover everything above." → a COVER cue.
- Ear rewrites: "read on to find out" → "go on to find out" (HI-C82, HI-C84,
  HI-C86, HI-C90-baad, HI-C91-bhashaen); "Read it again in **बस से**" → "Now
  meet it in …"; "Read **बच्चे को** and notice" → "Take …"; "Read the word:
  **क्योंकि** is …" → "Take the word apart: …"; "Read that table down the left
  / right" → "Go down the left / right"; "then read the word after it" → "then
  take …"; "Look closely at the spelling" → "Notice the spelling".
- Left alone: "Read the frame as three beats", "Read the pair as two small
  packages", "Read it inside out" (interpretation), "Find **नहीं** first" (a
  parsing strategy, audible), "Look at what that gives you" (the idiom), and
  the answer "(**No** — read them now and draw them when their stroke lessons
  arrive.)".
