---
category: Testing & coverage
---

# Making more lessons core-drivable moves the every-250th sample in drivable-instructions tests onto lessons the sample was never checked against

**What went wrong.** Folding 23 wide tables to three columns made 17 more
lessons drivable at their core. Every `check:*` gate passed. The full
human-language-data suite then failed one test in
`tests/drivable-instructions.test.ts`: "a planted 'Type the word for …' is
asked as 'Say …'", with the result `PA-C34-ji-aaian-nu: gate blind`. That
Punjabi lesson was not touched. The test's `sample` is every 250th
core-drivable lesson, sorted by id. Seventeen new members shifted the stride,
and the sample landed on a lesson whose planting site (its Wrap-up Recall) is a
section the lesson's notice sets aside. The spoken-eyes-or-hands gate rightly
never scans a set-aside section, so a planted "Type" there can never trip it.

**The fix.** The sibling deferral test already skipped set-aside sites. The
Type test now uses the same guard, and asserts that more than 10 sampled
lessons still demonstrate the gate, so the skip cannot hollow it out. The
same change also broke language-ladder's `voicescript.test.ts`, which read
French chapter 1 for a refused table that no longer exists. That test now
reads Spanish chapter 46, whose grid is refused on purpose.

**Do differently.** A change that moves `coreModality` for many lessons
reshuffles every strided sample over core-drivable lessons. Run the full suite,
not only the narration and modality gates. When a strided-sample test fails on
a lesson you did not touch, first check whether the sample moved onto a shape
the test never guarded against, before you suspect the gate.
