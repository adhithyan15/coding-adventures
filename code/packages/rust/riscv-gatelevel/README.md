# RISC-V RV32I gate-level simulator

Exact gate-level partner for the completed RV32I machine in Spec 07a/07a2.
The topology contains 525,505 clocked D flip-flops: 524,288 memory bits,
1,024 GPR bits, 32 PC bits, 160 bits across five M-mode CSRs, and one halt
bit. Installed program origin and length are validated lifecycle metadata, not
architectural storage.

The crate has an independent RV32I+CSR/MRET decoder and execution engine.
Repository gates implement Boolean logic, decode predicates, ripple arithmetic,
signed and unsigned comparisons, barrel shifts, addresses, branches, CSR
updates, traps, and returns. Its checked state, trace, result, and fault types
are shared with `riscv-simulator`.

Verification covers exact topology, lifecycle and transactional faults, the
complete documented instruction surface, and all 256 reproducible Python
full-state vectors in trace/state lockstep with the functional Rust oracle.
Strict formatting, Clippy, and rustdoc pass; package line coverage is 97.96%
(769/785).

Run the package checks from the Rust workspace:

```sh
cargo test -p riscv-gatelevel
cargo clippy -p riscv-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p riscv-gatelevel --no-deps
```
