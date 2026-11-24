## Unreleased - 2026-11-23 - GC archive is built once per test process

### Fixed

- `tests/common::gc_core_capi_archive` now runs its nested
  `cargo build --release -p gc-core-capi` once per test process (a
  `OnceLock`) instead of once per call. Cargo deletes and re-links the
  uplifted `target/release/libgc_core_capi.a` on every build, even a fully
  cached one, so a parallel test thread's build could remove the file between
  another thread's build returning and its `is_file()` check (or its `clang`
  link). That intermittently failed
  `pr5a_string_failures_are_registered_traps_on_every_column` on macOS CI with
  "Cargo-reported archive does not exist". Reproduced locally: 40 cached
  builds with a polling reader saw the file missing 263 times. Test-only; no
  version bump.
