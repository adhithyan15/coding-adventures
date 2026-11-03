---
category: Cross-platform & Windows BUILD_windows
---

# Each Haskell package root that builds a shared Darwin closure must select the compatible linker itself

Fixing a dependency package's `BUILD` front does not protect a higher-level
package root, because the repository build tool executes only the selected
root's command and Cabal builds its whole local project in that process. A
macOS CI run selected `haskell/affine2d`; its unchanged front launched Cabal
without the Command Line Tools `DEVELOPER_DIR`, so GHC 9.4.8 invoked a linker
that could not parse the CLT SDK's newer `arm64e.x1` stubs even though the same
routing had already been repaired in trig, point2d, and bezier2d.

When a Haskell package owns a Cabal project that names sibling packages, audit
and repair the selected root itself. On Darwin, scope
`DEVELOPER_DIR=/Library/Developer/CommandLineTools` to that root's Cabal process;
on other hosts preserve the ordinary command. Validate every independently
selectable root, not only the leaf where the first linker failure appeared.

`code/scripts/tests/test_haskell_darwin_cabal_linker.py` now executes every
Darwin-reachable Haskell `BUILD` front with stubbed `uname` and `cabal` and
fails if any Cabal call misses the CLT `DEVELOPER_DIR` on Darwin, leaks it to
other platforms, or changes the command, failure status or skip behaviour.
Copy the guard from `code/packages/haskell/trig/BUILD` when adding a package.
