## [Unreleased]

### Fixed

- Generated Haskell packages now get `BUILD_windows` = `cabal test all` instead
  of an echo skip ("haskell support not enabled in windows CI yet"). The
  toolchain is installed on the Windows runner, and BUILD-002 W6 makes an
  unlisted echo-only `BUILD_windows` a blocking audit finding.
- Generated Perl packages now get a real `BUILD_windows`: a `REM prereqs:` line
  and a call to `code\scripts\run-perl-tests-windows.ps1 -Deps <transitive
  siblings>` (the same set the Unix `BUILD` walks), replacing the
  "Perl testing is not supported on Windows - skipping" echo.
- New test `TestGeneratePerlWindowsRecipeIsReal`; the Haskell test pins the new
  recipe.
