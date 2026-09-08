# RISC-V RV64I + M gate-level simulator

Exact gate-level partner for the completed Spec 07y RV64I+M machine. The
topology contains 526,401 clocked D flip-flops: 524,288 memory bits, 2,048 GPR
bits, 64 PC bits, and one halt bit. Installed program origin and length are
validated lifecycle metadata rather than architectural storage.

The crate owns an independent strict RV64I+M decoder and execution engine.
Repository gates implement Boolean logic and decode, 64-bit ripple arithmetic,
signed and unsigned comparison, barrel shifts, addresses and branches, 32-bit
word-result networks, and fixed-width multiply and restoring divide. The
checked state, trace, result, and fault contracts are shared with
`riscv-rv64i-simulator`.

Verification covers exact topology, lifecycle and transactional faults, every
malformed-decode family, and all 364 reproducible Python full-state vectors in
complete trace/state lockstep with the functional Rust oracle. Strict Rustfmt,
Clippy, and rustdoc pass; package line coverage is 98.76% (717/726).

Run the package checks from the Rust workspace:

```sh
cargo test -p riscv-rv64i-gatelevel
cargo clippy -p riscv-rv64i-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p riscv-rv64i-gatelevel --no-deps
```
