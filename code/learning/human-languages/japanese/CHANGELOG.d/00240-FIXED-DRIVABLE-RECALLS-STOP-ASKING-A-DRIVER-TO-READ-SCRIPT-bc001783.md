## Fixed — drivable recalls stop asking a driver to read script

`[YOU RECALL: …]` is a spoken cue action, so the narration reads a recall to a
driver as an ordinary turn. JA-R14-one-to-five is drivable, and its warm-up
held `[YOU RECALL: read **さようなら** off the page — **R4**, eighty lessons
back]`: a driver heard "recall: read さようなら off the page". The cue was only
a reading, so it is now `[YOU READ: **さようなら** off the page — **R4**,
eighty lessons back]`. READ is a manual cue action, so the narration says
"once you have stopped driving — read: …", and the book prints "*Read it:* …".
The spacing tag stays on the cue that now does the recalling, as the writing
recalls kept theirs on WRITE. The lesson stays `drivable: true`.

- The new reading check in human-language-data
  (`tests/drivable-writing-cues.test.ts`) fails on any drivable recall that
  asks for printed script to be read ("read **…**").
- Judgement call, left as they are: the chapter 142 sign recalls
  (`[YOU RECALL: read *reception* on a sign — **R1**, one lesson back]`,
  `read *open*, then *not yet open*, and say which one lets you in`, and their
  siblings in JA-C142-chuui, -eigyouchuu, -junbichuu, -kinen, -uketsuke and the
  two JA-R142 reviews; 11 cues). Their object is an italic English meaning,
  not printed script: nothing is on the page, and the learner retrieves the
  sign's word from memory and says it, which a driver can do. The check
  agrees, and lists one of them as a control. JA-C27-hanasu and JA-C27-kaku's
  "say the Japanese for to read" recalls are glosses and are untouched too.
- Regenerated: book chapter 14 and its hash, its narration (`.json` and
  `.txt`) and narration hash, and JA-R14-one-to-five's `core/lesson-modality`
  owner (source hash only).

