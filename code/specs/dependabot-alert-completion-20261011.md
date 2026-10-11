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
both launches, preserving the two-second startup and 15-second case deadlines.
System-wide runtime ACL writes require privileges unrelated to this regression.
