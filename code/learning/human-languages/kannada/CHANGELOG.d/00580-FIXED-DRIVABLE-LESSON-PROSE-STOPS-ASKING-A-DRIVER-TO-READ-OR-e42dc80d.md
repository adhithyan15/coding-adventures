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

- **Count:** 18 drivable lessons; 19 `[YOU READ: …]` and 1 `[YOU FIND: …]`
  cues, and 2 ear-and-voice rewrites.
- KA-C01-practice "Sound out the three written in Kannada …" and "Read all five
  words aloud", KA-C07-naalku "Read the word slowly: **ನಾ** · **ಲ್** · **ಕು**",
  KA-C75-on-a-sign "Read **೧ನೇ** aloud", KA-C109-byanku / KA-C149-enisu "and
  read the sign form **೧ನೇ**", the digit and line warm-ups (KA-C118, KA-C137,
  KA-C138), the notices KA-C157 and the KA-C76 reading lessons (with their
  "Again — …") become READ cues.
- KA-C118-lekhani "Then find **ಠ** and say *ṭha*" → `[YOU FIND: **ಠ**, then say
  *ṭha*]`.
- KA-C59-sand "Read it slowly and you will see ಮರ …" → "Say it slowly and you
  will hear ಮರ …"; KA-C62-umbrella "Read it carefully" → "Say it carefully"
  (the shared start is audible).
- Left alone: "Read literally it says …" and "Read it in Kannada order: …"
  (interpretation), "Look at what changed" (the idiom), "When a word will not
  stay in the ear, ask for it on paper" (advice for a real conversation).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (KA-C157-jahiratu, KA-C157-suchane, KA-C157-velapatti)
    keep the notice in narrated prose and defer only the look: "[YOU READ: the
    advert]" then "The advert says **…** — …". The first pass had put the whole
    notice inside the deferred cue, so a listener heard the comment on a notice
    without the notice; that superseded form is the one described above.
  - A spoken premise, gloss or answer that the first pass had moved inside a
    deferred cue is said in prose again, and the cue keeps only the look
    (KA-C07-naalku, KA-C75-on-a-sign): "The word breaks into **ನಾ** (*nā*) ·
    **ಲ್** · **ಕು** (*ku*). [YOU READ: the word slowly]"; KA-C75 "[YOU READ:
    **೧ನೇ** aloud]" then "It is *Modalanē* — not \"one-nē\".".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (KA-C76-lines,
    KA-C76-modala-oduvike): "[YOU READ: the six lines again, …]", "[YOU READ:
    the passage again, and watch **ಇಲ್ಲಿ** and **ಅಲ್ಲಿ** take turns]".
