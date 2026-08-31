# Alpha AXP gate-level simulator

This crate is the Rust gate-level partner for the DEC Alpha AXP 21064
educational machine in Spec 07s. It implements the complete integer instruction
surface while retaining every persistent architectural bit in storage clocked
through the repository's master/slave D-flip-flop primitive.

The exact topology is 526,465 DFFs: 524,288 memory bits, 2,048 GPR bits, 128
PC/nPC bits, and one halt bit. Installed-program bounds are lifecycle metadata.
The zero register has physical storage for topology parity but reads as zero and
rejects architectural writes.

Arithmetic, comparisons, Boolean operations, shifts, scaled addressing, byte
manipulation, and multiplication use gate networks and ripple-carry adders.
The checked API shares `AlphaState`, `AlphaError`, `StepTrace`, and
`ExecutionResult` with `alpha-axp-simulator`, including complete snapshots,
atomic faults, and transactional bounded runs.

```bash
cargo test -p coding-adventures-alpha-axp-gatelevel
cargo clippy -p coding-adventures-alpha-axp-gatelevel --all-targets -- -D warnings
```

The aggregate differential replays all 624 reproducible Python full-state
vectors from the functional package. The normative completion contract is
Spec 07s2.
