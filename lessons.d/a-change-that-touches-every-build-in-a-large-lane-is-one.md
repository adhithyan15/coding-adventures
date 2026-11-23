---
category: BUILD files & dependency management
---

# A change that touches every BUILD in a large lane is one pull-request job that exceeds the macOS runner's six-hour cap

### What happened

Pinning the rust lane (KILN00 M0, KILN03 §7.1) means adding `--locked` to every
cargo invocation in `code/packages/rust/*/BUILD` and `BUILD_windows`: 3,566
lines across 2,134 files. That is not "a lot of files" for CI purposes; it is
**one affected set that contains the entire rust lane**, because the build tool
maps every changed BUILD to its package and then to that package's transitive
dependents.

On a pull request the build is one job per OS (only the scheduled full build is
sharded into 15). The macOS job therefore had to build and test every rust
package in a single job, and GitHub cancelled it at exactly 6h01m
(`build (macos-latest)` started 13:31:42 and the run's conclusion was
`cancelled` at 19:32). Linux and Windows finished; macOS is the slow one
because of linking. The required `CI gate` check then failed, so the PR could
not merge however correct the change was.

A secondary cost, discovered the same way: the affected set also re-triggered
~13 downstream workflows for that head (Release Engram, Release TaskApp, MJPEG
matrix jobs, Miri, VisiCalc, V8C, MosaicBook, ...), which queue behind each
other and saturate the repository's runner capacity for hours. The CI run
itself sat `queued` for 8 hours before it started, for this reason.

### The fix

Split a whole-lane mechanical change into subsets that each fit the smallest
job budget, and land them one PR at a time. A useful size check before opening
the PR: the *number of affected packages*, not the number of changed files —
`./build-tool -root . -diff-base origin/main -detect-languages` and the plan's
package count say what the slowest job will be asked to do. If the affected set
is a whole large lane, expect the macOS job to be the binding constraint.

For the rust pin specifically the split is by package directory (roughly 300-400
packages per PR), each PR adding `--locked` to those BUILDs and nothing else.
`pinned.json` gains `rust` only in the last of them, because a language is
pinned only when every BUILD in it installs in locked mode.

### What to do differently

- Before touching every BUILD in a lane, count the **affected packages** and
  check the change against the 6-hour macOS job cap. A 2,000-file change whose
  affected set is 1,500 packages is one job, not many.
- Treat a mechanical whole-lane edit as a series of PRs by default. The
  per-file edit is scriptable, so the split costs almost nothing to produce.
- Watch for the second-order fan-out too: a change that pulls a large lane's
  packages into the affected set also re-runs every workflow that watches those
  paths, and they compete for the same runners.
