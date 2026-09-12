# Apple M1 gate-level simulator

Exact gate-level partner for the completed Spec 07z Apple M1 teaching machine.
The topology contains 530,565 clocked D flip-flops: 524,288 memory bits, 2,048
GPR bits, 64 stack-pointer bits, 64 PC bits, four NZCV bits, 4,096 vector/FP
bits, and one halt bit. Installed program origin and length are validated
lifecycle metadata rather than architectural storage.

The crate reuses the independent Spec 07v2 AArch64 gate engine for the complete
integer core and owns an independent strict decoder for scalar FP, vector
memory, DUP, and NEON integer/FP instructions. Repository DFFs clock every
architectural bank. Boolean and ripple-carry gate networks implement NEON
integer lane add, subtract, and shift-add multiply; checked lane selection,
broadcast, masks, and big-endian memory wiring surround explicitly bounded host
IEEE-754 primitives for FP value operations.

The public state, error, trace, result, structured encoder, and atomic lifecycle
contracts match `apple-m1-simulator`. Eleven Rust tests cover exact topology,
lifecycle, malformed decode, transactional faults, instruction families, and
all 360 reproducible Python full-state vectors in complete functional trace and
state lockstep. Strict Rustfmt, Clippy, and rustdoc pass; package line coverage
is 90.52% (621/686). The 109-test Python Apple M1 and 256-test Python AArch64
gate consumers also pass.

Run from `code/packages/rust`:

```sh
cargo fmt -p apple-m1-gatelevel -- --check
cargo clippy -p apple-m1-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p apple-m1-gatelevel --no-deps
cargo test -p apple-m1-gatelevel
cargo llvm-cov -p apple-m1-gatelevel --summary-only
```
