## Fixed — spaced recalls that asked a driver to write are now writing cues

The drivable-writing detector in human-language-data now reads inside
`[YOU RECALL: …]` cues. RECALL is a spoken cue action, so the narration read a
spaced recall such as `[YOU RECALL: write **X** — **R2**]` to a driver as
"your turn — recall: write X — R2", with no deferral (issue #12070). Each such
cue in a drivable lesson of this track is now a `[YOU WRITE: … from memory]`
cue, the form the Marwadi fix used: WRITE is a manual action, so the narration
says "once you have stopped driving — write: …" and the book prints "*Write
it:* …". The spacing tag (**R1**–**R4**) and the distance note stay in the
cue, and "from memory" keeps what RECALL said in its name. Every lesson stays
`drivable: true`.

- **Lessons:** MR-C60-tisra, MR-R40-asking-column, MR-R44-directions (three
  cues).
- MR-R40-asking-column: "write **इथे**, then **तिथे**, and check which one
  needed a whole vowel letter" is one writing task with its check, so the
  whole of it is the cue.
- MR-R44-directions: "answer *kuṭhe?* with *ithe*, then *tithe*, and write
  **तिथे** once" had a spoken half a driver can do now. It stays a recall
  cue, and the writing becomes its own list item,
  `[YOU WRITE: **तिथे** once, from memory]`. This one has no verb at the start
  of the cue: the "?" inside *kuṭhe?* ends a clause, and the detector reads a
  writing verb chained on anywhere in a recall cue for exactly this case.
- Regenerated: book chapters 40, 44 and 60, their narration (`.json` and
  `.txt`), the generated book and narration hashes, and the three
  `core/lesson-modality` owners (source hash only).
