## Unreleased

- `BUILD_windows` now runs `cabal test all`, the same command as the Unix `BUILD`, instead of
  skipping with "haskell support not enabled in windows CI yet". This package is now built and
  tested on Windows.
- The previous Windows recipe only ran Cabal if `%ProgramFiles%\cabal\bin\cabal.exe` existed. The
  setup step installs Cabal under `C:\ghcup\bin`, so the file never existed and the package passed
  without running any tests. A missing `cabal` now fails the job visibly.
