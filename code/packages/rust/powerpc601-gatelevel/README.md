# PowerPC 601 gate-level simulator

This crate is the Rust gate-level partner for the PowerPC 601 educational
machine in Spec 07u. It implements the complete functional integer surface
while clocking every persistent architectural bit through the repository's
master/slave D-flip-flop primitive.

The exact topology is 525,473 DFFs: 524,288 memory bits, 1,024 GPR bits, 160
bits for LR/CTR/XER/CR/CIA, and one halt bit. Installed-program bounds are
validated lifecycle metadata rather than persistent circuit state.

Decode is fixed-field combinational logic. Arithmetic, carries, comparisons,
Boolean operations, shifts, count-leading-zero, effective addresses, branch
updates, 32-round multiplication, and fixed-round signed/unsigned division use
repository gate and ripple-carry networks. Successful transitions clock the
DFF state; typed instruction faults leave every DFF unchanged.

The checked API shares `PowerPcState`, `PowerPcError`, `StepTrace`, and
`ExecutionResult` with `powerpc601-simulator`, including exact snapshots,
validated restore, origin-aware loading, checked direct access, atomic steps,
and transactional bounded runs.

```bash
cargo test -p coding-adventures-powerpc601-gatelevel
cargo clippy -p coding-adventures-powerpc601-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-powerpc601-gatelevel --no-deps
```

The aggregate differential replays all 239 reproducible Python full-state
vectors from the functional package. The normative completion contract is
Spec 07u2.
