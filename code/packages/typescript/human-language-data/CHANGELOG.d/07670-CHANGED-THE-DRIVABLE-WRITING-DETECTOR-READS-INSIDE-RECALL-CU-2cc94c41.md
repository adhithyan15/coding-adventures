### Changed — the drivable-writing detector reads inside recall cues

- Issue #12070, fifth pass. `tests/drivable-writing-imperatives.ts` judged
  only prose and treated every cue as safe. That holds for `[YOU WRITE: …]`,
  which the narration defers because WRITE is in `MANUAL_CUE_ACTIONS`, but
  not for `[YOU RECALL: …]`: RECALL is a spoken action, so
  `[YOU RECALL: write **ば** — **R1**]` was narrated as "your turn — recall:
  write ば — R1" to a driver. The new `writingRecallCues` reads the content
  of every RECALL cue (re-parsed with `parseDeliveryCue` so the patterns see
  the authored Markdown) and `recallCueAsksForWriting` judges it three ways:
  the prose regex anchored at the start of the content ("write **け** …",
  "draw the **।** …"), the prose clause test (fronted or chained), and a
  writing verb chained on anywhere in the cue ("ask *kitthe?* and write it",
  "answer *kuṭhe?* with *ithe*, then *tithe*, and write **तिथे** once"). The
  third is looser than prose on purpose: a recall cue has no subject, RECALL
  is itself the step verb, and a "?" inside an italic word otherwise ends the
  clause that held the chain.
- Only RECALL is read inside. `[YOU SAY: …]` and `[YOU ANSWER: …]` hold the
  material to be spoken, which in a target language may be about writing.
  Controls are corpus cues: "say the Japanese for to write, then …" (a
  gloss), "point to the sign in **これ** you can already write, and the one
  you cannot" (a description; the chain link must precede the verb), "say
  how much space a written answer needs", "the first letters you wrote".
- `drivableWritingInstructions` (prose plus recall cues) is what
  `tests/drivable-writing-cues.test.ts` now runs over every drivable lesson.
  No new regex: the cue test reuses `BARE_WRITING_IMPERATIVE`, `clausesOf`,
  `opensChainedOrFrontedWriting`, `CHAIN_LINK` and `withoutQuotations`, after
  one whitespace-collapsing pass. Unit tests run `recallCueAsksForWriting`
  on eight ~50,000-character adversarial inputs (space runs, unclosed
  quotation openers, long link chains, many clauses, question marks, stars)
  and `writingRecallCues` on a 50,000-character paragraph of 2,000 cues and
  an unclosed one, asserting answers only.
- The ratchet no longer crashes when no track has debt. Git does not keep an
  empty directory, so deleting the last ledger file (Marwadi's, in the
  previous entry) deleted `tests/drivable-writing-debt/`, and `readdirSync`
  threw ENOENT before any test in the file ran. A missing directory now reads
  as "no debt", like a missing file. The anti-vacuity check
  `offenders.size > 0` asked for debt to exist; it is replaced by one that
  asks the detector to keep firing on the non-drivable lessons, where both
  shapes are legitimate pen work (more than 50 with prose instructions, more
  than 5 with writing recall cues).
- Corpus: 142 RECALL-write cues in 91 drivable lessons — Japanese 104 in 67,
  Marwadi 24 in 11, Gujarati 5 in 5, Marathi 3 in 3, Punjabi 3 in 2, Bengali
  2 in 2, Sanskrit 1 in 1. All are fixed as `[YOU WRITE: … from memory]`,
  keeping each spacing tag, so no ledger is added. Two needed more than the
  verb: MR-R44-directions keeps its spoken answer as a recall cue and gives
  the writing its own list item, and PA-C44-pahila folds the asking into the
  writing cue to keep "Four recalls" at four items. Details are in each
  track's changelog. The 35 non-drivable lessons with the same cue shape are
  untouched.
- Regenerated: the 28 affected book chapters, their narration, the generated
  book and narration hashes, and the 91 `core/lesson-modality` owners
  (source hash only; every lesson is still `drivable: true`).
