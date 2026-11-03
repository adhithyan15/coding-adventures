# Changelog

## Unreleased

- Add `BUILD_windows` (the repo's `cmd.exe` idiom, `if exist "%ProgramFiles%\cabal\bin\cabal.exe" (...)`), so Windows CI no longer runs the POSIX `sh` `BUILD` through `cmd /C`.
