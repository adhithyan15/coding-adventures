# Changelog

## Unreleased

### Fixed

- `go_build_cache` understands Cargo 1.100's build-dir layout
  (`<profile>/build/<name>/<hash>/out/<exe>`) as well as `deps/`, so the Go build
  cache still lands in the profile directory. Its test now asserts that property
  (the cache sits in an ancestor of the test binary that holds Cargo's `deps` or
  `build` directory) instead of a fixed `deps/` layout; it failed on the Windows
  runner under Rust 1.100.

## 0.1.0 — Private scratch directories and an isolated Go build cache

First release, from the security review of #18788 and the
`twig_pairs_and_closures.rs` flake.

- `Scratch` -- a per-call private directory: random name, created with
  `create_dir`, mode `0700` on Unix, removed with its contents on drop.
  `path`, `join` (which accepts only a single plain file name) and `write`.
- `write_new` / `create_new` -- `fs::write` / `File::create` with
  `create_new`, so an existing file or a planted symlink is an error.
- `go_command` / `go_build_cache` -- `go` with `GOCACHE` set to
  `<target>/<profile>/sir-go-build-cache`, so a `go clean -cache` elsewhere
  cannot delete export data a running test compiles against.
- Unit tests for each guarantee: fresh, private and removed (also on panic),
  no clobbering, no symlink following, plain names only, and where the cache
  lives.
