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

- **Count:** 15 drivable lessons; 15 `[YOU READ: …]` cues and 1 ear-and-voice
  rewrite.
- "Read them; you'll draw them later" (RU-C02), the RU-C111 warm-ups, "Read the
  spelling and then read the sound again" (RU-C18-chto-vopros), "Say it and
  read it at the same time: **т-р-и**" (RU-C23-tri), the RU-C23..C25 digit
  steps and the RU-C27-notices reading become READ cues; notices RU-C139 move
  the sign into the cue (RU-C139-vkhod / vykhod keep "you go in here" and "the
  way out" inside it).
- RU-C18-znayu-chto "Read those two English translations again" → "Go back over
  those two English translations".
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (RU-C139-raspisaniye, RU-C139-reklama, RU-C139-vkhod,
    RU-C139-vykhod) keep the notice in narrated prose and defer only the look:
    "[YOU READ: the sign over a door]" then "The sign over a door says **ВХОД**:
    you go in here." (the gloss "You go in here." was inside the cue). The first
    pass had put the whole notice inside the deferred cue, so a listener heard
    the comment on a notice without the notice; that superseded form is the one
    described above.
  - RU-C02-ochen-priyatno "[YOU READ: them now — you'll draw them later]" →
    "[YOU READ: both letters]" and "You'll draw them later." in prose; RU-C02-ya
    the same with "[YOU READ: **я** here]".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (RU-C27-notices): "[YOU
    READ: the six notices again, and notice the list turns over halfway]".
- **Second review follow-up:** RU-C25-practice's cue reads "the twelve amounts, printed and shuffled" (the noun was missing).

