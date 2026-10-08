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

- **Count:** 12 drivable lessons; 13 `[YOU READ: …]` cues and 1 ear-and-voice
  rewrite.
- Script steps: "Then read **এক**." (BN-C20-ekta), "Read on sight: **…**"
  (BN-C113-dhulo), "Read each word whole: **ভাত**, …" (BN-C129-girja), "Read
  without stopping: **…**" (BN-C137-golapi, BN-C97-dabi) and "Read the spelling
  and the romanization again, because they disagree: written **দ্ব**-, said
  *di*-." (BN-C27-ditiyo) become READ cues.
- Reading lessons BN-C40-lines and BN-C40-prothom-path: "Read down once,
  without stopping.", "Again — and notice the mark at the end of each line.",
  "Read it once without stopping …" and "Now read it again and find the three
  joining words." become READ cues; the explanation after each keeps its own
  paragraph.
- Notices BN-C147-bigyapon, BN-C147-notish, BN-C147-taimtebil: the sign, its
  transliteration and meaning move into the READ cue; the comment follows.
- BN-C07-dekha "Read those two romanizations again — they are not the same
  vowel." → "Say those two again — …": the vowels differ in the ear, which is
  the point.
- Left alone: "Look at what is missing" / "Look at what has happened" (the
  idiom for *consider*), "Both were taught once and read once" (a memory of
  reading), "Take the whole book's nouns and say each one near and far" (a
  spoken drill).
