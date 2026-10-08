# Changelog

All notable changes to the Python `cowsay` program are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- **`-f` / `--file` can no longer read `.cow` files outside the cows directory
  (issue #12169).** The cow name was interpolated straight into
  `code/specs/cows/<name>.cow`, so `-f ../../../../some/dir/file` or an
  absolute path opened (and printed) any readable `*.cow` file on disk. Cow
  selection now lives in the new standard-library-only `cow_path.py` (`is_safe_cow_name` / `resolve_cow_path`) and goes through two layers, matching the
  C#, F#, Java, Kotlin, Perl, Haskell, Dart, Lua and Swift ports:
  1. a syntactic check accepts only a bare file stem — it rejects the empty
     name and any name containing `/`, `\`, `..`, `:` or NUL (which also covers
     every absolute or drive-qualified path);
  2. a containment check, using `os.path.realpath` + `os.path.commonpath`, requires the resolved file to stay
     inside the cows directory.

  A rejected name behaves like an unknown cow: `default.cow` is drawn.
  `load_cow` now takes the cows directory rather than the repository root.

### Added

- Path-traversal tests in `tests/test_cow_path.py`: bare-name acceptance, relative (`../`,
  `..\`), absolute, nested (`dir/name`), percent-encoded and NUL-bearing names,
  a symlink-escape case (skipped on Windows), and a check that the repository's
  real cows still load.
- `tests/`, a `pyproject.toml` carrying the ruff configuration, and this package's first `BUILD`, `README.md` and `CHANGELOG.md`. `main.py` was also brought up to ruff cleanliness (one statement per line, sorted imports, no unused `json`/`ParseResult` imports, no function-local `import re`).
