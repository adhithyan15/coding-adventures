# Changelog

## [Unreleased]

### Changed

- `--language perl` now also writes a real `BUILD_windows` (the `REM prereqs:` line the validator needs plus `run-perl-tests-windows.ps1 -Deps ...`, the same shape as the Go generator). Before, a generated Perl package had no `BUILD_windows`, so Windows fell back to the Unix `BUILD` (`cpanm`, `prove` under the wrong Perl). Validated by generating a Perl and a Lua package and running the build tool's `-validate-build-files` over them (BUILD_linux copy trick).
- Generated packages now get a real `BUILD_windows` (the same `busted` run as
  their Unix `BUILD`) instead of `echo "Skipping Lua on Windows CI"`. An
  unlisted echo-only `BUILD_windows` fails `audit_build_windows.py --check`
  (BUILD-002 W6).
- `BUILD_windows` now runs `lua scaffold.lua --help` on Windows, mirroring the Unix `BUILD`, instead of skipping.

## 0.1.0 — 2026-03-23

### Added
- Initial Lua scaffold generator implementation.
- Generates rockspec, BUILD, BUILD_windows, init.lua, test stub, README, CHANGELOG.
- Dependency resolution: transitive closure via BFS, topological sort via Kahn's.
- CLI with --depends-on, --layer, --description, --type, --dry-run options.
- Name normalization: kebab-case input → snake_case dirs, coding-adventures-* rockspecs.
