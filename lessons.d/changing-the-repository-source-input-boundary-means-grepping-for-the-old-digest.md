---
category: CI & GitHub Actions
---

# Changing the repository-source-input boundary means grepping for the old digest, not following a fixed file list

PR #16274 added `code/packages/typescript/forme-plugin-installer-core`. Its
`tsconfig.json` extends `../tsconfig.base.json`, which puts it in the
`typescript-workspace-configuration` boundary of
`code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json`. The
PR did not add it there. Once it merged, the "Repo-wide metadata contracts" job
failed on main (step "Verify build-tool conformance corpus"):
`test_repository_source_input_boundary_is_closed_and_canonical` recomputes the
root list from the filesystem and found one root that the fixture did not
have. This is the same failure that
`a-new-typescript-package-with-tsconfig-base-json-needs-the-repository-source.md`
records for #15797 and #15820. Main CI kept getting superseded and cancelled,
so the red job was easy to miss. Each merge left the PR gate reporting a
failure, but no run ever went red as a whole.

That earlier lesson's list of places to update is incomplete. The boundary
digest is also hardcoded as a constant in these files:

- `code/programs/go/build-tool/internal/graphdiff/graphdiff_test.go`
- `code/programs/dotnet/build-tool-csharp/SourceHashing.cs` (`RepositorySourceInputBoundaryDigest`)
- `code/programs/dotnet/build-tool-csharp/tests/BuildTool.CSharp.Tests/HasherConformanceTests.cs`
- `code/programs/swift/build-tool/Sources/BuildToolCore/Hasher.swift` (`repositorySourceInputBoundaryDigest`)
- `code/programs/swift/build-tool/Tests/BuildToolCoreTests/HasherTests.swift`

Those lanes only run when their build tool is affected. A fix that follows the
fixed list passes the Python contract job, and the Go, C# or Swift lane goes
red later.

What to do instead:

1. Insert the root(s) in UTF-8 byte order into the JSON fixture and, at the
   same indentation, into the Swift and C# projections
   (`RepositorySourceInputBoundary.swift`,
   `SourceInputRegistries.Generated.cs`). Both embed the JSON verbatim.
2. Compute the new digest:
   `python3 -c 'import sys; sys.path.insert(0,"code/scripts"); import build_tool_conformance as r; print(r.repository_source_input_boundary_digest(r._default_repository_source_input_boundary()))'`
   (needs `jsonschema==4.26.0`).
3. Replace the old digest in every file that `git grep -l <old-digest>` lists.
   Skip the dated narrative in `code/specs/package-parity-roadmap.md` and
   `.claude/package-parity-loop-state.json`, which records what an earlier PR
   validated.
4. Bump `scope_count` / `authorization_count` and the two
   `MAX_REPOSITORY_SOURCE_*` mock limits in
   `code/scripts/tests/test_build_tool_conformance_runner.py` by one per root.
5. Run `python3 -m unittest discover -s code/scripts/tests -p 'test_build_tool_conformance_runner.py'`
   and `python3 code/scripts/build_tool_conformance.py validate-corpus`.
