# Changelog

All notable changes to the `unix-tools` program (Rust) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- The crate now passes `clippy -D warnings` on Windows (the CI gate runs it there): imports and helpers only used by Unix code or Unix-only tests are `#[cfg(unix)]`-gated in `chmod_tool`, `chown_tool` and the `id`/`logname`/`whoami` test modules, the integration tests that are mostly Unix-only carry a file-level `#![cfg_attr(not(unix), allow(unused_imports, dead_code))]`, and `ln_tool` returns a `String` instead of a no-argument `format!`.

### Changed — the suite now runs on Windows

`BUILD_windows` was an `echo` ("Skipping ... targets Unix/POSIX behavior"); it is
now `cargo build && cargo test --all-targets`, same as `BUILD`.  The crate
already compiled for Windows, and the tests that need Unix-only APIs
(permission bits, `chown`, symlinks, user/group lookup, `df`, `tty`, `uname`,
...) were already behind `#[cfg(unix)]`, so a Windows run reports its own
smaller count of passed tests rather than hiding anything.  Checked by running
the whole suite for `x86_64-pc-windows-gnu` under Wine (1278 passed, 0 failed).

The only failures were ten tests (three unit tests in `env_tool` and `xargs_tool`,
seven integration tests in `tests/env_test.rs` and `tests/xargs_test.rs`) that spawn
a real `echo` or `false`: neither exists as a Windows executable.  They now spawn the real programs on Unix and, elsewhere,
the test executable itself (`--list` exits 0, an unknown flag exits 101), so the
spawn-and-wait path is still exercised on every OS.

## [1.0.0] — 2026-03-21

### Added

- Restructured from standalone `pwd` program into `unix-tools` collection.
- Initial tool: `pwd` — print working directory, powered by CLI Builder.
- Logical path mode (`-L` / `--logical`): reads `$PWD` with validation against real cwd.
- Physical path mode (`-P` / `--physical`): resolves all symlinks via `canonicalize()`.
- Spec file (`pwd.json`) defining the CLI declaratively.
- Integration tests verifying CLI Builder parsing: default behavior, `-P`, `-L`, `--help`, `--version`, unknown flags.
- Business logic unit tests for `get_physical_pwd()` and `get_logical_pwd()`.
- Literate programming style with extensive documentation throughout.
