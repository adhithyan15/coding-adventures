---
category: Testing & coverage
---

# A test that holds two copies of a rule equal by comparing their constants does not hold the rule equal

The printed books place a lesson's stroke-order filmstrip with
`stripBlockIndex` (human-language-data `figure-targets.ts`): the first Writing
block, else the first Script block, else the first modelled-practice block.
The language-ladder app kept its own copy, `filmstripSectionIndex`, and a
comment said "A test holds this list equal to the book's". The test compared
`MODELLED_WRITING_STAGES` against the book's set, and nothing else. The STEPS
had drifted: the app took the first Writing OR Script section, whichever came
first. In 435 of the 795 strip lessons a Script section sits above the Writing
one, so the app showed the strip in a different section than the book. The
constant test stayed green the whole time.

Comparing the constants two copies share shows only that the constants match.
It says nothing about the control flow around them, and that is where this
drift happened.

The fix was to stop having two copies. The rule moved to an import-free module
(`strip-placement.ts`), because `figure-targets.ts` imports `node:path`, which
a browser bundle cannot take. The app now runs that function on the book's own
parse of the body and maps the chosen block to its section. A corpus test then
checks the one piece that is still app-specific, the block-to-section mapping,
for every lesson with a strip.

What to do differently:

- When a program needs the same decision another package makes, import the
  function. If its module drags in something the consumer cannot load, split
  out a pure module instead of copying the rule.
- If a copy is truly unavoidable, test it the way the other lesson in this
  directory says to test a mirrored list: run both implementations over the
  real inputs (here, the whole lesson corpus) and compare their outputs. Do
  not compare the constants they share.
- Treat a comment saying "the same rule as X, held equal by a test" as a claim
  to verify. Read the test and check what it actually compares.
