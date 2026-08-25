### Added - the strand dimension, and the three ladders nobody has climbed (HL-C80)

HL09 proved a course can be gentle on one ramp and brutal on another with
nobody noticing, because only the gentle ramp was counted. Spanish measured
178 headwords with every lesson inside the atom budget -- a textbook-perfect
vocabulary ramp -- while the learner still could not say "no", could not say
"I am", and met the entire past tense behind a spine node declaring one
concept.

The fix is not a bigger budget, it is more ladders. `core/spine.json` now
declares eight **strands** -- FUNCTION, GRAMMAR, LEXICON, SOUND, ETYMOLOGY,
CULTURE, IDIOM, TEXT -- and every one of its 33 nodes names exactly one. A new
`strands.ts` measures the distribution, and `report-cli` prints it.

The first snapshot is the reason the model was worth building:

    FUNCTION 14, GRAMMAR 7, LEXICON 2, SOUND 0,
    ETYMOLOGY 0, CULTURE 3, IDIOM 0, TEXT 7

**Three declared ladders have no nodes on them.** ETYMOLOGY is the sharpest:
HL00 calls it "the signature of this curriculum" and 708 lessons carry an
etymology hook, so the content is genuinely there -- as prose an author chose
to write, promised by no node and owed by no chapter. That is the difference
between a commitment and an intention, and it is exactly what a strand model
exists to expose. `summarizeStrands` seeds its counts from the DECLARED strand
list rather than from the nodes present, so an unclimbed ladder reports as a
zero instead of vanishing from the table.

`nodeSizeDefects` makes the HL09 section 1 defect checkable: a node is realized
by one to three chapters, so it may not declare more concepts than a chapter
may introduce. `SPINE-SAY-WHAT-I-DO` declares **42** against a ceiling of 12,
while `SPINE-TALK-ABOUT-PAST` declares one and stands for the entire past tense
of the language. Both cannot be one rung of the same ladder, and that asymmetry
is how a track claimed A2 on fourteen present-tense lessons. HL-C81 splits it;
until then the count is pinned so it cannot grow quietly.

`core/chapter-policy.json` gains the seven HL10 section 2.2 budgets, all
optional so a policy file written before them still loads. The consequential
one is `maxNewGrammarCellsPerLesson: 1` -- a *cell* is one filled slot in one
paradigm (`hablo`), not the six-form table, and Spanish holds roughly 630 verb
cells. `maxRuleStatementsPerLesson: 1` is the info-dump gate, and
`minDownstreamReach: 1` makes "every lesson leads to future lessons"
falsifiable by naming an introduced atom no later lesson ever uses.

Everything here is report-only, per the HL05 precedent: the corpus predates the
model, and a gate that fails on already-recorded debt teaches authors to route
around it rather than pay it down.

