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

**Recurrence: #20138 (typescript/nand-flash).**

PR #20138 added `code/packages/typescript/nand-flash` (with a `tsconfig.json`
extending `../tsconfig.base.json`) and again did not add the root to the
`typescript-workspace-configuration` boundary. Main went red on
`test_repository_source_input_boundary_is_closed_and_canonical`, and the fix
touched 30 files (the fixture, both projections, the 27 digest sites and the
count bumps). This time a TypeScript package broke main; the sibling
`code/packages/python/nand-flash` needed nothing. The package's own PR should
have added the root: before pushing any new TypeScript package, run the single
corpus test above and follow this lesson's steps in the same PR.

**Resolution: `sync-boundary` generates all of it, and CI checks it on the adding PR.**

This was fixed by hand at least four times (#19903, #20109, #20124, #20143)
because the corpus test only ran on PRs that the build-tool CI gate selected,
and the PR that merely adds a package is not one of them. Do not follow the
numbered steps above any more. Run:

```bash
python3 code/scripts/build_tool_conformance.py sync-boundary
```

It recomputes the roots with the same function the corpus test uses
(`expected_repository_source_exact_roots`), rewrites the fixture, the C# and
Swift projections, the digest in all 27 pinned files and the count pins, and is
idempotent. `sync-boundary --check` changes nothing and exits 1 with a diff and
this command when anything is stale. The "Repo-wide metadata contracts" job
runs `--check` on every pull request (about five seconds, ungated), so the PR
that adds the package fails instead of main.

If the tool exits 2 it found a pin or projection in a shape it does not
recognise, or the old digest in a file it does not know. It never guesses:
teach `code/scripts/build_tool_boundary_sync.py` about the new site
(`DIGEST_PINS`, `PROJECTIONS`) rather than editing around it. A hand-made
boundary change (a new boundary id, new inputs) still needs a hand edit of the
fixture first; run `sync-boundary` after it for the digest and projections.
