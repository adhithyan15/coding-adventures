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

- **Count:** 25 drivable lessons; 28 `[YOU READ: …]` cues and 1 ear-and-voice
  rewrite.
- Letter-by-letter readings ("Read right to left: **ک** *k*, …", "Read from the
  right edge: **ہ** *h*, …", "Read **خدا** first, then **حافظ**, and preserve
  that space …"), UR-C17's "**Read it off the page.**", "Read that sentence off
  the page", "Read these off the page, naming the letters …", "read down the
  right edge", the digit payoffs UR-C28..C32, the UR-C113 / C114 warm-ups, the
  UR-C146 notices and the UR-C33 reading lessons become READ cues.
- UR-C09-bahan "Read on for why." → "The reason comes next."
- Left alone: "Look at what happened to the noun", "Look hard at the plural"
  (the idiom), "a word you hold by ear, not yet by eye" (description).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (UR-C146-ishtihar, UR-C146-notis, UR-C146-taim-tebal)
    keep the notice in narrated prose and defer only the look: "[YOU READ: the
    advert]" then "The advert says **…** — …". The first pass had put the whole
    notice inside the deferred cue, so a listener heard the comment on a notice
    without the notice; that superseded form is the one described above.
  - A spoken premise, gloss or answer that the first pass had moved inside a
    deferred cue is said in prose again, and the cue keeps only the look
    (UR-C03-kya, UR-C05-khuda, UR-C06-hona, UR-C17-kaam): "Right to left: **ک**
    *k*, **ی** …, then **ا** long *ā*. [YOU READ: the word right to left]".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (UR-C17-yih, UR-C33-lines,
    UR-C33-pehla-sabaq): "[YOU READ: the two words off the page, …]", "[YOU
    READ: the six lines again, …]".
  - Four-skill items drop the "**Reading:**" label in front of a READ cue, which
    the book printed as "Reading: *Read it:* …" (UR-C30-practice).
