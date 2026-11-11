# HL120 — Cold modality source-derived hook budget

## Status

Implementation candidate for #19163, a child of #18941 and #18751. Full-suite
validation remains pending. This is a corpus-read and derivation performance
problem, not a curriculum, output-format, security-policy, or timeout change.

## Evidence

On PR #19160's final head, the unchanged full Windows coverage suite reported
4,423 passing assertions and no failed assertions, but the shared
`modality-manifest.test.ts` `beforeAll` exceeded its 30-second hook budget. A
new clean `origin/main` worktree reproduced that hook failure in isolation;
the immediate warm rerun passed 44/44. Temporary warm-run profiling measured
7.13 seconds for the committed direct-owner read, 13.42 seconds for
`loadLessonsAsync`, and 5.23 seconds to derive the manifest, or 18.65 seconds
for the overlapping hook. A later unchanged full Windows coverage rerun on
the same PR head passed 4,467/4,467 non-skipped tests. The cold failure is
therefore intermittent, but real. The owner read alone is not the whole cost.

On the same Windows source corpus with warm lesson-cache hits, a phase probe
measured 13.30-14.06 seconds for source loading, 7.41 seconds for the
overlapped committed-owner load, and 5.58 seconds for manifest derivation.
The derivation's per-lesson map and ordering accounted for 5.58 seconds;
findings, track rollups, and corpus hashing took under 0.2 seconds together.
Reusing the whole-lesson cue, script, and width scans when there is no
detachable block reduced that derivation phase to 3.30 seconds and the warm
hook from about 19.84 to 16.63 seconds. A cache-cleared, unchanged-timeout
Windows focused run of the manifest, modality, and async-loader suites then
passed 137/137 tests. The original cold hook failure is intermittent, so this
is not yet evidence that full-suite flakiness is eliminated.

## Contract

Profile the cold source read, content-addressed cache validation, parse, and
manifest derivation separately from the direct-owner read before selecting an
optimization. Reduce genuine redundant work or bounded I/O cost. Preserve
the independent source-derived manifest and committed-owner comparison; a
cached or generated manifest cannot stand in for source lessons. Every source
byte must still participate in the cache key on every call, and a source edit
must invalidate the result without relying on mtime. Keep track and lesson
order, source hashes, canonical owner bytes, exact-set and identity checks,
all symlink/junction and dangerous-key refusals, and error behavior.

Do not move work from the hook into a different test merely to split its
budget. Do not increase the 30-second hook, 60-second end-to-end test, worker
count, heap limit, or lower the 85% line-coverage threshold. Avoid unbounded
promises or file opens; started operations must drain before rejection. A
performance win must be demonstrated on the same source and cache state, with
the old and new paths checked for result and failure parity.

## Acceptance

- A deterministic fixture checks source-edit invalidation, output parity,
  ordering, missing/extra owner and unsafe-path refusal on the optimized path.
- Measure cold and warm phase times on Windows, including the full hook and
  the unchanged end-to-end `--check` case; record whether the cold failure is
  reproduced and eliminated rather than citing a warm pass alone.
- The unchanged full Windows coverage suite exits zero on the exact final
  head, with all corpus assertions and the >=85% line gate intact.
- macOS and Ubuntu CI pass on the same head, and a fresh final-source security
  review finds no new path, identity, or resource-exhaustion gap.
- Keep #18941, #18751, and #12732 open if the full-corpus gates remain flaky.
