## Chapters 344, 351 and 352 still file qualities under the numbers node

`HL23` §13 relocated chapters 365, 371, 379 and 380 from `SPINE-COUNT-ONE-TO-FIVE` to the new
`SPINE-DESCRIBE-QUALITIES`. Those four were **wholly adjectival**, so each moved as a one-line
segment retarget — §11.2's cheapest class, no lesson moves, no splits.

Three chapters were deliberately left behind because they are **mixed**, and a mixed chapter needs
a segment split rather than a retarget:

| chapter | genuinely quantitative | quality adjectives |
|---|---|---|
| 344 | `la mitad`, `la docena`, `doble` | — |
| 351 | `cien`, `la cantidad`, `ambos`, `varios`, `el triple` | — |
| 352 | `el conjunto`, `la mayoría`, `el resto` | `entero`, `escaso` |

`entero` ("whole") and `escaso` ("scarce") are the two that read as qualities rather than counts,
and both sit inside `ES-PATH-352-01` beside three quantity nouns. Moving them means cutting that
segment into runs of consecutive lessons sharing a destination, and — per §11.3 — **a segment split
is also an extension split**, since `ES-EXT-352-COUNT` names lessons that would land on both sides.

That is a different job from the retarget, with a real error surface (`validateCurriculum` reports
`uses ... outside ...` and `attached to both ...` when a split cuts across an extension). It is
also low-value on its own: both nodes are A1, so the move is score-neutral and changes nothing a
learner sees. Worth doing when someone is already splitting segments in this track.

`ES-PATH-359-01` (`metro`, `gramo`, `kilo`, `litro`, `peso`) was examined and **left on purpose** —
units of measure belong with counting.
