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
