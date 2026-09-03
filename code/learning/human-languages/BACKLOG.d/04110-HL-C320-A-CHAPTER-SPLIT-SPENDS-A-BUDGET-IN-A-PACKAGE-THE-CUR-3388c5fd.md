## HL-C320 — a chapter split spends a budget in a package the curriculum knows nothing about

German chapter 13 became three chapters. Every gate in `human-language-data` was
green, the German book compiled with zero missing glyphs and zero bad boxes, and
CI failed anyway — in the Language Ladder:

    bundle check: the size backstop split 2 band(s) beyond the 1 allowed:
      lessons-german-C5- into 2, lessons-spanish-C5- into 2.
    Reduce LESSON_BAND_CHAPTERS rather than raising this number.

**Lessons band by their own id number, not by the book chapter they are assigned
to.** The migration keeps lesson ids stable on purpose — that rule is what makes
a renumber safe — so all 27 new lessons carry the `GE-C09-` prefix and all 27
land in the band for ids 5–9. The band went past the bundler's 256 kB backstop,
the chunk was split, and the gate allows exactly one such split corpus-wide.

Three things worth carrying.

**1. The gate is the package's BUILD file, not its test suite.** The ladder's CI
job runs `bash BUILD` — `typecheck`, `build`, `check:bundle`, `vitest run`. A
verification routine that runs `npx vitest run` in the ladder reaches the last of
those four and nothing else. Every chapter migration up to this one passed
because no earlier one added enough lessons to one id prefix.

**2. Id stability and bundle banding pull against each other.** Keeping ids where
they are is right — it is what stops a renumber from rewriting the corpus — but
it means a split concentrates its new lessons into a single band rather than
spreading them the way the book chapters do. The bigger the split, the harder it
pushes one band. This will recur on any track whose migration adds ~25+ lessons
under one prefix, and it is invisible from inside `human-language-data`.

**3. Project a size knob forward before moving it.** Width 4 cleared the failure
that day: 451 batches, zero splits, largest batch 211 kB. It was still the wrong
answer, because German has four hand-written chapters left, all sized as splits,
arriving with `GE-C10-` through `GE-C13-` ids — which project band C12 back onto
the backstop by the last of them at the measured ~3.1 kB per lesson. Width 3
takes the same four to about 176 kB and 171 kB, a third of the backstop unused,
so the change was made once rather than once per tranche and `BAND_SPLIT_SLACK`
fell from 1 to a real 0.

**And re-measure immediately before pushing.** The band contents are corpus-wide
and contended: Punjabi grew to chapter 43 in the merge taken between the fix and
the push, moving the count 589 → 591 batches. The counters in this programme have
behaved that way all along; this one does too.
