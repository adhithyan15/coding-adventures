# A fixed-once-per-package fix doesn't reach a fallback path — an absent `BUILD_windows` still runs the POSIX `BUILD` verbatim on Windows

PR #11553 (Swift ZIP) repaired the Windows dependency-closure detection so far more
packages actually get *evaluated* on `windows-latest` than before. That correctly
exposed 28 packages that were never Windows-tested until this PR, even though the
`.[dev]`-quoting bug they hit (lesson at line 31) has been documented for a long time.

- **`GetBuildFileForPlatform` (`code/programs/go/build-tool/internal/discovery/discovery.go`)
  falls back to the generic POSIX `BUILD` file when no `BUILD_windows` exists — for
  EVERY platform, including Windows.** There is no "this package doesn't support
  Windows" opt-out mechanism anywhere in the build tool (no marker file, no
  platform field). A package with only a `BUILD` file is not "skipped on Windows" —
  its POSIX script is handed to `cmd /C` verbatim. `#!/bin/sh` / `set -eu` headers,
  `if [ "$(uname)" = "Linux" ]; then ...; fi` guards, and quoted `-e ".[dev]"` all
  fail loudly (`Environment variable -eu not defined`, `"$(uname)" was unexpected at
  this time.`, `not a valid editable requirement`) the first time such a package
  actually gets exercised on Windows.
- **A known, documented fix (line 31) does not self-propagate.** 21 packages had the
  exact `-e ".[dev]"` quoting bug the lesson already named, either because they never
  had a `BUILD_windows` at all, or because their hand-authored `BUILD_windows`
  predated/diverged from `code/programs/python/scaffold-generator/scaffold_generator.py`
  (which has always emitted the correct unquoted form). Two packages
  (`prolog-core`, `swi-prolog-lexer`) used `.venv\Scripts\python -m pip install`
  instead of `uv pip install` — a second hand-authored variant of the same bug.
  **Fix:** unquoted `-e .[dev]` — copy an existing multi-dep `BUILD_windows` (e.g.
  `nib-parser`) as the template rather than re-deriving it.
- **A POSIX-only guard line (`if [ "$(uname)" = "Linux" ]; then cargo tarpaulin ...;
  fi`) is not "harmless to skip" on Windows — it is a syntax error there.** `cmd.exe`
  has no `[ ]`/`$()` and reads `$(uname)` as a literal command name. Fix: give the
  package a `BUILD_windows` that drops the tarpaulin line entirely (tarpaulin is
  Linux-only anyway) — see `bytecode-compiler/BUILD_windows` for the established
  one-line precedent.
- **When newly exposing a previously-untested platform for dozens of packages at
  once, budget review time per failure, not just per PR.** Six inherited defects
  were fixed before this head; 28 were still failing after — because "expose more of
  the closure" and "fix everything the closure exposes" are different amounts of
  work, and the former doesn't bound the latter.
