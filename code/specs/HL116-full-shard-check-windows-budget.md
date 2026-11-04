# HL116 — Keep the complete shard check inside the Windows test budget

## Status

Proposed child of #18957, which blocks #18751. This changes the schedule and
reuse of independent owner reads, not the set of plans or the existing
30-second test and 85% line-coverage gates.

## Problem

The generic `shard-cli --check` test executes every plan and exceeds its
30-second Windows Vitest budget with coverage. Profiling found the heaviest
repeated work in generated book/narration hash owners, authored curriculum
owners, and chapter capability owners. The same independent sources are read
again by later plan checks. A warm-cache coverage run still took 48 seconds;
this is not a cold-runner anomaly.

## Contract

- Keep `--check` over every `SHARD_PLANS` entry, with the same reconstructed
  bytes, removed-monolith refusals, completeness comparisons, unsafe-key
  rejection, owner-name rules, and independent-source meanings.
- Read generated book and narration hash owner sets once in a bounded batch.
  Pin every directory parent, verify the exact owner set and regular-file
  kinds from that handle, and reject symlinks and Windows reparse points.
  Parse and validate every returned JSON owner with the ordinary validators;
  filenames alone do not establish chapter validity.
- In the full check, validate all curriculum and chapter plans before their
  shared cross-ledger consumers, then reuse the checked identities. A
  single-plan check still loads its independent sources directly. If an
  unplanned authored track exists, fall back to the ordinary loader so new
  tracks cannot disappear from completeness.
- Apply the ordinary chapter-label safety rule to a reused chapter fold.
  Do not replace source checks with generated output, raise timeouts, split or
  narrow the all-plan test, relax coverage, or suppress a failing check.

## Acceptance

- Batch and individual hash readers agree on valid fixtures; malformed,
  dangerous-key, extra-owner, symlink/reparse, and resurrected-monolith
  fixtures fail on the batch path.
- The complete all-plan test passes within its unchanged 30-second budget
  under Windows coverage, including a non-warm run. The full package coverage
  suite still reaches at least 85% lines and all required exact-head CI
  checks finish acceptably.
- TypeScript, Python boundary, canonical shard CLI, and security review pass
  on the final head before this PR becomes ready or enables auto-merge.
