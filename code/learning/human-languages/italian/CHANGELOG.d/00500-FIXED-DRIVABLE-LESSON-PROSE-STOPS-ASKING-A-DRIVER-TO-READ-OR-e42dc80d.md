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

- **Count:** 11 drivable lessons; 11 `[YOU READ: …]` cues and 2 ear-and-voice
  rewrites.
- Warm-ups IT-C114-lezione and IT-C115-compiti, notices IT-C139-la-ricevuta,
  IT-C139-laffitto, IT-C139-lappuntamento, IT-C139-lavviso, IT-C139-lorario,
  and the IT-C36-frasi / IT-C36-parole reading steps (with their "Again — and
  notice …") become READ cues.
- IT-C30-ne "Read that first Italian sentence again and count the negatives" →
  "Say that first Italian sentence again …"; IT-C34-nostro-vostro "Turn back to
  the very first lesson" → "Think back to the very first lesson".
- Left alone: "Look at what changed and what did not" (the idiom).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (IT-C139-la-ricevuta, IT-C139-laffitto,
    IT-C139-lappuntamento, IT-C139-lavviso, IT-C139-lorario) keep the notice in
    narrated prose and defer only the look: "[YOU READ: the receipt]" then "The
    receipt says **…** — …". The first pass had put the whole notice inside the
    deferred cue, so a listener heard the comment on a notice without the
    notice; that superseded form is the one described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (IT-C36-frasi): "[YOU
    READ: the six lines again, and notice …]".

