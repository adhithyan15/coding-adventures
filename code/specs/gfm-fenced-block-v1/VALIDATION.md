# GFM fenced-block contract validation

The implementation source at `0bf46aac85cc62de387fe7f297dff4095feec3b9`
was rebased onto main `ddcf1f81c30710201a3fdfedd4de85c73ef5ae21` on
October 24, 2026. This receipt covers the bounded TE04 fenced-node contract,
not complete GFM or all-language qualification.

## Passing checks

- All 15 changed TypeScript package BUILD fronts pass, executing every command
  in an independent native shell from the package directory. Dependency installs,
  real build scripts where present, and production coverage fronts ran.
- The 14 changed runtime packages retain full source coverage include globs and
  report 83.17%–100% statements. The AST package is type-only and receives no
  executable coverage credit. Library coverage targets above these values remain
  improvement goals; no excluded source or lowered threshold was introduced.
- Parser: 743 tests, 89.07% statements / 86.96% branches, including the unchanged
  652 literal CommonMark cases. Facade: 694 tests, 88.02% statements. Editor:
  54 tests, 98.76% statements / 91.27% branches. HTML: 46 tests, 98.59% statements.
- Go: complete module tests, go vet, gofmt, and executable build pass. Aggregate
  statement coverage is 84.6%; root CLI package remains 62.5%. The modified
  selector function is 98.8%, with all added fixture-routing blocks exercised.
- Security rereview finds no actionable issues; all 23 changed production/fixture
  files are byte-identical to the earlier complete corrected-diff review.
- The collision-checked inventory finds 1,533 implementation identities and
  4,815 package slots across 15 established lanes; zero collisions/unknown buckets.
  OCaml remains emerging. Ten newly merged Rust roots are registered for
  documentary classification, without native or exception credit.

## Downstream closure limits

All 118 prerequisite packages were attempted, including all 74 affected consumers:
105 targets passed and 13 failed. These failures remain failures, not qualification.

Seven targets have compiler diagnostics reproduced without this GFM change:
AsciiDoc parser/facade/demo, layout-block, forme-doc-site-emitter, forme-doc-demo,
and markdown-canvas-demo. A TypeScript compiler-host comparison restores all
21 changed production TS/TSX modules from the base in memory while keeping the
same installed dependencies and package cwd. Earlier baseline comparisons match
5 AsciiDoc parser errors, 33 layout errors, and the imported node:crypto type error.
The additional consumer comparison against main `887332c1f270ea5b54243b652c02e26957055274`
matches all 91 doc-demo and all 33 canvas-demo diagnostics, with zero new errors.
This is source-baseline evidence, not an independently clean baseline installation.

Four targets (plugin-host, installer-core, sandbox-core, blog) fail at Windows
symlink fixture creation with EPERM before the relevant production assertions.
The CLI also encounters symlink permissions and native sandbox readiness failures.
Its Python PATH omission was corrected and the tests rerun: 73 pass, 3 fail,
3 skip. Loading the installed Visual C++ development environment makes the Windows
sandbox build pass; its tests report 17 pass and 2 fail on test/readiness deadlines.
No tests were skipped or weakened to hide these results. OS authorization or
sandbox policy was not changed. WSL is unavailable on this host.

The landing-page build/tests, journal-app build/tests, desktop-shell build and
49 tests, CommonMark demo, and most other transitive consumers pass. The full
118-target structured result, per-step exit codes, logs and coverage outputs are
retained in the local task evidence directory. Subsequent final-head CI is
required; no native consumer-complete or all-language claim follows from this PR.

The final rebase onto `ec0d8fbe3dd2bc1bea3d22d21647a1472d8f29ba` adds
only unrelated ALGOL and OCSP main changes. All 23 reviewed production/fixture
files retain exactly the validated bytes; the additional structural OCSP root
is classified under the pending OCSP owner. Final inventory: zero collisions.
