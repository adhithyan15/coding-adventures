# GE-225 Gate-Level Simulator (Rust)

Gate-backed educational simulator for the General Electric GE-225. Persistent
architectural bits are stored in simulated D flip-flops, instruction selection
uses one-hot gate decoders, and arithmetic/logic results flow through the Rust
`logic-gates` and `arithmetic` primitives. Host integers are limited to control
sequencing, addresses, trace bookkeeping, and deterministic device queues.

RCPU-P006A establishes the storage, decoder, and central binary core and checks
it in lockstep against `ge225-simulator`. P006B1 adds DFF-backed decimal mode,
decimal carry, and 19-bit real-time-clock state; gate-only single/double BCD
arithmetic; `LAC`/`LCA`; deterministic 65-bit gate reduction for arbitrary
clock advances; and atomic BCD validation. The combined 23 tests cover lifecycle and
fail-closed bounds, one-hot decode, core-memory X modification, the complete
single/double binary instruction families, every central shift/normalize path,
overlapping `MOV`, manual decimal/clock examples, and 48 seeded decimal vectors;
core line coverage is 89.91% (1,257/1,398). The later P006B2/P006B3 and P006C
slices add direct I/O, selector/API state, then the separate AAU datapaths and a
full instruction-family differential audit.

Run from `code/packages/rust`:

```sh
cargo test -p ge225-gatelevel
cargo clippy -p ge225-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ge225-gatelevel --no-deps
```
