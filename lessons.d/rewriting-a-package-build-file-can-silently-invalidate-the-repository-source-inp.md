---
category: BUILD files & dependency management
---

# Rewriting a package BUILD file can silently invalidate the repository source-input boundary fixture, whose digest is pinned in 21 files

`code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json` lists, per
boundary, the exact package roots whose BUILD files consume a shared input. The
`starlark-rust-library-rule-consumers` boundary is "every Rust BUILD that mentions
`rust_library.star`", and `test_repository_source_input_boundary_is_closed_and_canonical`
recomputes that list from the BUILD files on disk and compares it with the fixture.

LP-001 rewrote `code/packages/rust/gf256/BUILD` and `BUILD_windows` to call cargo directly,
so they stopped mentioning `rust_library.star`. Nothing in that PR touched the fixture,
and the test only runs in the build-tool conformance step, so it was green until the
fixture next mattered. Fixing the fixture is not one edit, because the boundary digest is
pinned in 21 places and two generated projections carry the root lists:

- the fixture itself (drop the root from that boundary's `exact_roots`);
- `code/programs/dotnet/build-tool-csharp/SourceInputRegistries.Generated.cs` and
  `code/programs/swift/build-tool/Sources/BuildToolCore/RepositorySourceInputBoundary.swift`
  (the same root, same boundary);
- the digest, recomputed with `repository_source_input_boundary_digest`, in the Go, C#
  and Swift hasher sources and tests, the Python runner test, and 15 case fixtures;
- the `scope_count` and `authorization_count` in the Python test (each drops by one per
  removed root).

Two things made this slow to see:

- **The assertion order masked it.** The TypeScript-roots assertion earlier in the same
  test method failed first on `main`, so the Rust one never ran. Fixing the TypeScript list
  (#19050) is what exposed it. After you fix the assertion a failure points at, re-run the
  whole test; the next failure is often behind it.
- **A BUILD file's text is an input.** Changing what a BUILD file *says* (not just what it
  does) can change which boundary it belongs to. If you rewrite a BUILD file, grep the
  boundary fixture for its root, and run
  `python3 code/scripts/build_tool_conformance.py validate-corpus` plus
  `python3 -m unittest discover -s code/scripts/tests -p test_build_tool_conformance_runner.py`.

Recipe that worked without a Swift or .NET toolchain: edit the three lists, recompute the
digest from the fixture with the runner's own function, replace the old digest in every
file that pins it (`git grep -l <old>`), update the two counts from the test's own failure
message, then confirm `validate-corpus` reports `valid` and nothing still holds the old digest.
