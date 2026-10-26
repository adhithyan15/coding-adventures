---
category: Cross-platform & Windows BUILD_windows
---

# Invoke the same Node executable directly when npm script PATH lookup fails

On this Windows host npm itself was available, but `npm run build` could not
resolve `node` in its child shell. Prepending the expected directory to the
PowerShell Path did not repair that child lookup. The native interchange runner
could still execute the installed Node binary successfully.

Run the exact package-script entry point with the explicit verified Node
executable: TypeScript's `node_modules/typescript/bin/tsc --noEmit` and Vitest's
`node_modules/vitest/vitest.mjs run --coverage`. This preserves the production
build and test commands rather than changing package sources for a local shell
problem. Report the invocation boundary honestly; actual CI still runs the
checked-in npm scripts. Do not call a failed npm invocation a successful build.
