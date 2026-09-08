# AArch64 gate-level simulator

Exact gate-level partner for the completed Spec 07v AArch64 machine. The
topology contains 526,469 clocked D flip-flops: 524,288 memory bits, 2,048 GPR
bits, 64 stack-pointer bits, 64 PC bits, four NZCV bits, and one halt bit.
Installed program origin and length are validated lifecycle metadata rather
than architectural storage.

The crate owns an independent strict AArch64 decoder and execution engine.
Repository gates implement Boolean logic and decode, 32/64-bit ripple
arithmetic and NZCV, condition evaluation, barrel shifts and rotates, logical
bitmask application, big-endian memory assembly, bit reversal and counting,
fixed-width multiply/high-half, and restoring divide. The checked state, trace,
result, and fault contracts are shared with `aarch64-simulator`.

Verification covers exact topology, lifecycle and transactional faults, every
malformed-decode family, functional lockstep suites, and all 836 reproducible
Python full-state vectors. Strict Rustfmt, Clippy, and rustdoc pass; package
line coverage is 97.45% (763/783). The historical Python AArch64 gate consumer
also passes all 256 tests at 82.24% statement coverage.

Run the package checks from the Rust workspace:

```sh
cargo test -p aarch64-gatelevel
cargo clippy -p aarch64-gatelevel --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p aarch64-gatelevel --no-deps
```
