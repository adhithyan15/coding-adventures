---
category: Workspace & package metadata
---

# TypeScript packages extending tsconfig.base must update the repository source-input boundary

When adding a TypeScript package whose `tsconfig.json` extends `../tsconfig.base.json`,
its package root must be registered in `code/specs/fixtures/build-tool-v1/repository-source-input-boundary.json`
under `typescript-workspace-configuration` (`applies_to.exact_roots`).

Because PR CI relies on git diff detection to run tests for changed packages,
repo-wide metadata contract checks (such as `test_build_tool_conformance_runner.py`)
do not run on package-only PRs unless the full build flag is set. When multiple
TypeScript packages are merged without updating the boundary fixture, the failure
only surfaces on `main` when full CI runs with `-force`.

Whenever `repository-source-input-boundary.json` is updated:
1. Recompute the boundary SHA-256 digest, `scope_count`, and `authorization_count`.
2. Update the assertions in `test_build_tool_conformance_runner.py`.
3. Update the matching `boundary_sha256` in the build-tool-v1 conformance test cases.
4. Regenerate and update the language-neutral build-tool projections and conformance assertions in Go, C#, and Swift.
