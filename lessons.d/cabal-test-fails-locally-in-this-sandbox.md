---
category: Haskell
---

# `cabal test` fails locally in this sandbox

with `ghc-pkg-9.4.8.exe: ...package.conf.inplace\: openBinaryTempFileWithDefaultPermissions: invalid argument` — confirmed via `git stash` that this reproduces on unmodified/original package code too, so it's a pre-existing environment issue (likely OneDrive-sync interference with the deeply-nested `dist-newstyle` path), not a real code bug. `cabal build` (compile + link, including the test suite's own component) works fine; only the package-registration step for local "inplace" packages fails. Locally, rely on `cabal build cowsay:test:spec` (or the equivalent target) for type-check confidence, then run the built test `.exe` directly if you need to see it execute, or trust CI (which doesn't hit this OneDrive-path issue) for the official `cabal test` run.
