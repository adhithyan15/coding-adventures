# Guest microbenchmark support

Shared command parsing, raw duplicate-aware replay, and a versioned envelope
for guest-runtime microbenchmark lanes. `GuestEnvelope` keeps the portable
VER07 `Report` unchanged and stores runtime details and two separate raw
diagnostics: host clock reads around a no-op, and repeated empty calls through
the actual guest wrapper. Raw samples and calibration settings are preserved;
zero host diagnostic durations are valid clock-resolution observations, and
neither diagnostic is subtracted from workload results.

The crate exposes `replay_core_file` for portable JSON reports and
`replay_file` for guest envelopes. Both read bounded UTF-8 input and reject
duplicate JSON object members before materialization, including escaped-key
aliases. The external downstream smoke consumer at
`code/tests/rust-guest-microbenchmark-consumer` extracts the portable report
and verifies it equals the envelope report emitted by each guest driver.

```text
cargo test --manifest-path Cargo.toml
cargo run --manifest-path ../../../tests/rust-guest-microbenchmark-consumer/Cargo.toml -- report guest.json
```

The host runtime metadata field currently reports `unknown`: this library does
not invoke `rustc` during a consumer build to discover a compiler version.
It depends on the small `coding_adventures_microbenchmark` package and does not
depend on the optional Warrant framework.
