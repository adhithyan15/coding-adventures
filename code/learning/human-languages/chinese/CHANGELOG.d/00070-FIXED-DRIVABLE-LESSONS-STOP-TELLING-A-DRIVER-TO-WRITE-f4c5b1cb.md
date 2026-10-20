## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 57 lessons in chapters 12-28 and their review
lessons `drivable: true`, but each still asked for writing in bare prose
("Copy **学** once.", "Say **son**, then write 儿 and 子 separately.",
"**Write.** 的 — …"). Narration reads bare prose unhedged, so the audio edition
told a driver to write (issue #12070). Each writing task is now a
`[YOU WRITE: …]` cue, as #16893 did for chapters 3-6: the narration defers it
("[once you have stopped driving — write: …]") and the book prints it as
"*Write it:* …". The cue does not create a writing block, so every lesson
stays drivable.

- **Lessons:** ZH-C12-erzi, -jia, -nu, -practice, -ren; ZH-C13-da, -practice,
  -xue, -xuesheng; ZH-C14-practice, -xiao; ZH-C15-practice, -tongxue, -zhong,
  -zhongxuesheng; ZH-C16-hanyu, -hanzi, -practice; ZH-C17-zhongguo,
  -zhongwen; ZH-C18-kanjian, -shu; ZH-C19-haobuhao, -ma, -nihaoma, -practice,
  -shima; ZH-C20-ba, -jiu, -liu, -practice, -qi, -shi; ZH-C21-bai, -practice;
  ZH-C22/C23/C24/C25/C26/C28-practice; ZH-R12-family-six,
  ZH-R13-school-six, ZH-R14-primary-three, ZH-R15-school-map-1/2/3/4,
  ZH-R16-identity-1, ZH-R17-character-language-r2,
  ZH-R17-country-language-r1, ZH-R18-book-reading-r1,
  ZH-R18-looking-three-r1, ZH-R18-writing-five, ZH-R19-ma-three-r2,
  ZH-R19-reading-three, ZH-R21-close.
- A spoken half stays prose ("Say **son**. [YOU WRITE: 儿 and 子
  separately]"). A sentence that followed the old instruction and is not
  about the writing ("Then say seven.", "Repair only the missed item.") is
  now its own paragraph. Advice about the writing itself ("keep the crossing
  near the centre") goes inside the cue after a dash.
- "Copy **X** once." becomes `[YOU WRITE: one copy of **X** — …]`, so the
  narrated cue still says the learner is copying a model.
- The four-skill blocks of ZH-C22, C23, C24, C25, C26 and C28-practice
  labelled their writing line `**Write.**`. That label was itself the bare
  imperative, so the line is now just the cue; the book prints it as
  "*Write it:* …", which keeps the Listen / Speak / Read / Write order. Where
  the line held two characters, a semicolon now separates them inside the cue.
- ZH-R17-character-language-r2: "Write **请** and point to its 青 sound
  component" points at the character just written, so the pointing is inside
  the cue.
- ZH-R19-ma-three-r2: "then write it. Before you write, decide which of the
  two question patterns it uses" now asks for the decision first and ends with
  the cue, so the order is the same and nothing tells a driver to write.
- ZH-R21-close: each numbered step is one cue, with its spoken half and the
  stroke-count answer inside it, so the list keeps its six items.
- Seventeen more writing tasks in these lessons used an "and write" clause
  ("Say, read, and write each.", "Cover it, wait five seconds, and write
  it.", "4. **Writing:** hear all six and write them without a model."),
  which the detector does not match. They are cues too.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).

