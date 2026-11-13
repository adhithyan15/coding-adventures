## Unreleased

- The generated-package `BUILD_windows` is now `cabal test all` instead of the
  `echo "haskell support not enabled in windows CI yet -- skipping"` record. An
  unlisted echo-only `BUILD_windows` fails `audit_build_windows.py --check`
  (BUILD-002 W6), and the Windows runner already has GHC and Cabal. It still
  never carries the POSIX Darwin prefix and does not probe for `cabal`, so a
  missing Cabal fails visibly. The spec test pins the exact text.
