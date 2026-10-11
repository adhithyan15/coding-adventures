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

- **Lessons:** BN-C27-ditiyo, BN-C27-ponchom (two cues).
- BN-C27-ponchom's "write **ঞ**, and say where it is made" keeps the spoken
  half inside the cue, after the writing it refers to.
- Regenerated: book chapter 38, its narration (`.json` and `.txt`), the
  generated book and narration hashes, and the two `core/lesson-modality`
  owners (source hash only).
