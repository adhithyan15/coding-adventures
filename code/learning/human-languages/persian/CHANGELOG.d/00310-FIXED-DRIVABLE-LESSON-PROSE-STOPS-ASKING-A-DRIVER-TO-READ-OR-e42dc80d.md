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

- **Count:** 26 drivable lessons; 27 `[YOU READ: …]` and 2 `[YOU CHECK: …]`
  cues, and 3 ear-and-voice rewrites.
- Letter-by-letter readings ("Read from right to left: **ح** *h* + **ا** long
  *â* + **ل** *l*.", "Read from the right.", "Read **ح ا ف ظ** from right to
  left.") and the right-edge rereadings (FA-C10-dokhtar, FA-C11-cheshm) become
  READ cues, the explanation after each in its own paragraph (FA-C01..C06).
- Digit and reading steps (FA-C16..C21, FA-C114, FA-C130) and the FA-C139
  notices become READ cues; FA-C18-practice "Then check each against a printed
  list" and FA-C01-practice "Then compare your **ا** with the model" → CHECK
  cues.
- FA-C01-practice "Keep every model visible" → "Every model stays in view";
  FA-C14-khahar "Read it **kh-â-har**" → "Say it …"; FA-C13-khorshid "Look up,
  and this lesson names what is in it" (eyes off the road) → "Picture the sky,
  and …".
- Left alone: the descriptive "From the right: **ف** *f*, …" (no instruction),
  "read simply as English *z*" / "How is its final **ه** read?" (how a letter
  is read), "Look at what is **not** there" (the idiom).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (FA-C139-agahi, FA-C139-jadval-e-zamani, FA-C139-tablo)
    keep the notice in narrated prose and defer only the look: "[YOU READ: the
    advert]" then "The advert says **…** — …". The first pass had put the whole
    notice inside the deferred cue, so a listener heard the comment on a notice
    without the notice; that superseded form is the one described above.
  - FA-C02-esm-e-man: the speaking step comes out of the reading cue, "[YOU
    READ: the phrase from right to left]" then "Say its words in this order: …".
    A spoken premise, gloss or answer that the first pass had moved inside a
    deferred cue is said in prose again, and the cue keeps only the look
    (FA-C03-chist, FA-C04-hal, FA-C05-khoda): "From the right: **چ** *ch*, **ی**
    long *i*, **س** *s*, **ت** *t*. [YOU READ: the word from the right]".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (FA-C21-adad,
    FA-C21-answers): "[YOU READ: the lines again, …]", "[YOU READ: the six lines
    again, …]".
  - FA-C01-practice drops "Every model stays in view." (the first pass's rewrite
    of "Keep every model visible"): the step that follows, "Say **سلام** once to
    greet and once to answer", needs no page.

