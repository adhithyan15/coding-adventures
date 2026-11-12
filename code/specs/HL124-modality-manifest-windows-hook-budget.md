# HL124 — Keep the modality manifest's real-corpus hook inside the Windows budget

## Status

Implementation child of #18941 and #18751. On merged-source main
`85edaed986bc`, the isolated Windows modality-manifest file can fail its
unchanged 30-second `beforeAll` budget before any of its 44 assertions run.
The hook overlaps the independent committed-owner read with an async lesson
read and then derives the manifest. A warm diagnostic measured the owner read
at 6.3 seconds, the lesson read at 12.3 seconds, and the completed derivation
at 15.6 seconds; the cold run timed out. The same source can therefore pass
alone when warm while failing under cold or whole-suite disk load.

## Contract

The hook must still derive a manifest from every real lesson and independently
read every committed modality owner through the anchored, guarded route. Its
later assertions must compare the same complete reports; no owner, track, or
lesson may be removed or mocked. The test-only lesson parse cache and the
pinned-owner reader must retain their source-byte identity, parse parity,
track ordering, and symlink/junction refusals. No production loader behavior
changes are required unless profiling proves the change preserves them.

A measured synchronous-loader substitution is **not** the fix: on the same
Windows host, `loadLessons()` alone took 20.9 seconds and the pure derivation
finished at 26.2 seconds, leaving less than four seconds for the independent
owner read. Starting the child first was worse: synchronous Node work stopped
draining its stdout and triggered the child's existing 20-second deadline.
The original test source was restored after this diagnostic.

The next bounded change keeps the async lesson loader but moves the independent
committed-owner read from the shared hook to the first assertion that actually
uses it. That read remains a single lazy promise for both consumers, with the
existing 20-second deadline on each anchored track child; neither consumer
may use a mock or a partial owner set. The first consumer's 30-second test
budget still measures the entire multi-track read. The hook's 30-second budget
measures only the complete source derivation, without two readers contending over cold
disk I/O. The committed-marker case retains its own 30-second test budget,
and the end-to-end `--check` case retains its 60-second budget.

The test, hook, worker, heap, and coverage budgets remain unchanged. Temporary
timing instrumentation is not part of the committed test or package output.

## Acceptance

- A cold and a warm isolated Windows modality-manifest file pass all existing
  assertions without a timeout override.
- The package's full unchanged Windows coverage suite and final-head Ubuntu CI
  pass; the macOS full-corpus suite must also pass before #18941, #18751, or
  #18947 is closed on budget evidence.
- Any isolated pass is reported as isolated evidence, never as full-suite proof.
