# x86-64 gate-level simulator

This crate is the Rust gate-level partner for the educational x86-64 integer
machine in Spec 07w. It owns the complete functional lifecycle while clocking
every persistent architectural bit through the repository master/slave DFF.

The exact topology is 525,382 DFFs: 524,288 memory bits, 1,024 GPR bits, 64
RIP bits, the five specified RFLAGS bits, and one halt bit. Installed-program
origin and length are validated lifecycle metadata rather than persistent
circuit state.

The decoder is shared only as the instruction bit-field boundary. The gate
package owns instruction execution. Repository gates and ripple-carry adders
implement arithmetic flags, Boolean operations, barrel shifts and rotates,
effective addresses, all 16 conditions, a fixed 64-round partial-product
multiplier, and a fixed 128-round restoring divider. Product truncation flags
and signed/unsigned quotient-fit faults are qualified by reductions over every
discarded result wire rather than host-wide integer comparisons. Successful
transitions clock the DFF state; typed instruction faults leave every DFF
unchanged.

The checked API shares `X86State`, `X86Error`, `X86StepTrace`, and
`X86ExecutionResult` with `x86-simulator`, including exact snapshots,
validated restore, origin-aware loading, wrapping direct byte access, atomic
steps, and transactional bounded runs.

```bash
cargo test -p coding-adventures-x86-64-gatelevel
cargo clippy -p coding-adventures-x86-64-gatelevel --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding-adventures-x86-64-gatelevel --no-deps
```

The aggregate differential replays all 262 reproducible Python full-state
vectors from the functional package. Focused multiply/divide differentials
also pin every signed/unsigned 64-bit fit boundary and atomic overflow fault.
Parsed-syntax contracts require unique reviewed top-level qualifier and
multiply/divide entry functions, a direct `exec_one` instruction dispatch,
and exact instruction consumers. Adversarial fixtures prove that helper
indirection, shadowed reductions, dead anchors, iterator shortcuts, direct
high-word predicates, inactive `cfg` decoys, and later active duplicates
invalidate the contract. The normative completion contract is Spec 07w2.
Standard all-target coverage is measured with:

```bash
cargo llvm-cov --manifest-path code/packages/rust/Cargo.toml \
  -p coding-adventures-x86-64-gatelevel --all-targets --summary-only
```

The completed package reports 88.40% line coverage (1,357/1,535).
