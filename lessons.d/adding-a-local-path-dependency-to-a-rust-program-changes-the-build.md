---
category: BUILD files & dependency management
---

# Adding a local path dependency to a Rust program changes the build tool's repository-source-input boundary; run sync-boundary before pushing

**What failed.** CI on #20175 (VOS-040b) failed the "Verify the repository-source-input boundary is in sync" step. `cargo test` passes locally either way, so nothing on the local inner loop catches it. A new local `path =` dependency in a Rust program's `Cargo.toml` changes which repository files the build tool treats as source inputs, and the boundary snapshot (JSON, `build-tool-v1/cases/*` fixtures, and the Go, Swift, C# and Python mirrors) no longer matched.

**The fix.** Run `py -3.13 code/scripts/build_tool_conformance.py sync-boundary`, then `validate-corpus`. It rewrites about 30 generated files across Go, Swift, C#, Python and the fixtures. Never hand-merge them: on a merge or rebase conflict, take main's version of every generated boundary file and re-run `sync-boundary`.

**The cross-PR hazard.** Two PRs that each change the boundary can both be green on their own and leave main missing one entry, because each regenerated against a main that lacked the other. That happened here: main lost `vos-model-sat` and every PR failed until someone resynced. After rebasing onto a main that moved, run `sync-boundary` again even if your branch was green before.
