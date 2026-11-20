## [0.1.0] - UNRELEASED

### Added (census workflow: `home: fresh|runner`, KILN04)

- `.github/workflows/kiln-census.yml` takes a `home` input (`fresh`, the default
  and the old behaviour, or `runner`), validated in shell like `shard_count` and
  passed on as `HOME_MODE`.
- `home: runner` installs the toolchains the main build installs, with ci.yml's
  pins and SHA-pinned actions: Python 3.13.17 (+ uv 0.13.7, fontTools 4.68.0),
  Node 22.23.5 (+ npm 11.13.0), Rust 1.100.0 (clippy, llvm-tools, rustfmt) and
  cargo-tarpaulin 0.37.2, Haskell GHC 9.4.8 / cabal 3.10.3.0, .NET SDK 9.0.319,
  Ruby 3.4.12 (+ bundler 4.0.24), and the apt packages the Rust and Haskell
  builds need. All of it runs before the job-start stamp. A verification step
  fails the job when a toolchain does not resolve under a declared tool root of
  `code/kiln/legacy/host-roots.json`.
- With `home: runner` the recording uses the runner's real `$HOME` (never
  removed; only `~/.cargo/registry` and `~/.cargo/git`, the state roots, are
  emptied before each shard so `prepare` does not refuse `state-preloaded`), the
  start deadline drops to 180 minutes to make room for the longer setup, and a
  not-clean shard FAILS the job: `ci/census_not_clean.py --strict` exits 1 for
  skipped_too_large, actions_skipped, every trace refused, or an unreadable or
  missing summary. A `runner` run that exits 0 is the PR8 gate.
- `census_not_clean.py` gains `--strict`; without it the checker still always
  exits 0.
