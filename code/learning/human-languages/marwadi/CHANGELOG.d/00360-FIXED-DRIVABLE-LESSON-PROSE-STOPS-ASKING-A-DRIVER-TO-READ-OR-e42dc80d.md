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

- **Count:** 132 drivable lessons; 109 `[YOU READ: …]`, 51 `[YOU COVER: …]` and
  2 `[YOU WRITE: …]` cues, and 2 ear-and-voice rewrites.
- Look–cover–write routine: "Look, cover, and wait five seconds." (31 lessons)
  → `[YOU COVER: the model after one look, then wait five seconds]`; "Look for
  five seconds, cover **कांई**, and wait five seconds." → `[YOU COVER: **कांई**
  after a five-second look, then wait five seconds]`; "Read the line once and
  cover it for ten seconds." (C21..C25) → one READ cue. The WRITE cue that
  follows each is unchanged.
- Four-skill payoffs: "3. **Read:** match the printed question to the printed
  answer" → "3. **Reading:** [YOU READ: the printed question, then match it to
  the printed answer]" and siblings (C01..C09); "3. Match five printed cards to
  meanings" → `3. [YOU READ: five printed cards and match them to meanings]`
  (C10..C20); C21..C26, C32..C35, C39 and R32..R39 reading and ticket steps
  become READ cues.
- Spaced reviews R09..R25: "Hear all five …, say each from meaning, and read
  the five cards" → "… and say each from meaning. [YOU READ: the five cards]";
  "Read two cards." → a READ cue; R08-family-map's "Place four cards under the
  correct cues … Read each card and turn it over." → two READ cues; "Repair
  only a missed card" → "… a missed word".
- "Retrieve **हां सा** once by ear, voice, eye, and writing."
  (MW-C05-hear-kain, MW-C05-hear-naam) → "Retrieve … once by ear and voice.
  [YOU READ: **हां सा**] [YOU WRITE: **हां सा** once]"; "Close the page.",
  "Hide the line.", "then put the page out of sight", "Cover every written
  model." → COVER cues. MW-C04-practice "see a picture or imagine a glass of
  water" → "imagine a glass of water".
- Left alone: "Keep the page closed" and "Keep your pencil down" (states, not
  steps), "Look back at what the hand had to learn" (the idiom), "read the
  other when a page prints it" (advice for later), "Read in order, these six
  are …" (description).
