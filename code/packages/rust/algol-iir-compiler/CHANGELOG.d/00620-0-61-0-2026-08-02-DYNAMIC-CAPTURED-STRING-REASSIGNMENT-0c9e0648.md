## 0.61.0 — 2026-08-02 — dynamic captured-string reassignment

Fixes bare-variable detection so a one-argument procedure call is not mistaken
for its actual. Regression coverage now proves that a captured string can be
overwritten by branch-selected string procedure results and passed onward as a
typed string formal.

