---
category: Haskell
---

# Cabal 3.10 coverage planning must avoid vector internal libraries

PR #18183 failed before compiling `haskell/starlark-compiler`: GHC 9.4.8 and
Cabal 3.10.3.0 selected `vector-0.13.2.0`, then rejected its internal libraries
because coverage disabled the required per-component build mode. The exact
coverage command reproduced the same failure locally after rebasing.

Use a package-local compatibility constraint for the affected dependency plan;
do not disable coverage, change the inert provider API, or count a newer Cabal's
success as reproduction of the older CI failure. Verify the selected version,
run actual corpus tests, and measure the production module's HPC separately
from the test reader. PATH selected Cabal 3.16 locally, while the versioned
GHCup `cabal-3.10.3.0.exe` shim supplied the actual CI version.
