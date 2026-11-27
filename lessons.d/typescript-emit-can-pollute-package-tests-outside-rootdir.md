---
category: TypeScript / JavaScript
---

# TypeScript emit can pollute package tests outside rootDir

A package build can report TS6059 because tests or configuration files sit outside the configured `rootDir`, while still emitting JavaScript and declarations beside those inputs. Vitest may then discover both `.ts` and generated `.js` tests, or select a generated `.js` config instead of the intended `.ts` config. The resulting test run is not a clean-source validation and can silently double-run stale compiled tests.

Use `tsc --noEmit` for diagnostic typechecks that are not intended to emit. For any emitting command, inspect its output paths and the worktree status immediately afterward. Run package discovery and consumer checks from a fresh committed-source checkout or an isolated output directory, with explicit Vitest inclusion where appropriate. Preserve TS6059 as a build failure; do not treat a later polluted test pass as proof that the package build succeeded.
