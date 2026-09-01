## HL-C200 — The Indic C2 program, and why progress stalled

The directive: bring every Indic track gently to C2, with reading, writing,
speaking and listening intermixed and constantly reviewed. Page count and lesson
count are explicitly not constraints.

### Why nothing shipped for ~120 commits

Honest accounting. Two infrastructure PRs landed and zero lessons. The session
spent itself on measurement and blocker removal while the corpus moved under it.
Both PRs were defensible in isolation and neither was the assignment.

The generalisable failure: **a blocker is only a blocker for the work it
actually blocks.** `script-ductus` was queued as the next "parallelism blocker"
on the strength of its global pins (`keys: 349`, three corpus-wide SHAs). But
`script-ductus` is not a dependency of `human-language-data` at all — only
`language-ladder` consumes it. Lesson authoring never touches it. Checking the
dependency edge before scheduling the fix would have saved a whole PR, and that
check is one grep.

### The parallelism position, measured

A content commit touches only per-language files — `<track>/lessons/`,
`<track>/CHANGELOG.md`, `<track>/book/chapters/`, `<track>/narration/`, plus
`core/lesson-modality/<track>.d/`, `core/generated-book-hashes/<track>.d/`,
`core/generated-narration-hashes/<track>.d/` — and one uniquely-named BACKLOG
shard. Zero shared files. Twelve tracks therefore run with zero contention.

The one real blocker was `grouped-shards.test.ts`, which pinned corpus-wide
totals and was touched by 22 of 40 HL commits. Since it was de-pinned, 23 HL
commits have landed and **none** touched it.

Remaining, and correctly deferred because it does not block authoring:
`script-ductus/tests/stroke-ownership.test.ts` still pins `keys`, `keyHash` and
`nonTamilDataHash` globally. It only bites work that adds ductus stroke data.
Note the precedent for when it does: Tamil is already sharded one file per glyph
under `src/strokes/tamil/` with matching evidence, which is why Tamil is excluded
from `nonTamilDataHash`. Every other script is one monolithic file, and
`devanagari.ts` is shared by hindi, marathi, sanskrit and marwadi.

### Per-track priorities

Two axes. Vocabulary is distance to the 300-headword pre-A1 gate. Writing is the
gloss-first ramp: romanized on first meeting, glyphs taught later one at a time,
interleaved and spaced.

- **Script ladder is the gap** — bengali (39 glyphs shown but never taught, the
  worst in the corpus), urdu (28), telugu (24), punjabi (21), sanskrit (18),
  kannada (18).
- **Glossing is the gap** — hindi (70 load-bearing headwords, by far the worst),
  malayalam (31), marathi (30), gujarati (25), tamil (21).
- **Vocabulary is the gap** — marathi and punjabi (36 of 300), urdu (43),
  gujarati (44), bengali and marwadi (46).
- **Modality is skewed pen-first** — punjabi (49% ear-drivable), gujarati (52%),
  marathi (56%). Writing was front-loaded rather than interleaved; rule B is the
  corrective.
- **marwadi is the reference implementation**: zero closure violations, zero
  never-taught glyphs, zero missing glosses. Keep it that way; regressions there
  are worse than shipping nothing.

### Standing rule for this program

One new headword per lesson, at most ~3 new atoms, five minutes. Split rather
than compress. Never let a learner decode a glyph nobody taught them.
