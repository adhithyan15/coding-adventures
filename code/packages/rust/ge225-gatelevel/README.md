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
overlapping `MOV`, manual decimal/clock examples, and 48 seeded decimal vectors.
P006B2 adds 53 DFF-backed direct-I/O state bits, exact punched-card DMA and sync
words, continuous reader slots, bounded card/paper-tape/typewriter queues,
shared N-command routing, readiness branches, parity/overrun/priority alarms,
atomic transfer validation, and functional-oracle lockstep coverage. The 33
combined tests cover 83.48% of core lines (1,339/1,604). The later
P006B3 slice adds 1,085 DFF-backed controller/API bits: eight controller status
banks, bounded selector commands, condition branches, ready-event latches,
special X-group 32 vectoring/return, and BRU target inhibition. Its 13 new tests
bring the combined suite to 46 tests and 84.92% core line coverage
(1,650/1,943). P006C completes the model with 167 AAU DFFs: separate 40-bit
AX/BX/QX/IX registers, calculation mode and ready state, transient and hold
alerts, gate-vector fixed and floating add/subtract, partial-product multiply,
widened-remainder restoring divide, normalization, all exact memory/general/
status words, and atomic preflight. Fourteen AAU oracle tests bring the combined
suite to 60 tests, cover 85.61% of core lines (2,190/2,558), and close the final
full-family differential audit.

Run from `code/packages/rust`:

```sh
cargo test -p ge225-gatelevel
cargo clippy -p ge225-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ge225-gatelevel --no-deps
```
