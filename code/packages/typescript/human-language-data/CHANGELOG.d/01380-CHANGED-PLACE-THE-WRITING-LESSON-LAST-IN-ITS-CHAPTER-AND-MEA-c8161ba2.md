### Changed - place the writing lesson LAST in its chapter, and measure why

The lesson was first written at sequence 1145, between `TA-C38-udambu` and
`TA-C38-sugam`. Measuring that placement against the alternative changed it, and
both effects are worth recording because neither is visible in a total:

- Hands-free start. `chapter-modalities.tex` for chapter 38 went from "all 3
  lessons" to "first 1 of 4": a `pen` lesson in position 2 truncates the core
  drivable prefix, and two speaking lessons lost hands-free reachability.
  Placed last it reads "first 3 of 4" and nothing is lost. The manifest summary
  cannot see this — its per-chapter `drivablePrefix` for chapter 38 is 0 either
  way — so no pinned test would have caught it.
- Reinforcement distance. At index 127 the lesson sits 6 lessons after `TA-W18`
  and 10 after `TA-W17`, both inside R2's 5-15 span. At index 125 it sat 4 after
  `TA-W18` — past R1's 1-3 and short of R2's 5, in the dead zone between them.
  The same lesson practising the same atoms therefore rescues three atoms from
  R2 rather than one.
- Placing a writing lesson last in its chapter is also the corpus's dominant
  pattern: eleven chapters already do it. The gap from `TA-W18` becomes five
  speaking lessons rather than three, which the strand already varies (existing
  gaps include six and nine).

Being last, the lesson carries no `Next:` line. That is not a special terminal
convention — 84 of the Tamil track's 128 lessons carry no `Next:` line — it is
simply that there is no successor left to name. `TA-C38-vidai`, which the move
displaces from last position, has no `Next:` line either — that much predates
this change — but it now has a successor it could name and does not, and that
much is created by this change. It is left alone: of the 84, all but TA-W19 have
a successor, and only a handful gesture at it at all — `TA-C01-practice` and
`TA-C02-practice` with a "Next chapter:" teaser, `TA-C04-po` in running prose.
So adding a teaser to `TA-C38-vidai` would be an isolated exception rather than
a convention.

