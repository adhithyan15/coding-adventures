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

- **Count:** 62 drivable lessons; 14 `[YOU READ: …]` and 1 `[YOU FIND: …]`
  cues, and 52 ear-and-voice rewrites.
- "Close the lessons before this one." (and "Close the three lessons …") opened
  34 drivable recall lessons (ML-R70..ML-R112) → "Leave the lessons before this
  one closed." Judgement call: the instruction is to answer from memory, which
  a driver already does; leaving the book closed asks for nothing.
- Reading: ML-C69-adyathe-vaayana, ML-C69-lines, ML-C69-words (with "Again —
  and this time look at how every one of them ends"), ML-C158-bakkarru,
  ML-C158-treyin, ML-C176-catuka, ML-C110-kattu "Read and say the word for a
  telephone: **ഫോൺ**", ML-C68-seventh "Read it; do not copy it yet" and the
  notices ML-C180 become READ cues; ML-C32-pokuka "— and find it written: **ഞാൻ
  പോകും**" → a FIND cue.
- Ear rewrites where the sentence is the subject, not the script: "**Read that
  sentence twice and look for the ending.**" → "**Say that sentence twice and
  listen for the ending.**" (ML-C102-azhcha); "Put a hand over the question
  word" → "Take the question word out" (ML-C102-ethra-manikku); "Read
  **ചി-കി-ത്-സ** slowly", "Read it aloud and you can hear …", "Read *avan
  vannāl …* again" → "Say …"; "Read the ends and not the fronts", "read the
  fronts" → "Compare …"; "Read the middle of each word" → "Notice …"; "Read
  that against …" → "Set that against …"; "Read the gloss again" → "Think about
  the gloss again"; "**Read what you did there.**" → "**Notice what you did
  there.**"; "Read the three rows in that order", "Read the two **phrases**
  right to left" → "Take …"; "Cover the first character of **അകത്ത്**" → "Take
  the first character off …"; "read it **at**" → "it means **at**"; ML-R94
  "Read straight down the middle:" → "Straight down the middle:".
- Left alone: "Read it in Malayalam order", "Read it literally", "Read it in
  that order and it says" (interpretation), "Look at what that is" and its
  siblings (the idiom), "**Read to the end of it.**" (advice for a long verb
  met on a page), "Keep your pencil down". Nine `ML-R` recall lessons that are
  not drivable keep "Close the lessons before this one".
