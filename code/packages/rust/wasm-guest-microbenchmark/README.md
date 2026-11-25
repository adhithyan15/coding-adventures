# WASM guest microbenchmark

This Rust library encodes one small WebAssembly module with recursive and
loop-based guest sums, then parses, validates, and instantiates it with the
repository's real `wasm-runtime`. The timed calls execute guest instructions
through `WasmRuntime::call`; they do not measure native WASM compilation or
simulated processor cycles. Setup and correctness checks happen before timing.

```powershell
cargo run --locked -p coding_adventures_wasm_guest_microbenchmark --bin wasm-guest-microbenchmark -- --pairs 40 --revision local --manifest wasm-envelope.json > wasm-report.json
cargo run --locked -p coding_adventures_wasm_guest_microbenchmark --bin wasm-guest-microbenchmark -- --replay wasm-report.json
cargo run --locked -p coding_adventures_wasm_guest_microbenchmark --bin wasm-guest-microbenchmark -- --replay-envelope wasm-envelope.json
cargo test --locked -p coding_adventures_wasm_guest_microbenchmark
```

The CLI writes the standard raw-pair `Report` JSON to stdout and a separate
strict guest envelope to the explicit `--manifest` path. It records the runtime,
guest boundary, exact core report, host clock/no-op samples, and 21 raw empty-
guest batch durations with their calibration settings. Each
`WasmRuntime::call` rebuilds the execution engine and transfers instance state;
the empty guest samples expose this runtime boundary. Diagnostics are context
only and are never subtracted from raw timings.

The shared Rust support library supplies collection and statistics. Tests replay
all portable cases from this package's own fixture, comparing canonical repository
bytes when available. Extracted source does not need a sibling microbenchmark
package's fixture file. Publishing a crate still requires its internal runtime
dependencies to be available through the chosen release mechanism.
