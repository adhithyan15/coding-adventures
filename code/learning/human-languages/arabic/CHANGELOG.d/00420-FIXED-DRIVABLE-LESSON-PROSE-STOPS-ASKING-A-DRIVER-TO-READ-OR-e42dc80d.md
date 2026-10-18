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

- **Count:** 17 drivable lessons; 21 `[YOU READ: …]` cues and 2 ear-and-voice
  rewrites.
- Script steps: `Read **قال**.`, `Read **قرأ**.`, `Read **ال** and say what it
  means.`, `Read **أخذ** in its three pieces.`, `Read **قال** right to left:
  *qāf*, long *ā*, *lām*.` and the C134/C136/C154 warm-ups (`Read these turns
  without stopping: **…**`) become READ cues; the question after each ("Which
  letter is the deep *q*?") follows in its own paragraph (AR-C100-misad,
  AR-C127-qifl, AR-C129-tamr, AR-C132-shallal, AR-C134-arnab, AR-C136-timsah,
  AR-C154-qawi, AR-C29-qala).
- Reading lessons AR-C42-words, AR-C42-lines and AR-C42-first-passage: "Read
  down once, right to left, without translating.", "Read again.", "Read the
  whole encounter once." and "Read it again. This time notice the hinge: **شاي
  أو حليب؟**" become READ cues, the noticing inside the cue; the warm-up "Now
  read one encounter from its first peace to its last" is now "Now one
  encounter, from its first peace to its last" (it introduces the reading
  rather than asking for it). AR-C01-practice ("Read all six greetings aloud,
  right to left", "Sound each out **right to left** before checking:") and
  AR-C02-practice ("Read right to left:") likewise.
- Notices AR-C158-ilan, AR-C158-jadwal, AR-C158-lafita: `Read the advert: **شقة
  للإيجار** — *shaqqa li-l-ījār* — "Flat for rent."` → `[YOU READ: the advert —
  **شقة للإيجار** — *shaqqa li-l-ījār* — "Flat for rent."]`, the comment after
  it in its own paragraph.
- AR-C23-afwan "Look closely at the spelling:" → "Notice the spelling:".
- Left alone: "**من فضلك** — read right to left, **min faḍlik** — is two
  pieces" (a gloss of how the phrase runs), "Look at what Chapter 1 opened
  with" and "Look what grows from it" (the idiom for *consider*, not the page).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (AR-C158-ilan, AR-C158-jadwal, AR-C158-lafita) keep the
    notice in narrated prose and defer only the look: "[YOU READ: the advert]"
    then "The advert says **شقة للإيجار** — *shaqqa li-l-ījār* — \"Flat for
    rent.\"". The first pass had put the whole notice inside the deferred cue,
    so a listener heard the comment on a notice without the notice; that
    superseded form is the one described above.
  - A spoken premise, gloss or answer that the first pass had moved inside a
    deferred cue is said in prose again, and the cue keeps only the look
    (AR-C29-qala, AR-C42-first-passage): "**قال** runs right to left: *qāf*,
    long *ā*, *lām*. [YOU READ: **قال** right to left]"; AR-C42 says the hinge
    **شاي أو حليب؟** in prose.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (AR-C42-lines,
    AR-C42-words): "[YOU READ: the six lines again]", "[YOU READ: the six words
    again]".

