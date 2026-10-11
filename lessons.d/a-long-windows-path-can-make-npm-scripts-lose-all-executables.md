---
category: Cross-platform & Windows BUILD_windows
---

# A long Windows PATH can make npm scripts lose all executables

The desktop environment accumulated enough old worktree tool paths to exceed
cmd.exe's environment limits after npm added its node_modules/.bin prefixes.
`npm ci` succeeded, but both `npm test` and `npm run build` reported that vitest
and tsc were unrecognized. Their .cmd files and package metadata were present.
Run verification with a short process-local PATH containing the required tools;
do not change the package scripts or the user's persistent environment to hide
an execution-host limitation.
