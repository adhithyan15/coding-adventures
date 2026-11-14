---
category: BUILD files & dependency management
---

# Rewriting a package's BUILD can silently drift the repository-source-input boundary registry, because the conformance test only runs when build-tool inputs change

PR #19311 rewrote `code/packages/rust/arithmetic/BUILD` as plain command lines, so it stopped referencing `rust_library.star`, but the `starlark-rust-library-rule-consumers` boundary in `code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json` still listed it. The conformance test recomputes those roots from the BUILD files, yet CI only runs it when build-tool inputs change, so main stayed green and the next PR touching those inputs (#19354) failed.

Fix: drop the root from the neutral fixture, regenerate the C# and Swift projections with their `.ps1` generators, and update every pinned boundary digest (it appears in the `cases/*.json` corpus, Go, C# and Swift hashers) and the scope/authorization counts.

Next time: when a BUILD change adds or removes a reference to a shared input (a `library-rules/*.star` file, a shared config), run `test_build_tool_conformance_runner.py::CorpusTests::test_repository_source_input_boundary_is_closed_and_canonical` locally. Do not rely on CI to trigger it.
