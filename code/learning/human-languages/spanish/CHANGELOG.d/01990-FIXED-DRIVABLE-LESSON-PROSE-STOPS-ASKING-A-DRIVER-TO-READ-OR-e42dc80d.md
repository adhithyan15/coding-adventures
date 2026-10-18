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

- **Count:** 97 drivable lessons; 47 `[YOU READ: …]` and 21 `[YOU WRITE: …]`
  cues, and 55 ear-and-voice rewrites.
- "Close the lessons before this one." opened 29 drivable `ES-R` reviews →
  "Leave the lessons before this one closed." (five that are not drivable keep
  it).
- Synthesis lessons ES-C431..C464: "Read it once straight through, then
  answer.", "Read both, then answer.", "A notice on a noticeboard. Read it,
  then answer." → READ cues; "Now your turn, out loud and then in writing. Four
  sentences: …" → "Now your turn, out loud. Four sentences: …" followed by
  `[YOU WRITE: the same four sentences]` (21 lessons — writing in prose that
  the writing check's verbs did not see).
- "Read aloud. Every word has been taught:" (ES-C421, ES-C423, ES-C427), "Read
  this note and count the marks", "Read this exchange aloud", "Read this
  invitation aloud …", "Read both versions aloud", "Read this aloud and watch
  …" (ES-C44, ES-C46), "Read the second version aloud", "Read it before you
  read the explanation", ES-C09-sintesis-ocho "Read it slowly", ES-C40 "Read it
  again and count what it took", ES-C470 "Read the last sentence again", and
  the ES-C67 readings (including "Once through, without stopping.") become READ
  cues.
- Ear rewrites where the sentence is said, not studied: "Read those twice" /
  "Read both aloud" / "Read this twice, changing only one word" / "Read the
  second line again" / "Read your three sentences back" → "Say …"; "Look again
  at that sentence and count the negatives" → "Say that sentence again …";
  "read on" / "before reading further" → "go on" / "before going further";
  "Look around the room you are in" → "Picture the room you are in"; "Look for
  the person doing it" → "Listen for …"; "Look down the two columns" → "Compare
  the two columns"; "Look hard at that third row" → "Focus on …"; "Read what
  each tense is doing:" → "Here is what …"; "Now read only the imperfects" →
  "Now take only …"; "Keep the pair … in view" → "… in mind".
- Left alone: the titles "(…) — read it, then do it yourself", "Read in order
  they are …" (description), "When you meet **celebrarse** on a notice, read it
  as **is held**" and "look for a verb inside it and read the whole as …"
  (advice for later), "Read the door before you lean on it" (advice for the
  street), "Open the Academy's dictionary at *euro* and find two entries" (a
  narrative conditional — the next paragraph gives them), "Then check yourself:
  did you …" (a spoken check), "Fill in the line out loud".
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - "Leave the lessons before this one closed. Say all five: …" (the form
    described above, 29 drivable ES-R431..R464 reviews) → "From memory alone,
    say all five: …".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (24 lessons): the
    ES-C450..C463 synthesis lessons name the document their opening sentence
    describes, "A notice on the gate of the sports centre. [YOU READ: the
    notice, then answer]" (card, sheet, message, poster), where the first pass
    left "[YOU READ: it, then answer]"; ES-C431..C442 "[YOU READ: the notice
    once straight through, then answer]" (advert, message, page); ES-C67 "[YOU
    READ: the passage once through, …]"; ES-C44, ES-C46 "[YOU READ: the exchange
    aloud and watch …]"; ES-C421 "[YOU READ: the scene aloud]"; ES-C56 "[YOU
    READ: the sentence before the explanation]".
  - A spoken premise, gloss or answer that the first pass had moved inside a
    deferred cue is said in prose again, and the cue keeps only the look
    (ES-C40-repaso-un-sabado-de-octubre): "[YOU READ: the passage again and
    count what it took]" then "It took a day name, a month, a season, …".
  - ES-C391-euro "Open the Academy's dictionary at *euro* and find two entries,
    not one" → "The Academy's dictionary has two entries at *euro*, not one"
    (description).

