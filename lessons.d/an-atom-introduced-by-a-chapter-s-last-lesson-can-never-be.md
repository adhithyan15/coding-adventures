---
category: Repo policy / workflow reminders
---

# An atom introduced by a chapter's last lesson can never be revisited

When a chapter sits at the end of the corpus, **the lesson that introduces an
atom and the lesson that pays it off cannot be the same lesson**. Nothing comes
after it to do the revisiting, so `atomsNeverRevisited` rises by one for every
atom the final lesson introduces — a regression that appears only in the
snapshot diff and that every gate passes.

Chapter 105 of the Hindi track hit this with a single writing lesson that both
taught the shape of a written message and had the reader produce one. Splitting
it in two fixed the metric and was better teaching anyway: one lesson shows the
shape with a model on the page, the next produces the whole thing from a cue
with no model. That is the `HI-C96` pattern — a review, then a synthesis with an
empty `introduces` list — and it exists for this reason.

**The same split also cured a duration violation**, which is the useful
generalisation: the lesson computed 301 effective seconds against a 300-second
ceiling. A lesson doing two jobs tends to be over budget *and* terminal at the
same time, because both symptoms come from the same cause.

**Check for it before generating.** If the last lesson of a new chapter has a
non-empty `introduces.knowledge`, it is going to move `atomsNeverRevisited`. The
fix is structural, not cosmetic: move the introduction one lesson earlier and
let the last lesson practise.
