---
category: CI & GitHub Actions
---

# A new TypeScript package with tsconfig.base.json needs the repository-source-input-boundary fixture regenerated across all its projections

PRs #15797 and #15820 each added a TypeScript package (`forme-deploy-runner-fs-adapter`, `forme-deploy-runner-github-pages-adapter`) whose `tsconfig.json` extends `../tsconfig.base.json`, which qualifies them for the `typescript-workspace-configuration` boundary in `code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json`. Neither PR updated that checked fixture, so `test_repository_source_input_boundary_is_closed_and_canonical` (in `code/scripts/tests/test_build_tool_conformance_runner.py`) started failing on main as soon as both landed, because the test recomputes the expected root list live from the filesystem and compares it against the recorded one.

The fixture is not the only place the list is recorded: `repository-source-input-boundary.json` is projected byte-for-byte into per-language checked copies — `code/programs/swift/build-tool/Sources/BuildToolCore/RepositorySourceInputBoundary.swift` (via `tools/generate-repository-source-input-boundary.ps1`) and `code/programs/dotnet/build-tool-csharp/SourceInputRegistries.Generated.cs` (via `tools/generate-source-input-registries.ps1`) — plus the boundary's SHA-256 digest, which is asserted directly in the test and re-quoted in `code/specs/package-parity-roadmap.md` and in several `boundary_sha256` fields under `code/specs/fixtures/build-tool-v1/cases/`. Go and Haskell read the JSON fixture at test time instead of embedding a projection, so they don't need regeneration.

Adding a TypeScript package under `code/packages/typescript/` that extends `tsconfig.base.json` (or any other change to that boundary's root set) requires: inserting the new root(s) in UTF-8 byte order into the fixture, running (or, absent a `pwsh` toolchain, faithfully hand-reproducing) both `.ps1` generators, and then re-running `test_repository_source_input_boundary_is_closed_and_canonical` to pick up the new `scope_count`/`authorization_count`/digest and propagating that digest to every other file that quotes it.
