## HL-C321 — the stage-direction leak, re-measured, and two numbers that do not reconcile

HL-C317 recorded the leaked-stage-direction debt as **192 occurrences across 61
chapter files in 12 tracks**. That figure is stale: #14198 has since fixed a
large part of it. This entry supersedes the number and, more usefully, records
that two attempts to restate it disagree.

**Measured here**, on a tree containing #14198, with the same expression that
produced the original 192 — `{[}` followed by `YOU SAY`, `PAUSE`, `REPEAT` or
`YOU HEAR`, over every `*/book/chapters/*.tex`:

| marker | occurrences | files | tracks |
|---|---|---|---|
| `[PAUSE Ns]` | 56 | 10 | 3 |
| `[YOU SAY: …]` | 43 | 19 | 6 |
| `[REPEAT xN]` | 30 | 18 | 4 |
| `[YOU HEAR: …]` | 5 | 4 | 1 |
| **total** | **134** | **50** | **10** |

Tracks still affected: chinese, gujarati, hindi, japanese, malayalam, marathi,
marwadi, punjabi, spanish, tamil.

**The disagreement, stated rather than smoothed over.** A separate restatement
gave "35 across 5 tracks". 134 is what the original expression yields now, and it
agrees exactly with that same restatement's own arithmetic — 192 fixed down by 58
is 134. The 35 does not reproduce under any marker subset: the closest single
marker is `REPEAT` at 30 across 4 tracks, and no combination lands on 35/5.

So one of the two figures comes from a narrower expression than the one HL-C317
used, and the narrower expression was not written down. **That is the actual
finding.** A debt counter is only comparable across time if the expression that
produces it travels with it, and this one did not — which is why restating it
twice produced two answers and no way to tell which question each was answering.

The expression is recorded above so the next restatement can be checked against
this one rather than argued about.

**Unchanged from HL-C317:** the cause, and why this is filed rather than swept.
The book generator strips these markers inside a Guided Practice bullet list and
nowhere else, so a stage direction in prose, a warm-up, or any other list reaches
the page verbatim. Fixing the generator would silently rewrite fifty generated
chapters across ten tracks that other agents are authoring; rewriting the 134
source lines is the right answer per lesson but is 134 authoring decisions in
someone else's material.
