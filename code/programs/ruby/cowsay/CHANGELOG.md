# Changelog

All notable changes to the Ruby `cowsay` program are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- **`-f` / `--file` can no longer read `.cow` files outside the cows directory
  (issue #12169).** The cow name was interpolated straight into
  `code/specs/cows/<name>.cow`, so `-f ../../../../some/dir/file` or an
  absolute path opened (and printed) any readable `*.cow` file on disk. Cow
  selection now lives in the new standard-library-only `cow_path.rb` (`CowPath.safe_name?` / `CowPath.resolve`) and goes through two layers, matching the
  C#, F#, Java, Kotlin, Perl, Haskell, Dart, Lua and Swift ports:
  1. a syntactic check accepts only a bare file stem — it rejects the empty
     name and any name containing `/`, `\`, `..`, `:` or NUL (which also covers
     every absolute or drive-qualified path);
  2. a containment check, using `File.realpath` + a separator-terminated prefix check, requires the resolved file to stay
     inside the cows directory.

  A rejected name behaves like an unknown cow: `default.cow` is drawn.
  `load_cow` now takes the cows directory rather than the repository root.

### Added

- Path-traversal tests in `test/test_cow_path.rb` (Minitest): bare-name acceptance, relative (`../`,
  `..\`), absolute, nested (`dir/name`), percent-encoded and NUL-bearing names,
  a symlink-escape case (skipped on Windows), and a check that the repository's
  real cows still load.
- `test/`, a `Gemfile` (minitest, standard), and this package's first `BUILD`, `BUILD_windows`, `README.md` and `CHANGELOG.md`. `main.rb` was also run through `standardrb --fix` (trailing whitespace, bare `rescue => e`).
