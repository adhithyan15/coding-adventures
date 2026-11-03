---
category: Cross-platform & Windows BUILD_windows
---

# Adding POSIX sh syntax to a BUILD requires a cmd.exe BUILD_windows beside it


PR #18810 added the macOS `DEVELOPER_DIR` guard (`if [ "$(uname -s)" = Darwin ]; then ...`)
to about 200 Haskell `BUILD` files. Windows CI then failed in seven of them with
`"$(uname -s)" was unexpected at this time` and `-v was unexpected at this time`.

The Go build tool runs a package's `BUILD_windows`, or its `BUILD` when there is
none, through `cmd /C`. It translates only a plain `VAR=value cmd` prefix. So:

- a `BUILD` that gains `sh`-only syntax must get a `BUILD_windows` at the same
  time; `rng` had none, so the guard went straight to `cmd.exe`;
- a `BUILD_windows` must itself be `cmd` syntax. Fifteen Haskell ones used
  `command -v ... /dev/null`. They were broken on main all along but never ran,
  because nothing had changed those packages.

Before editing many BUILD fronts, check every one for a `BUILD_windows`, and run
the repo's own cmd idiom (`if exist "%ProgramFiles%\\cabal\\bin\\cabal.exe" (...)`).
`test_haskell_darwin_cabal_linker.py` now enforces both rules for Haskell.
