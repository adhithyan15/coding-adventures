# Complete the Dependabot sweep

The authenticated GitHub alerts API reports 409 open npm alerts in 330 manifests
on 2026-10-11. PR #17185 already carries most compatible lockfile updates. Finish
that PR against current main, then verify GitHub's actual alert state after merge.

## Remaining dependency paths

- Replace Jest in trig, wave, matrix, gradient-descent and loss-functions with
  Vitest 4.1.11. Preserve every numerical test and the optional native matrix
  parity suite. Remove Jest's Istanbul / js-yaml 3 / argparse 1 / sprintf-js path.
- Override Electron packaging's optional global-agent with 4.1.3, whose dependency
  graph removes roarr and sprintf-js. Verify @electron/get's proxy bootstrap API.
- Generated Electron projects must emit the same override and pin shell-quote to
  1.11.0, the patched release published 2026-09-29. This avoids the newer
  concurrently release without bypassing Engram's seven-day resolution floor.
- Resolve http-cache-semantics 4.3.0 under the existing release-age floor.
- Update barcode-layout-1d's lockfile, rerun its conformance tests and record the
  resulting package tree and verified revision in its fixture evidence. Merge
  this PR with a merge commit so the verified revision remains an ancestor.

## Validation and landing

Compare every alert's vulnerable range with its manifest's resolved versions;
audit all tracked npm locks for advisories that GitHub has not yet indexed. Run
the changed test suites, TypeScript builds, generated-lock contract tests and
fixture registry tests. Review registry URLs, integrity hashes, new install
scripts and override compatibility before pushing. Keep required CI checks and
CodeQL green before merging; refresh alerts after GitHub processes the new main.

CI on the previous head failed on human-language-data corpus test timeouts and a
Windows sandbox launch deadline. Reproduce and fix the underlying constraints,
preserving coverage and sandbox enforcement.

The Windows JVM writer must reject NTFS reparse points using lstat file
attributes on supported Python versions, including Python 3.10. Check existing
root ancestors before canonicalization and each output component before writing.
Regressions cover a junction root, missing descendants and existing descendants.
Corpus assertions may share one immutable measurement per file; invalid mock
levels must fail before any corpus read. Preserve live CLI and baseline checks.

The overlapping Windows Node ACL test uses one owned copy of the runtime for
both launches. Use the production five-second startup budget and a five-second
child lifetime for the overlap fixture, matching the Python overlap test, while
retaining its 15-second case deadline and both exit assertions. On run
38163286318 the owned Node fixture took 2.3 seconds and was cancelled at its
test-only two-second attestation limit; the earlier DACL privilege failure was
already fixed. System-wide runtime ACL writes require unrelated privileges.

Load the immutable modality corpus and build its manifest once during module
initialization, as the other real-corpus fixtures do. The same CI run passed
207 human-language test files but exceeded the 30-second modality setup hook.
Keep the actual assertions and CLI checks, with their existing deadlines.

The default-config local modality rerun exposed repeated immutable artifact reads
inside two assertions (122.6 seconds and 97.6 seconds). Load the real committed
manifest once alongside the real lessons, retaining the live `--check` invocation
and all equality checks. Use directory junctions for the directory-link rejection
fixtures on Windows so non-administrator validation exercises the same boundaries.

The remaining live CLI check measured 74.2 seconds after those repeated test reads
were removed. Each CLI invocation currently builds the identical modality manifest
twice: once for validation and once for emitted owner bytes. Build it once and
derive owner bytes from that same manifest. Preserve all live byte comparisons,
language/lesson/narration identity checks, owner reconstruction and write recovery.
After eliminating the duplicate build, the live CLI regression still measured
66.1 seconds against its old 60-second budget. Give this single full-corpus,
direct-owner filesystem integration case an explicit 120-second budget. Ordinary
test and hook limits remain 30 seconds. Do not bypass owner/path validation or
move this live CLI call into unbounded module setup to avoid its deadline.
