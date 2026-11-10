---
category: CI & GitHub Actions
---

# Converting a BUILD from Starlark to shell must update the repository source-input boundary

PR #19077 replaced `code/packages/rust/gf256/BUILD` (a Starlark file that
loaded `code/packages/starlark/library-rules/rust_library.star`) with a plain
shell BUILD. Its PR CI passed, but `main` then failed
`test_repository_source_input_boundary_is_closed_and_canonical`:

```
AssertionError: Lists differ: [... 'code/packages/rust/gf256', ...] != [...]
```

`code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json`
lists, per Starlark rule file, exactly which packages load it
(`starlark-<lang>-library-rule-consumers`). The test recomputes that list from
the BUILD files. PR CI only runs the conformance tests when the diff touches
build-tool inputs, so the drift surfaced on the next PR that did.

Fix: remove the root from the boundary JSON and from the C# and Swift
projections (`SourceInputRegistries.Generated.cs`,
`RepositorySourceInputBoundary.swift`), recompute the digest with
`build_tool_conformance.repository_source_input_boundary_digest`, replace it in
the corpus cases, the Go/C#/Swift tests and the runner test, and update the
scope/authorization counts.

Do differently: whenever a package's BUILD starts or stops loading a Starlark
rule (or a new package extends `tsconfig.base.json`), run
`python3 -m unittest discover -s code/scripts/tests -p 'test_build_tool_conformance_runner.py'`
before pushing, even though PR CI may not.
