---
category: Repo policy / workflow reminders
---

# A vocabulary chapter can LOWER a track's attained level: the reinforcement criterion needs every atom revisited twice after introduction

Landing the first Spanish A2 vocabulary tranche — 30 new headwords, nothing
removed — dropped Spanish's level-gate **attained** from `A1` to `pre-A1`.
Adding material caused a regression.

The gate named it exactly:

```
criterion: reinforcement
detail: 2 atom(s) at or below A1 are revisited fewer than twice
        (129 etymology hook(s) waived)
shortfall: 2
```

Two of the thirty new atoms were introduced and then practised by only **one**
later lesson:

- `ES-LEX-C432-CASA-05` (*sofá*) — its chapter repaso covered it, its payoff did
  not, because the payoff is a notice about the water being cut off and a sofa
  has no place in one.
- `ES-LEX-C436-GENTE-02` (*grupo*) — the payoff covered it, the repaso's Guided
  Practice block did not, even though that block's text says "all five words".

**The chapter template does not guarantee the second revisit.** Five word
lessons plus a repaso plus a payoff *looks* like two revisits per word, but only
if both the repaso and the payoff actually assess every word. A payoff built
around one realistic scenario usually reaches four of five, and nothing warns
you: `practises` was consistent, every atom resolved, all thirteen gates passed,
and `npm run validate` was 21/21.

Count it before pushing, per new atom, excluding the introducing lesson:

```python
others = {lesson for lesson in practises[atom]} - {introducer[atom]}
assert len(others) >= 2, atom
```

Fix by giving the word a genuine second outing — a real line in the payoff, or a
block that already covers it semantically — not by padding `practises`. Padding
satisfies the count and teaches nobody.

**The general shape is worth keeping.** A criterion can be a *floor*
(vocabulary: more is always better, so adding is safe) or a *ratio/coverage* rule
(reinforcement: adding an item adds an obligation). Adding material is only
monotonically safe against the first kind. Before assuming a purely additive
change cannot regress anything, ask which kind each criterion is.
