---
category: Testing & coverage
---

# A rule that keeps strips out of no-model blocks must also guard the Writing block, not only the fallback

**What went wrong.** The modelled-practice fallback in
`human-language-data/src/figure-targets.ts` was written so that a dictation
or composition block never takes a stroke-order strip, because a strip there
hands the learner the answer. The rule was applied only to lessons with no
`## Writing` or `## Script` block. Lessons whose Writing block was itself the
dictation ("Writing — short dictation", `hl-writing-stage:
dictation-transcription`) kept the old behaviour and printed the strip at
the top of that block, above the cue: 37 of 817 strips, in 14 tracks
(ES-W00-hola-dictation drew "How hola is written" above "Hear: OH-la").
Every test passed, because the tests for the rule built only Warm-up /
Guided Practice / Wrap-up fixtures.

**Fix.** `stripBlockIndex` skips any block that declares a no-model stage,
wherever it looks (Writing, Script, practice). The app's
`filmstripSectionIndex` does the same. A real-corpus test asserts that no
resolved strip lands in a no-model block.

**Do differently.** When a rule exists to keep something out of a kind of
block, test it against the corpus by that property ("no strip lands in a
block whose stage shows no model"), not only against the new code path's
fixtures. Enumerate where every existing output lands before calling the
rule done: a one-line script over `resolvedFigureTargets` and
`filmstripBlockIndex`, grouped by the landing block's `writingStage`, found
all 37 at once.
