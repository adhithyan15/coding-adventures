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

- **Count:** 18 drivable lessons; 16 `[YOU READ: …]`, 2 `[YOU COVER: …]`, 2
  `[YOU CHECK: …]`, 1 `[YOU FIND: …]` and 1 `[YOU WRITE: …]` cues, and 6
  ear-and-voice rewrites.
- TE-C03-practice "Read the whole cycle off the page first", TE-C05-practice
  "read off the page before you say them", the TE-C112 / TE-C142 warm-ups,
  "Read two words again: **ఛాయ** … and **ఝరి** …" (TE-C153, TE-C156-rasidu,
  TE-C171), the TE-C156 notices and the TE-C83 reading lessons become READ
  cues.
- TE-R151 / TE-R152: "Read **ఛాయ** once. Cover it." / "Look once at **ఝరి**,
  then cover it." → READ and COVER cues; "Now uncover the word and check two
  things …" → a CHECK cue (UNCOVER is not a cue verb), and "Add the long **ా**
  sign only after the base letter is steady" → a WRITE cue. TE-C152-jhari "Now
  find the familiar **రి** …" → a FIND cue.
- "Read the word as **ఛా-య**" / "**ఝ-రి**" → "Say the word as …"; TE-C68-song
  "Watch the middle letter" → "Listen for the middle letter"; TE-C06 "Close the
  book on everything so far" → "Leave the book closed on everything so far".
- Left alone: "look closer, and it's the same word" (TE-C13 — figurative),
  "heard from elders and read at the top of letters", "you have now read it
  doing so" (description).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - TE-C06-second-pass-greetings drops "Leave the book closed on everything so
    far." (the form described above): "From memory: …" follows it.
  - The notice lessons (TE-C156-kalapattika, TE-C156-prakatana, TE-C156-sucana)
    keep the notice in narrated prose and defer only the look: "[YOU READ: the
    notice]" then "The notice says **…** — …". The first pass had put the whole
    notice inside the deferred cue, so a listener heard the comment on a notice
    without the notice; that superseded form is the one described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (TE-C83-lines,
    TE-C83-modati-chadavu, TE-R151-chhaya-recall, TE-R152-jhari-recall): "[YOU
    READ: the six lines again, …]"; TE-R151, TE-R152 "[YOU READ: **ఛాయ** once]
    [YOU COVER: it]" → "[YOU READ: **ఛాయ** once]", "[YOU COVER: the word]", each
    in its own paragraph.
