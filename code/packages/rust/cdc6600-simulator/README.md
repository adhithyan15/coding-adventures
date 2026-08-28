# CDC 6600 Simulator (Rust)

Fidelity-first behavioral simulator for the repository's CDC 6600 (1964)
Central Processor subset. It implements every instruction in
`code/specs/07t-cdc6600-simulator.md`: 22 short register instructions and 14
long immediate, memory, and branch instructions.

The public model exposes eight 60-bit X registers, eight 18-bit A registers,
eight 18-bit B registers with B0 hardwired to zero, a checked parcel-address P
register, and 4,096 words of 60-bit memory. Four big-endian 15-bit parcels are
packed into each word. Short instructions consume one parcel; long instructions
consume two.

All architectural widths are explicit. X arithmetic wraps at 60 bits, A/B
arithmetic wraps at 18 bits, signed compares interpret bit 59, and multiply uses
a widened host intermediate before retaining the low 60 bits. Loads, stores,
branches, long-instruction fetches, program packing, and fall-through addresses
are validated before architectural state changes, so errors are atomic. B0 is
reasserted after every instruction and ignores every write path.

`Cdc6600Simulator::load` accepts the repository transport format: every 15-bit
parcel is right-aligned in a two-byte big-endian value. Non-canonical parcels,
odd byte lengths, and oversized programs are rejected without resetting the
machine. `step`, `run`, and `execute` return deterministic traces and never
allocate from the caller-provided step limit.

The behavioral scope intentionally matches the repository specification. The
historical machine's floating-point instructions, peripheral processors,
scoreboard timing, and exchange jump remain outside this package and belong to
future specification work rather than silent approximations.

Run verification from `code/packages/rust`:

```sh
cargo test -p cdc6600-simulator
cargo clippy -p cdc6600-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p cdc6600-simulator --no-deps
```
