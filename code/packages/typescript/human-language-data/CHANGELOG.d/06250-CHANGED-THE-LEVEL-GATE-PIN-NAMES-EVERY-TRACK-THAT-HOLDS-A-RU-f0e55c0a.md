### Changed — the level-gate pin names every track that holds a rung, now Spanish and Telugu

`tests/level-gate.test.ts`: `tracksWithAnyLevel` 1 -> 2, because Telugu attains
pre-A1. The anti-spurious sweep over `attainedByLevel` used to derive its single
exception from `spanish.attained`; it now derives the expected per-level counts
from a pinned `{ spanish: "A1", telugu: "pre-A1" }` map and asserts the tracks
holding a rung equal that map exactly, so the next track to climb edits one
entry instead of rewriting the loop. `curriculum-digests/telugu.json` moves for
chapters 92-99 (425 -> 469 lessons).
