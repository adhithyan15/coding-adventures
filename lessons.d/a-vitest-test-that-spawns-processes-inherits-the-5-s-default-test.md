---
category: TypeScript / JavaScript
---

# a vitest test that spawns processes inherits the 5 s default test timeout even when its spawnSync has a longer budget, so it fails on slow Windows runners

**What went wrong.** #18791 added `real CLI returns exit 2 for a Cargo self-edge` to
`code/programs/typescript/build-tool/tests/resolver.test.ts`. The test starts the CLI as a child
process (`spawnSync(node, [tsx, src/index.ts, ...], { timeout: 30_000 })`). That 30 s limit applies
only to the child process. The test itself still ran under vitest's default `testTimeout`
of 5 s. On `windows-2025` the process start alone took 5.26 s, so the test timed out with
no assertion failing. The existing `gitdiff.test.ts` case, which makes about ten git calls,
timed out the same way at 5.25 s. Both passed on macOS in well under a second.

**The fix.** `code/programs/typescript/build-tool/vitest.config.ts` sets `testTimeout: 30_000`,
matching the child-process limit, with a comment explaining why.

**Do differently.** When a test spawns processes, give the test itself a time limit at
least as long as the child's. Set it per test (`it(name, fn, 30_000)`) or in the package's
vitest config. A spawn that takes half a second on macOS can take five on a hosted
Windows runner.
