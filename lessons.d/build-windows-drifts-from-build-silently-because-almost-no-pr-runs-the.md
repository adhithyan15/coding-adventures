# `BUILD_windows` drifts from `BUILD` silently, because almost no PR runs the Windows shard

Adding BUILD files to 93 orphan Rust crates pulled several Swift packages
into the affected set, which flipped `needs_swift=true`, which is the only
condition under which CI schedules `build (windows-latest)` at all. That job
then failed instantly — not on anything this branch wrote, but on **17
pre-existing `BUILD_windows` files** that had fallen out of sync with their
`BUILD` counterparts. `origin/main` fails identically; it just almost never
runs the job that would say so.

The mechanism is that `discovery.getBuildFile()` returns the *platform*
BUILD file, so on Windows every downstream check — the dependency graph, the
`# build-tool: deps=` directive scan, `ValidateBuildFiles` — reads
`BUILD_windows` and never sees `BUILD`. A dep added to `BUILD` is therefore
invisible on Windows until someone adds it there too, and nothing complains
until a Swift-touching PR happens by.

Three distinct shapes, all repairs of the same drift:

1. **Missing prereqs** (12 Python packages). `BUILD` had `-e ../cas-solve`,
   `BUILD_windows` did not. Purely additive fix.
2. **Stale over-listing** (2 Perl packages). These are skip-stubs whose
   `REM prereqs:` line exists only to declare refs; the line still named
   packages that had long since stopped being dependencies, which trips the
   *undeclared local package refs* check from the opposite direction.
3. **Missing `# build-tool: deps=` directive** (3 Swift packages). The Linux
   `BUILD` declares `deps=rust/conduit-capi`; the Windows twin referenced the
   same crate with no directive, so the ref was undeclared.

The reliable repair rule: **make `BUILD_windows` declare exactly the package
set `BUILD` declares**, adapting only path separators and the venv/interpreter
invocation. `BUILD` already passes this validator on Linux, so mirroring its
package list is correct by construction — no need to re-derive the graph by
hand. In all three shapes above the trimmed/extended list reproduced the
Linux file's list exactly, which is a good self-check that the repair landed.

Corollary: put `# build-tool: deps=` in the default `BUILD` so the Linux detect
job carries the edge into its shared plan. Repeat it as
`REM # build-tool: deps=` in `BUILD_windows`: the resolver still sees the
embedded marker, while `cmd /C` can execute the line safely if that platform
file is invoked directly.
