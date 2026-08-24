# Changelog — `gate-netlist-format`

## Unreleased — 2026-08-18

Brought under CI: the crate had no `BUILD` file, so nothing ever ran its tests
or linted it.

### Added

- **`BUILD` file — this crate is now built, tested and linted in CI.**

  This crate is a member of the `code/packages/rust` workspace, so it compiled
  whenever a sibling with a `BUILD` file pulled it in as a path dependency. But
  the build tool discovers work by scanning for `BUILD` files, so with none of
  its own it was never a package in its own right: its **test targets were never
  compiled, its assertions never ran, and `cargo clippy --all-targets -- -D
  warnings` never linted it**, on any platform. Adding `BUILD` puts it under the
  same per-package clippy gate and test run as every other watched Rust crate.

  The BUILD is the repo-standard one-liner, `cargo test -p gate-netlist-format -- --nocapture`,
  kept on a single line: the build tool runs each BUILD line as its own
  `sh -c`, so a backslash continuation would silently truncate the command.
  It was verified green locally first — clippy `-D warnings` clean and a full
  unfiltered `cargo test --no-fail-fast` passing — per the "expect to find
  existing breakage when you start watching a long-unwatched package" rule in
  `lessons.md`.

- **`README.md`** — where HNL sits in the synthesis pipeline, the
  GENERIC/STDCELL level split, the JSON schema, and why a connection is a
  bit-sliced `NetSlice` rather than a bare net name.
