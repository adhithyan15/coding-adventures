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
lesson may be removed or mocked. The test-only lesson parse cache may be used
through the synchronous `loadLessons` route if a measured comparison shows it
avoids the overhead of per-file async reads on Windows. Both lesson routes must
retain their existing source-byte cache keys, parse parity, track ordering, and
symlink/junction refusals. No production loader behavior changes are required.

The test, hook, worker, heap, and coverage budgets remain unchanged. Temporary
timing instrumentation is not part of the committed test or package output.

## Acceptance

- A cold and a warm isolated Windows modality-manifest file pass all existing
  assertions without a timeout override.
- The package's full unchanged Windows coverage suite and final-head Ubuntu CI
  pass; the macOS full-corpus suite must also pass before #18941, #18751, or
  #18947 is closed on budget evidence.
- Any isolated pass is reported as isolated evidence, never as full-suite proof.
