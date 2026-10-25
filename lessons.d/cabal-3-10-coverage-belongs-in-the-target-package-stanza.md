---
category: Haskell
---

# Cabal 3.10 coverage belongs in the target package stanza

PR #18183 failed before compiling `haskell/starlark-compiler`: GHC 9.4.8 and
Cabal 3.10.3.0 selected `vector-0.13.2.0`, then rejected its internal libraries
because coverage disabled the required per-component build mode. The exact
coverage command reproduced the same failure locally after rebasing. Narrowing
only vector exposed the same planner failure in `attoparsec-0.14.4`; its
preceding 0.14.3 release has no internal library in the package metadata.

Older attoparsec then conflicted with GHC 9.4's bundled ghc-prim. Version caps
therefore did not provide a sound fix. The deprecated library-coverage flag
is an alias and reproduces the original planner failure too.

Enable `coverage: True` in the named `package starlark-compiler` stanza of
`cabal.project`, and invoke `cabal test all` without the global coverage flag.
This preserves instrumentation of both the library and its test executable
without applying Cabal 3.10's global per-package planning restriction to all
dependencies. Keep dependency bounds unchanged and measure the production
module's HPC separately from the test reader. PATH selected Cabal 3.16 locally,
while the versioned GHCup `cabal-3.10.3.0.exe` shim supplied the actual CI version.

The distinction is visible in the pinned [Cabal 3.10.3.0 planner source](https://github.com/haskell/cabal/blob/cabal-install-v3.10.3.0/cabal-install/src/Distribution/Client/ProjectPlanning.hs):
the per-component restriction reads the global local-package configuration,
while instrumentation reads the merged package-specific configuration.
