## [0.1.0] - UNRELEASED

### Added (census only)

- `kiln record prepare --allow-preloaded-state` (`Options.AllowPreloadedState`).
  It skips only the fail-fast `stateRootEmpty` refusal (the sandbox
  configuration and the probes' post-run state-root check; the tmpdir check is
  unchanged). RunStep's seq-0 `traces/.state-preloaded` marker and collect's
  `state-preloaded` shard-void are untouched, so a shard recorded with the flag
  is always void, by design.

### Changed

- `.github/workflows/kiln-census.yml`, `home: runner` only: the cargo state
  roots are no longer emptied per shard. A new step runs `cargo fetch --locked`
  for every tracked `Cargo.lock` (failures tolerated and counted) before the job
  start stamp, `prepare` gets `--allow-preloaded-state` and
  `CARGO_NET_OFFLINE=true`, the banner says the shard is void by design, and the
  90-minute cap has its own "BUILD TIMEOUT" message. This ends the sandbox's
  589-second `index.crates.io` retry per Rust step. Fresh mode is unchanged.
- KILN04 census section: documents the flag and that stage-5 keyed fetchers are
  the real path to Rust recordability.
