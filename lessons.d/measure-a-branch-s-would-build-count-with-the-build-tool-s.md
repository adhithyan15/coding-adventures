---
category: CI & GitHub Actions
---

# Measure a branch's would-build count with the build tool's dry run before opening the PR, and land rebuild-the-world changes incrementally

#19080 (capability-analyzer hardening) bundled 17 commits: the analyzer
crate, its specs, certificate regeneration, 14 manifest schema fixes, and
new `required_capabilities.json` files for ~57 leaf crates. Each part was
small, but together they touched ~75 Rust packages, and the diff-based build
tool selected **1013 of 5552 packages**. On contended runners the Windows job
needed 4-5 hours. It hit the 300-minute step limit twice, once on raw
slowness (5539/5552 done) and once on an unrelated intermittent hang
(`rust/xlsx-writer`) that a smaller build would not have run. Each retry cost
every OS a full rerun, which blocked the PRs queued behind it for most of a
day. The PR was then split after the fact.

A PR's CI cost is set by its fan-out, not its line count. A one-line manifest
change in a widely-depended-on crate costs more than a thousand lines in a
leaf.

Do this before opening (or pushing more onto) a PR:

1. Build the tool once:
   `cd code/programs/go/build-tool && go build -o "$SCRATCH/bt" .`
2. From the branch's worktree, run
   `"$SCRATCH/bt" -root . -diff-base origin/main -dry-run 2>&1 | tail -1`.
   The `K would-build` it prints is what CI will build.
3. If K is more than about 60, or touches packages known to be slow on
   Windows/macOS, split before pushing:
   - the core change, together with only what must change with it to stay
     green (e.g. packages whose BUILD runs the changed tool);
   - follow-up fixes;
   - mechanical sweeps (one manifest or one lint fix per crate across many
     crates) in batches of a few crates each, chosen by fan-out. A crate with
     many dependents gets a batch of its own.
   Measure each piece the same way.
4. Order the pieces so each lands green on its own: base later pieces on main
   once their prerequisite has merged, never as a stacked chain that a squash
   merge would break.
5. Re-measure after merging main: a merge can pull a wide change into a
   previously narrow branch.

When a wide PR is unavoidable (it really changes something everything
depends on), say so in the PR body with the K value and expect a long CI.
Never let it sit at the front of a serialized queue: land the narrow PRs
around it first.
