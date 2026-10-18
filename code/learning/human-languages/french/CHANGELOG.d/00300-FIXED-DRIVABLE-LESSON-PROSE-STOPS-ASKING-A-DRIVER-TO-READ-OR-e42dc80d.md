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

- **Count:** 13 drivable lessons; 16 `[YOU READ: …]` cues and 1 ear-and-voice
  rewrite.
- FR-C09-mois-practice-1, FR-C09-mois-practice-2, FR-C09-saisons-practice:
  "Read down the left, then cover the right and read back:" → one READ cue
  before the table.
- FR-C126-lecon and FR-C127-reponse warm-ups ("Read the instruction and do it:
  *Dites « la classe ».* Then read *le stylo, …* aloud as one line") and
  FR-C45-details, FR-C45-instructions, FR-C45-premiere-lecture ("Read down
  once", "Again — and notice the colon", "Read it once without stopping …",
  "Now read it again …") become READ cues.
- Notices FR-C142-la-facture, FR-C142-lannonce, FR-C142-lavis,
  FR-C142-lhoraire: the notice moves into the READ cue; the comment follows.
- FR-C25-lire wrap-up "Now read these two aloud. (*Le chien, le chat*.)" → "Now
  say these two aloud."
- Left alone: "Read it literally", "Read literally:", "read it as the single
  vowel", "Read it as a sum", "Read that as Latin", "Read them as a **shape**"
  (interpretation — how to take a form — which a listener can do), and "Look at
  what is holding it together" (the idiom).
- **Review follow-up** (same change, second commit). The first pass's rewrites lost some of what a listener needs and broke some of what the book prints; this track's share of the fixes:
  - The notice lessons (FR-C142-la-facture, FR-C142-lannonce, FR-C142-lavis,
    FR-C142-lhoraire) keep the notice in narrated prose and defer only the look:
    "[YOU READ: the bill]" then "The bill says **…** — …". The first pass had
    put the whole notice inside the deferred cue, so a listener heard the
    comment on a notice without the notice; that superseded form is the one
    described above.
  - Cues that opened with *it*, *them*, *this* or *these* name their object, so
    the book no longer prints "*Read it:* them again" (FR-C45-details,
    FR-C45-premiere-lecture): "[YOU READ: the six lines again, and notice the
    colon]", "[YOU READ: the passage again, …]".
  - A new cue followed by prose or by another cue in the same paragraph now ends
    its paragraph, so the book no longer runs "*Cover:* the page The next
    question …" together (FR-C126-lecon).

