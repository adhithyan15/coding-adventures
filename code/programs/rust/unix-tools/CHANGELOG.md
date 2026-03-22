# Changelog

All notable changes to the `unix-tools` program (Rust) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
