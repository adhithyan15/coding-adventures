# HL113 — Per-track reading-reach tests load their own track

## Status

Specified for issue #18751, a child of the corpus-suite performance backlog
#12732. No lesson, task inventory, report algorithm, timeout, worker count, or
coverage threshold changes.

## Problem

Thirteen assertions in twelve task-shape test files inspect one reading-reach
row for one language and level, but each calls `loadLessons()` for the entire
human-languages corpus. In a full
parallel Windows suite, several of those cases exceed their existing budgets
while the same cases pass alone. Each call also holds parsed lessons from all
other tracks even though the asserted row reads none of them.

## Contract

`measureReadingReach` first computes the longest comprehension passage keyed by
language. Each inventory row then reads only the entry for its own language.
Therefore, for a test asserting the row for track `T`, passing
`loadTrackLessons(T)` instead of `loadLessons()` must produce the same row.

The thirteen assertions continue to load the committed task-shape inventories
and to assert their existing status, measurable-part counts, and reach values. They
read every committed lesson of the asserted track through the same guarded
loader and content-addressed test cache. A focused two-track regression test
pins that an unrelated track cannot affect another track's row.

This narrows only the input each per-track assertion needs. Corpus-wide gates
and their complete lesson walk remain elsewhere in the package. No timing or
memory gate is raised to make a check pass.

## Verification

- Compare the narrowed and whole-corpus rows on real committed data before
  publishing this change.
- Run all affected task-shape test files at their default budgets, the package
  type-check, and `git diff --check`.
- Run the full unweakened Windows suite on the exact final source and report
  any remaining failures honestly. Keep #18751 open until that suite passes;
  #18852 separately tracks the remaining macOS full-suite timeouts.
