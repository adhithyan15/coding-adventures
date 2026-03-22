# Changelog

All notable changes to the `pwd` program (Elixir) will be documented in this file.

## [1.0.0] - 2026-03-21

### Added

- Initial implementation of the POSIX `pwd` utility in Elixir.
- Logical path mode (`-L` / `--logical`) using `$PWD` environment variable with validation.
- Physical path mode (`-P` / `--physical`) using `File.cwd!/0` with `realpath` symlink resolution.
- Full CLI Builder integration via `pwd.json` spec for argument parsing, help text, and version output.
- POSIX-compliant fallback: logical mode falls back to physical path when `$PWD` is unset or stale.
- Comprehensive test suite covering CLI parsing integration and business logic functions.
