### Added — every track's forward references are pinned, and may only fall

`tests/continuity.test.ts` now holds `FORWARD_REFERENCE_CEILINGS`, one
ceiling per track on `measureContinuity`'s forward references (a lesson that
writes a word a later lesson teaches as its headword). Every track in the
corpus must have a pin, and each track's count must stay at or under it. A
failure lists which lesson used which word, and which lesson owns it. The
German and Italian chapter-1 tests now read their ceilings from that table
(45 → 1 and 34 → 0).

The pins were taken after a pass over fourteen tracks that brought the corpus
total from 1,173 to 1,028: italian 34 → 0, german 29 → 1, kannada 13 → 2,
malayalam 12 → 0, marathi 11 → 4, bengali 10 → 2, telugu 10 → 0, arabic
10 → 0, urdu 9 → 5, tamil 8 → 0, punjabi 7 → 2, russian 5 → 0, sanskrit
3 → 0, persian 2 → 2. Spanish (622), French (156), Portuguese (102), Latin (73)
and Hindi (57) were not worked yet and are pinned where they stand.

No detector rule changed. Each hit was either reworded or recognised through
a rule `forwardReferenceBody` already had:

- **Reworded.** A "Next: **word**" teaser said in English. An example rebuilt
  from words the reader owns. A preview withdrawn. An example moved to the
  lesson that teaches its word. AR-W09-khayr-bikhayr moved after the word
  lesson it writes. Letter lessons re-anchored on words taught before them.
- **Recognised.** A decomposition the lesson already made, such as
  ಹೇ + ಗೆ → ಹೇಗೆ, ل + ا → لا, or இரு + பத்து → இருபது, was split across two
  lines or written with `=`. It is now one `parts + … → whole` line, which is
  the existing decomposition-equation rule.

Every remaining hit in a worked track has one shape: a phrase the track
teaches whole, which a later lesson unpacks into a word. A multi-word headword
owns no single token, so each earlier use of the phrase counts as borrowing
the word. The table's comments name each one.
