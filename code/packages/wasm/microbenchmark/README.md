# Reusable WASM microbenchmark library

This package builds a `wasm32-unknown-unknown` WebAssembly module with a small
caller-owned ABI for paired workload collection, statistical analysis, and
strict report replay. It delegates collection, replay validation, and rank
statistics to the repository's Rust `microbenchmark` core. The guest imports
only a monotonic clock and a batch-workload callback from its host; it has no
filesystem, process, random, network, or ambient clock access.

The ABI and its bounds are specified in
[`VER07aa-wasm-microbenchmark-library.md`](../../../specs/VER07aa-wasm-microbenchmark-library.md).
The result is an actual `.wasm` artifact, not a Rust host wrapper around a
different embedded workload. The separate Node consumer loads that artifact
from disk in a new host process and exercises its exported collection,
analysis, replay, and ownership contract.

Build and run the external consumer with the repository-pinned toolchain:

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target wasm32-unknown-unknown --release -- -D warnings
cargo build --locked --target wasm32-unknown-unknown --release
node ../../../tests/wasm-microbenchmark-consumer/consumer.mjs target/wasm32-unknown-unknown/release/microbenchmark_wasm.wasm
```

The module imports exactly `microbenchmark_host.now_ns` and
`microbenchmark_host.run_batch`. Hosts must keep their callbacks synchronous,
must not reenter mutating exports, and must copy result bytes before freeing
the exact returned pointer and length. The consumer assertions inspect actual
module imports/exports and memory limits in addition to invoking the API.
