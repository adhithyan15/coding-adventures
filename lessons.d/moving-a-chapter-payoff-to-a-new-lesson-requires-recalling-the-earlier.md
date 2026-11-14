---
category: Testing & coverage
---

# Moving a chapter payoff to a new lesson requires recalling the earlier atoms

The Kannada A1 spatial-relator tranche added two lessons after chapter 110's
five place-word lessons and moved the chapter payoff to the new final lesson.
That lesson initially assessed only its three new atoms. The chapter gate then
found Kannada no longer clean: the payoff covered 3 of 8 chapter atoms.

The correction was teaching, not bookkeeping. The final lesson now asks for
market, beach, garden, corner, and turn before the new named-reference
phrases; its block directive and chapter payoff both assess all eight atoms.
Whenever a new final lesson takes over a chapter payoff, count every atom the
chapter introduced before it, make the learner retrieve them, and run
`tests/chapters.test.ts` before generating the final artifacts.
