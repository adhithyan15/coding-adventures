## Fixed — a drivable recall no longer asks a driver to point at the page

HI-C51-sweet is drivable, and its guided practice ended with
`[YOU RECALL: say *mulāqāt*, then read **प्रणाम** and point to its **ण**]`.
RECALL is a spoken cue action, so the narration read it to a driver as an
ordinary turn: read a printed word, then put a finger on one of its letters.
The recall now asks for the same retrieval by ear:
`[YOU RECALL: say *mulāqāt*, then spell **प्रणाम** aloud letter by letter,
naming its **ण** as you reach it]`. The lesson still assesses the same
atoms (the ण recognition atom included) and stays `drivable: true`.

- The new pointing check in human-language-data
  (`tests/drivable-writing-cues.test.ts`) now fails on a drivable recall that
  says "point to" or "point at".
- Separately, the narration now defers every `[YOU READ: …]` and
  `[YOU LOOK: …]` cue ("[once you have stopped driving — read: …]"), which
  regenerates this track's narration for the 44 chapters that use them
  (chapters 6 to 128). The lessons themselves are unchanged.
- Regenerated: book chapter 51 and its hash, the narration (`.json` and
  `.txt`) and narration hashes, and HI-C51-sweet's `core/lesson-modality`
  owner (source hash only).
