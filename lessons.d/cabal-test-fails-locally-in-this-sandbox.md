---
category: Haskell
---

# `cabal test` fails locally in this sandbox

with `ghc-pkg-9.4.8.exe: ...package.conf.inplace\: openBinaryTempFileWithDefaultPermissions: invalid argument` — confirmed via `git stash` that this reproduces on unmodified/original package code too, so it's a pre-existing environment issue (likely OneDrive-sync interference with the deeply-nested `dist-newstyle` path), not a real code bug. `cabal build` (compile + link, including the test suite's own component) works fine; only the package-registration step for local "inplace" packages fails. Locally, rely on `cabal build cowsay:test:spec` (or the equivalent target) for type-check confidence, then run the built test `.exe` directly if you need to see it execute, or trust CI (which doesn't hit this OneDrive-path issue) for the official `cabal test` run.

The 2026-10-12 Haskell registry run reproduced the registration error in a
managed `.codex` worktree with Cabal 3.16 and GHC 9.4.8. A short external
`--builddir` allowed real native RED and GREEN tests. Both unchanged BUILD
fronts then passed through a verified short junction alias to that same
worktree, with their original arguments and the pinned native toolchain.
This demonstrates a path-setup workaround without claiming OneDrive is the
only cause. Cabal 3.16's v2 build directory is command-line-only;
`CABAL_BUILDDIR` and global config do not relocate `dist-newstyle` for v2.
Keep aliases and generated outputs in task-owned temporary storage and verify
their exact targets before cleanup; do not substitute a successful skip or
argument-modifying wrapper for an unchanged front run.
