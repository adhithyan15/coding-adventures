---
category: BUILD files & dependency management
---

# Converting a Starlark BUILD to shell commands changes the source-input boundary: sync the fixture, its digest pins and the generated projections in the same PR

#19311 (Warrant adoption for `rust/arithmetic`) rewrote that package's BUILD from a Starlark
`rust_library(...)` target into plain shell commands. The repository source-input boundary
fixture (`code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json`) lists, per
Starlark rule, every package whose BUILD loads the rule file, and a Python test recomputes that
list from the BUILD files. Nothing updated the fixture, so `test_repository_source_input_boundary_is_closed_and_canonical`
went red on main and took the required "Repo-wide metadata contracts" check with it for every PR.
The PR's own CI did not catch it because that contract step is path-gated and the files the PR
changed did not select it.

Changing the boundary is not a one-line edit. In the same PR:

1. Edit the fixture (here: delete the one root).
2. Regenerate the embedded projections with the repo's generators, not by hand:
   `pwsh code/programs/dotnet/build-tool-csharp/tools/generate-source-input-registries.ps1` and
   `pwsh code/programs/swift/build-tool/tools/generate-repository-source-input-boundary.ps1`.
3. The boundary digest changes, and it is pinned as a literal in ~22 files: the neutral case
   fixtures under `build-tool-v1/cases/`, the Go, C# and Swift implementations and their tests,
   and the Python runner test. Replace the old digest everywhere (`grep -rl <old>`), and confirm
   zero old pins remain.
4. Update the hard-coded `scope_count` and `authorization_count` in
   `test_build_tool_conformance_runner.py` (each fell by one for one removed root).
5. Run all four `test_build_tool_conformance_*.py` files, `go test` in `go/build-tool` and
   `go/kiln`, the Python `test_graph_diff.py`, and the C# build-tool tests (run them in place:
   they locate the repo root from the test binary, so a `--artifacts-path` outside the repo
   fails with "could not locate the repository root", which looks like a real failure and is not).

Before this fix the runner showed 1 failure in CI; after changing only the fixture it showed
28 locally, which is the digest pin telling you the other places still to move.
