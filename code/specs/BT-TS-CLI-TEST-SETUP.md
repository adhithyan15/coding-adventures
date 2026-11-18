# TypeScript build-tool real CLI test setup

The TypeScript build-tool's real-process CLI tests exercise the executable
boundary, not only functions imported into Vitest. They compile the production
TypeScript entry point once per Vitest run with the package's locked TypeScript
compiler and `tsconfig.json`, then start a fresh Node process for each CLI
fixture against the emitted JavaScript.

The emitted test executable lives in a dedicated ignored directory beneath the
package root. Setup removes stale output before compilation and copies
`src/language-source-input-registry.json` byte-for-byte beside the emitted
`hasher.js`; setup fails if the copied bytes differ. Keeping the output beneath
the package root preserves Node's normal upward dependency lookup for
`createRequire(import.meta.url)` in the Starlark evaluator. Setup failure must
fail the suite, and teardown removes only this test-owned emitted directory even
when a test fails.

The duplicate-package-identity and invalid-UTF-8 CLI fixtures keep their exact
arguments, expected exit status, and byte-exact diagnostic assertions. Each
fresh Node process, including executable loading, module loading, fixture
discovery, validation, and exit, remains bounded by that fixture's
`limits.wall_time_ms` (currently 5,000 ms). Compilation is test setup outside
that per-invocation budget; the executable is not pre-warmed by a prior CLI
invocation. Both direct `vitest run` and the package BUILD front must include
this setup automatically.
