# Motorola MC68020 functional simulator

A checked, bounded first-cell simulator for the Motorola MC68020 problem-state
architecture in [Spec 07at](../../../specs/07at-motorola-68020-simulator.md).
It owns complete 16 MiB state while preserving 32-bit wrapping address
arithmetic and exact installed-program lifecycle metadata.

The implemented boundary includes `MOVE.L`/`MOVEA.L` with compatible modes,
scaled brief and full extension words (including both memory-indirect orders),
byte/word/long branches, `RTD`, `LINK.L`, `EXTB.L`, `TRAPcc`, and a private
checked-MC68000 bridge for a bounded inherited register-only subset. Remaining
MC68020 instruction families are explicitly queued rather than approximated.

```rust
use motorola68020_simulator::Mc68020Simulator;

// MOVEQ #42,D0 ; TRAP #15
let mut cpu = Mc68020Simulator::new();
let result = cpu.execute(&[0x70, 0x2a, 0x4e, 0x4f], 0x1000, 2)?;
assert_eq!(result.final_state.d[0], 42);
assert!(result.halted);
# Ok::<(), motorola68020_simulator::Mc68020Error>(())
```

Every typed failure is transition-atomic. Successful steps report exact bytes,
width, mnemonic, PC transition, effect, and complete before/after state.

## Verification

The completion suite covers every claimed family, effective-address classes,
full-extension displacement/suppression/indirection and reserved encodings,
aliases, condition codes, stack banks, lifecycle, typed rollback,
self-modifying trace capture, unaligned data transfers, and inherited
differentials. All 18 tests,
strict Clippy, rustdoc, and the package BUILD are green. Measured line coverage
is 89.28% (641/718).
