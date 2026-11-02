# HL108 — Per-track level-progress ownership

## Status

Proposed for issue [#18519](https://github.com/adhithyan15/coding-adventures/issues/18519).

## Problem

The corpus already stores each track's attained CEFR level in its own JSON
owner. That first migration removed one shared scalar map, but it did not remove
the real authoring collision. A language advancement still edits two shared
tests:

- `tests/level-gate.test.ts` accumulates track-specific progression history.
- `tests/levels.test.ts` maintains exact shared lists of tracks by reach.

On the fresh main branch after Punjabi Chapter 207, 82 of the latest 100
human-language commits touched the former and 38 touched the latter. Ten
independent language authors therefore still serialize on the same two files.

## Ownership contract

Every registered track has exactly one regular-file owner at
`tests/level-gate-attainment/<track>.json`. The stable filename is the track
identity. Each owner contains exactly these fields:

```json
{
  "attained": "A2",
  "reach": "B1",
  "evidence": "A concise current-state explanation grounded in owned chapters."
}
```

- `attained` is the highest level whose complete gate criteria pass, or `null`.
- `reach` is the highest spine level touched by the track, or `null`.
- `evidence` is a non-empty, bounded explanation of the current pin. It records
  why the two level values are credible without turning a shared test into a
  chronological project diary.

The owner reader establishes the exact expected filename set from the
independent corpus track registry before it opens owner bytes. It rejects an
empty set, unsafe or case-fold-colliding names, directories, symlinks, extra or
missing owners, unknown fields, unsafe CEFR values, and empty or unbounded
evidence.

## Generic consumers

The shared level-gate and levels tests may contain only cross-track invariants.
They load the direct owners and assert, for every registered track:

1. the measured attained level equals the owner pin;
2. the measured reach equals the owner pin;
3. reach never ranks below attainment; and
4. summary counts and per-level sets are derived from the owners rather than a
   second handwritten track list.

A mismatch diagnostic names the track, field, expected value, and actual value.
Synthetic negative tests prove malformed owners and mismatches fail closed.

## Anti-regression boundary

A source-shape regression rejects restoration of track-specific progression
lists or advancement history in the shared tests. Future progression changes
update only the advancing track's owner plus its direct content and generated
shards.

## Validation

- TypeScript build and typecheck.
- `level-gate.test.ts`, `levels.test.ts`, and the new owner regression.
- Full `human-language-data` test suite.
- Book/progress/gentle-snapshot/modality/narration/shard/document-shard checks.
- Exact-head independent security and integrity review before push.

## Non-goals

- Changing any track's measured CEFR attainment or reach.
- Weakening vocabulary, writing, reinforcement, lesson-budget, or spine gates.
- Generating the owners from the report under test; they remain independent
  regression pins.
- Claiming that touching a level is equivalent to attaining it.
