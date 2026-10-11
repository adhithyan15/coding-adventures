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

- **Count:** 22 drivable lessons; 20 `[YOU READ: …]`, 3 `[YOU COVER: …]` and 1
  `[YOU WRITE: …]` cues, and 1 ear-and-voice rewrite.
- R4 reviews PA-R24 / PA-R25 / PA-R27 / PA-R28: "Read **ਪਾਣੀ · ਚਾਹ** once." →
  READ cues; PA-R27's "**Read, then stop.**" label folds into the cue (`[YOU
  READ: **ਕੰਨ · ਮੂੰਹ** once, then stop]`).
- PA-C43-kitthe "Read it in pieces: **ਕ**, the **ਿ** sihari …", PA-C133-murgi,
  PA-C138-bag, PA-C143-sukka, PA-C148-lukna, PA-C103-visakh and the PA-C45
  reading lessons become READ cues.
- PA-R158 reviews: "Cover the answer before the next cue", "Cover the three
  answers", "Cover the earlier answers" → COVER cues. PA-R18-wellbeing-r4
  "listen, speak, then type only a record" → "listen, then speak. [YOU WRITE:
  then only a typed record …]".
- PA-R48 "Read across rather than down" → "Take it across rather than down".
- Left alone: "Check: **ਸੱਚਾ** (*saccā*), then **ਝੂਠਾ**" (the answer is spoken
  after the pause, so a driver checks by ear), "Keep the two meanings separate
  before looking at the script" (a time, not a step), "Look for *ṭhāk* in a
  dictionary and it will not be there" (a narrative conditional).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - A spoken premise, gloss or answer that the first pass had moved inside a
    deferred cue is said in prose again, and the cue keeps only the look
    (PA-C43-kitthe, PA-C148-lukna): "In pieces: **ਕ**, the **ਿ** sihari before
    it, … [YOU READ: the word in pieces]"; "The form line says **ਕੰਮ: ਨੌਕਰੀ**.
    [YOU READ: the form line once]".
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (PA-C45-pehla-paath):
    "[YOU READ: the passage again, and notice how little work it took]".
  - PA-R158-food-quality-recall "Check only after all three: **ਕੱਚਾ**, …" →
    "Only after all three, hear the answers: **ਕੱਚਾ**, **ਬੇਹਾ**, **ਪੱਕਾ**.";
    PA-R158-truth-and-food-check "Check: **ਸੱਚਾ** …" and "Check in that same
    order: …" → "Hear the answers: …" and "Hear the answers in that same order:
    …", "Reopen" → "Revisit". Both "[YOU COVER: …] [PAUSE 4s each] Say …"
    openings give the spoken step its own paragraph. The corpus pin on "does not
    award independent Gurmukhi writing evidence" is untouched.
