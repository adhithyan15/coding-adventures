# HL125 — Give each shard migration test its own check scope

## Problem

Issue #19242 is a child of the Windows full-suite blocker #18751. On merged
main `a3f9ba633677`, the unchanged Vitest coverage suite passed 252 of 253
files; `chapters-shards.test.ts` timed out in 33.218 seconds against its existing
30-second budget. That test, `curriculum-shards.test.ts`, and `shard-cli.test.ts`
each invoke the same complete `runShardCli(["--check"])` over every plan. This
repeats the largest owner reads three times under whole-suite contention.

## Contract

- `shard-cli.test.ts` remains the canonical complete all-plan CLI check at its
  unchanged 30-second test budget. The package's `check:shards` command and
  repository verification and Books workflow continue to run the full check.
- The chapters and curriculum migration suites check every real plan in their
  respective families with `--check <path>`. Keep their existing real-corpus
  membership, layout, byte/order, and negative-fixture assertions. Name each
  plan in its test result so one broken owner is immediately identifiable.
- A scoped check must use the ordinary production single-plan path. It may
  neither mock owner reads nor substitute generated summaries. The canonical
  all-plan test continues to exercise cross-ledger completeness and batch
  reuse, which a single-plan check cannot replace.
- Do not change source validation, shard bytes, the 30-second timeout, worker
  count, coverage threshold, or security refusals to obtain a passing run.

## Evidence and acceptance

On the same main source, read-only uninstrumented single-process measurements
of all 23 chapter plans and all 23 curriculum plans took 6.60 and 9.45 seconds,
respectively. These are directional, not a substitute for coverage validation.
The three focused migration/CLI files must pass with unchanged coverage and
budgets; the complete Windows suite must pass under its ordinary settings on
the final head. Required Ubuntu checks and a full macOS corpus run must also
reach terminal acceptable results before merge. If the canonical all-plan case
still crosses 30 seconds, optimize its real work rather than moving its gate.
