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

- **Count:** 32 drivable lessons; 34 `[YOU READ: …]` and 1 `[YOU CHECK: …]`
  cues, and 1 ear-and-voice rewrite.
- Before-the-new-one warm-ups TA-C67..C73 ("read **கடை**, and say what it
  means") → `[YOU READ: **கடை**, then say what it means]`; TA-C74..C81 "Read
  **சரியா**. Then ask to be let inside." → READ cue, the spoken step in its own
  paragraph; TA-C65-stop / TA-C66-inside "Read **…** off the page" likewise.
- TA-C84-lines, TA-C84-mudhal-vaasippu, TA-C84-words (with "Again — and look at
  where the question word sits"), the TA-C120 / TA-C151 warm-ups and the
  TA-C160 notices become READ cues.
- TA-C01-practice "Keep that model visible when you copy it again below" → "The
  copy below keeps that model in view"; "Compare your one copied **வ** with the
  model. Repair one curve if you want, then stop." → a CHECK cue.
- Left alone: "Next: place the word in its real register and read the doubled
  alveolar n" (a preview of the next lesson), "read the full one nearly always"
  (advice), "Why is its **ட** read as a soft *ḍ*?" (how a letter is read),
  "Keep your pencil down".
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - TA-C66..C73 (eight lessons): the first pass put the spoken warm-up inside
    the reading cue; the step is spoken again, "Before the new one, say what
    **வண்டி** means. [YOU READ: **வண்டி**]" (TA-C71-run keeps its answer "(\"To
    run\".)" spoken before the cue).
  - The notice lessons (TA-C160-arivippu, TA-C160-kala-attavanai,
    TA-C160-vilamparam) keep the notice in narrated prose and defer only the
    look: "[YOU READ: the notice]" then "The notice says **…** — …". The first
    pass had put the whole notice inside the deferred cue, so a listener heard
    the comment on a notice without the notice; that superseded form is the one
    described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (TA-C84-lines,
    TA-C84-mudhal-vaasippu): "[YOU READ: the six lines again, …]", "[YOU READ:
    the passage again, and notice which lines have no verb at all]".
