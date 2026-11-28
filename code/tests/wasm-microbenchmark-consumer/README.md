# External WASM microbenchmark consumer

This Node host proves that an independent runtime can consume the emitted
`microbenchmark_wasm.wasm` file without linking to its Rust crate. The
`wasm/microbenchmark` package's Unix and Windows BUILD files invoke it after
building the artifact. The Go build tool registers this exact script as a
repository input of `wasm/microbenchmark`, so edits to the assertions schedule
and re-key the producer's test. This directory is not a separate discovered
package root.

The host checks the binary's import/export boundary and declared memory
limits, then loads the artifact through Node's built-in WebAssembly APIs and
exercises collection, analysis, replay, input validation, reentrancy, and
exact allocation ownership.

Run it after the producer package builds its pinned artifact:

```sh
node consumer.mjs ../../packages/wasm/microbenchmark/target/wasm32-unknown-unknown/release/microbenchmark_wasm.wasm
```

The source intentionally uses only Node built-ins. Its injected clock is a
deterministic counter for assertions, not a timing measurement.
