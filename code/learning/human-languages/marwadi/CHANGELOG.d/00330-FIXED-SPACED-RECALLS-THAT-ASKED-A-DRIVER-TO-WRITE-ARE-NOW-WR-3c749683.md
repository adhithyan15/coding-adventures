## Fixed — spaced recalls that asked a driver to write are now writing cues

The drivable-writing detector in human-language-data now reads inside
`[YOU RECALL: …]` cues. RECALL is a spoken cue action, so the narration read a
spaced recall such as `[YOU RECALL: write **X** — **R2**]` to a driver as
"your turn — recall: write X — R2", with no deferral (issue #12070). Each such
cue in a drivable lesson of this track is now a `[YOU WRITE: … from memory]`
cue, the form the previous entry gave eight of them: WRITE is a manual action,
so the narration says "once you have stopped driving — write: …" and the book
prints "*Write it:* …". The spacing tag (**R1**–**R4**) and the distance note
stay in the cue, and "from memory" keeps what RECALL said in its name. Every
lesson stays `drivable: true`.

- This finishes the follow-up the previous entry left open: the eleven
  drivable lessons in chapters 37-39 that still carried the RECALL-write
  shape, 24 cues in all. **Lessons:** MW-C37-hear-baara, MW-C37-hear-chauda,
  MW-C37-hear-igyaara, MW-C37-hear-pandara, MW-C37-hear-tera,
  MW-C38-hear-athara, MW-C38-hear-satara, MW-C38-hear-sola,
  MW-C38-hear-ughanis, MW-C39-hear-koni, MW-R39-refuse-price.
- The figure recalls ("write the figures **०**, **१** and **२** — **R3**")
  put "from memory" after the whole list, so the cue still names all three.
- Eleven non-drivable Marwadi lessons carry the same cue shape (19 cues, in
  MW-C37-baara, MW-W38-a and their neighbours) and are untouched: their
  narration already opens with the hands-and-eyes notice, so the writing is
  hedged at the lesson level.
- Regenerated: book chapters 37-39, their narration (`.json` and `.txt`),
  the generated book and narration hashes, and the eleven
  `core/lesson-modality` owners (source hash only).
