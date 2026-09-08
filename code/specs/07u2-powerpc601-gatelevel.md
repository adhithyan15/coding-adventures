# Layer 07u2: PowerPC 601 gate-level simulator

## Scope

This specification defines the Rust gate-level completion boundary for the
educational PowerPC 601 machine. Spec 07u remains authoritative for instruction
and lifecycle semantics. The gate implementation produces the same complete
states, traces, results, and typed failures.

## Exact persistent topology

The machine contains exactly **525,473 D flip-flops**:

| State | Bits |
|---|---:|
| 64 KiB unified memory | 524,288 |
| 32 32-bit GPR storage words | 1,024 |
| 32-bit LR, CTR, XER, CR, and CIA | 160 |
| HALT latch | 1 |
| **Total** | **525,473** |

Installed-program origin and length are validated lifecycle metadata, not
persistent circuit state. `FLIP_FLOP_COUNT` publishes the exact total. Every
simulator-owned state transition reconstructs and clocks the repository
sequential primitive. Reset establishes the same stable-Q state as construction.

## Combinational networks

The instruction word is decoded into opcode, GPR selectors, immediate,
branch, extended-opcode, record, CR, and SPR fields. Repository gates and
ripple-carry adders implement:

- 32-bit add/subtract, carry, negation, and signed/unsigned comparisons;
- AND, OR, XOR, NAND, NOR, CR masks, and XER/CR updates;
- logical/arithmetic shifts and count-leading-zero;
- effective addresses, branch predicates, CTR/LR/CIA updates;
- a fixed 32-round partial-product multiplier; and
- fixed-round restoring signed and unsigned division.

Native integers are limited to package-boundary field extraction, memory
addresses, big-endian byte assembly, stable bit-vector conversion, and trace
bookkeeping. They are not substitutes for the listed datapaths.

## Lifecycle and atomicity

`PowerPc601GateSimulator` exposes the complete Spec 07u checked boundary:
exact owned state, validated restore, deterministic origin-aware loading,
checked register and byte/halfword/word access, atomic single steps, complete
traces, and transactional bounded runs. Illegal, truncated, misaligned,
division-by-zero, halted, and step-limit failures leave the full machine
unchanged. Successful instruction transitions clock all modified persistent
state through DFFs.

## Conformance

Completion requires:

1. exact-topology and gate-network tests;
2. lifecycle, direct-I/O, trace, workload, and atomic-failure suites;
3. differential equality for all 239 reproducible Python full-state vectors;
4. strict Rust formatting, Clippy, and rustdoc checks; and
5. at least 80% package line coverage.
