## HL-C441-b659132e — Arabic was the last track with reinforcement debt, and the shape of the debt rather than its size set the price

**Status: CLOSED (2026-09-24) — implemented.** Fifteen `type: review` lessons,
arabic 78 thin atoms -> 0. With this, **all twenty-three tracks carry zero pre-A1
reinforcement debt**, which closes the run that began at HL-C435.

### What was measured

    pre-A1 atoms revisited fewer than twice   78
    of those, never revisited at all          68  (87%)
    retrieval slots  (68 x 2) + 10          = 146
    path segments carrying the debt           16
    top three segments                        52 of 78 atoms
    chapters in the track                     45
    lessons in the track                     129 -> 144

### The claim this entry exists to close

HL-C438 and HL-C439 both said arabic "would need more retrieval lessons than the
track has content chapters," and the first draft of HL-C440 repeated it. The
correction was recorded in HL-C440 and is now closed by measurement: arabic has
**45 chapters**, and 146 slots came to **fifteen lessons**. The original figure
rested on a chapter count nobody had checked.

### What actually distinguished arabic

Not the slot count. **The ratio of zero-revisit atoms to thin atoms:**

| track | thin atoms | never revisited | share | slots | lessons |
|---|---|---|---|---|---|
| malayalam | 48 | 1 | 2% | 49 | 4 |
| tamil | 73 | 15 | 21% | 88 | 9 |
| arabic | 78 | 68 | **87%** | 146 | 15 |

An atom at zero revisits costs two lessons; one at one costs one. So a track whose
pre-A1 already has a review layer pays roughly one lesson per ten atoms, and a
track without one pays two. Arabic's pre-A1 had almost none. **That ratio is the
number to measure first on any future track** — before the atom count, and before
the segment spread.

Segment spread still predicted cost the way HL-C440 said it does: three segments
carried 52 of 78, so no new chapter, segment or extension was needed, exactly as
for HL-C438, HL-C439 and HL-C440. Punjabi (HL-C437) remains the counter-example —
forty atoms across seventeen segments, and expensive because of it.

### What the offline gate mirror bought

Hindi (HL-C439) took roughly six trimming passes per lesson against the 300-second
duration ceiling. Tamil (HL-C440) was drafted against an offline mirror of
`estimateLessonDuration` and landed nine for nine on the first pass. Arabic landed
**fifteen for fifteen**, the mirror having first been verified to reproduce the
live estimator exactly on all five of arabic's existing review lessons. Highest
computed duration in the tranche: 279 seconds against a ceiling of 300.

What the mirror did **not** model, and what therefore bit:

- a **four-column table** is refused by the narrator, so the audio learner never
  hears it — one table in the tranche, reshaped to three columns rather than
  bumping the refusal count;
- an interspersed lesson may carry only **one** `## Writing:` segment;
- `curriculum-membership.d` orders are **0-indexed dense**, not 1-indexed;
- the phrase "the other tracks" trips `standalone-book`, because the reader is
  holding one volume.

All four are lessons in `lessons.d/` now. Two of them — the wide table and the
path order — already had entries there and were not read before drafting, which is
the recurring cost of skipping step 7 of CLAUDE.md.

### Still open

- **HL-C435** — the 37-chapter nested-emphasis rendering defect across 13 tracks.
  Gate first, then per-track repair. Untouched by this run.
- The **A1-and-above** reinforcement layer. Everything measured in HL-C435 through
  HL-C441 was scoped to pre-A1 by `level-gate.ts`. The same measurement run without
  that filter is the obvious next question, and nobody has asked it yet.
