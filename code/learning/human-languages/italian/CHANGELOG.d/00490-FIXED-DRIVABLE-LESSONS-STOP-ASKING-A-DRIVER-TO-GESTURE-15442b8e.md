## Fixed — drivable lessons stop asking a driver to gesture

A spoken cue is read to a driver as an ordinary turn, and so is bare prose.
Drivable lessons in this track still asked for a hand or a gesture inside one:
a hand-wobble (issue #12070, ninth pass). Each ask is now said for the ear and
voice where that keeps the learning goal, or moved into a cue the narration
defers (`[YOU POINT: …]`, `[YOU READ: …]`: "once you have stopped driving —
…"). The new gesture check in human-language-data demands zero such spoken
cues in drivable lessons. Every edited lesson stays `drivable: true` (only its
`core/lesson-modality` source hash changes).

- **Count:** 2 spoken cues in 2 drivable lessons.
- IT-C02-prego-here-you-are: `"Prego" while handing over an imaginary cup`
  → `"Prego" as you offer someone a cup` (found by the security review; the
  gesture check now knows "hand over").
- IT-C02-cosi-cosi: "*koh-ZEE koh-ZEE*, with a hand-wobble" → "with a shrug in
  the voice". The warm-up's scene ("you wobble a hand and say *così così*")
  describes what Italians do and is unchanged.

