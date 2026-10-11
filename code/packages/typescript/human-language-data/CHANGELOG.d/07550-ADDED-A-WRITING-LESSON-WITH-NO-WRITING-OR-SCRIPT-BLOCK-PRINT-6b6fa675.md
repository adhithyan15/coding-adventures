### Added — a writing lesson with no Writing or Script block prints its strip in its modelled practice

- `figure-targets.ts`: a writing lesson with no `## Writing` or `## Script`
  block was never a filmstrip candidate, even when its headword was fully
  cited. It now falls back to its first block whose writing stage
  (`<!-- hl-writing-stage: … -->`) shows the learner a model:
  `observe-trace`, `guided-copy` or `delayed-copy`
  (`MODELLED_WRITING_STAGES`, `modelledPracticeBlockIndex`). Dictation,
  composition and timed stages, and blocks that declare no stage, never
  take a strip, whatever their title says, so a lesson built on writing
  without a model (ZH-R17-writing-five, a dictation; PA-W09-date-select, a
  "decide before writing" selector card) keeps printing none.
- New `stripBlockIndex` (Writing, else Script, else modelled practice) is
  what every candidate function and `filmstripBlockIndex` use.
  `letterBlockIndex` keeps its meaning (Writing, else Script), and
  `letter-anchoring.ts` now checks it before `writingLetterOf`, so a copy
  lesson that gains a strip is not counted as a second letter lesson for a
  letter its -observe sibling already teaches. The anchoring report is
  unchanged (982 letter lessons) and no ceiling moves.
- 25 lessons gain a strip, none lose or move one: chinese 14 (ZH-W16 to
  ZH-W19 -guided and -delayed for 汉 语 国 文 看 书 吗), gujarati 10
  (GU-C20-ghar, -mandir, GU-C21-haath, -paisa, GU-C22-shaalaa, -shahar,
  GU-C23-dukaan, -gaam, GU-W20-gha, GU-W21-ai-matra), hindi 1
  (HI-W01-na-ma, the list न, म). Target-count pins: chinese 58 -> 72,
  gujarati 63 -> 73, hindi 57 -> 58. The tallest new strip is શાળા, about
  1,070 units, under the 1,801 already printed.
- Prose moved to agree with the new strips: HI-W01-na-ma numbers म's four
  movements as its strip draws them, and five Chinese delayed-copy lessons
  (语 国 文 看 书) look at the model and cover it at the top of Guided
  Practice, under the strip, instead of in the Warm-up before it.
- Regenerated: 25 figures and their hash owners, book chapters (chinese 16
  to 19, gujarati 24 to 27, hindi 1) and their hashes, narration (chinese
  ch16 to ch18, hindi ch01) and six modality owners.
- Tests: `a-strip-in-modelled-practice` covers each stage, the stage-less
  block, the first-modelled-block choice, the Writing/Script precedence and
  the anchoring boundary, and pins the 25 real lessons and five cited
  lessons deliberately left without a strip; the real-corpus case pins the
  new Gujarati words; a letter-anchoring case keeps copy lessons out of the
  measure.
