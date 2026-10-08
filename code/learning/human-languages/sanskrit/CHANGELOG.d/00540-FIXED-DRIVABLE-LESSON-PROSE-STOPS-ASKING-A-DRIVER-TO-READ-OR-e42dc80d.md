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

- **Count:** 13 drivable lessons; 11 `[YOU READ: …]` cues and 4 ear-and-voice
  rewrites.
- Notices SA-C140 (five lessons), the SA-C64-prathama-pathanam reading, and the
  SA-C66 / SA-C67 warm-ups ("read a short line aloud word by word", "read it
  aloud from the page, one letter-group at a time") become READ cues.
- SA-C52-ca "Read the phrase again and notice …" → "Say the phrase again …";
  SA-C63-first "Look for **एक** in it" → "Listen for …" and "Read it in three
  pieces" → "Take it in three pieces"; SA-C17-time-words "Look for an *-am* …"
  → "Listen for …".
- Left alone: "Read it literally" (SA-C53, SA-C57 — interpretation), "Read, as
  a noun." (a gloss), "Look at what it is made of" (the idiom).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (SA-C140, five lessons) keep the notice in narrated prose
    and defer only the look: "[YOU READ: the notice]" then "The notice says
    **…** — …". The first pass had put the whole notice inside the deferred cue,
    so a listener heard the comment on a notice without the notice; that
    superseded form is the one described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again"
    (SA-C64-prathama-pathanam): "[YOU READ: the passage again, and watch where
    **अत्र** turns into **तत्र**]".
  - SA-C63-first "Listen for **एक** in it. There is none — not a letter of it."
    → "Listen for *eka* in *prathamaḥ*. There is none — not a sound of it."
