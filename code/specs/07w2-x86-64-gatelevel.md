# Layer 07w2: x86-64 gate-level simulator

## Scope

This specification defines the Rust gate-level completion boundary for the
educational x86-64 integer machine. Spec 07w remains authoritative for
instruction and lifecycle semantics. The gate implementation produces the same
complete states, traces, results, and typed failures. Backend-runtime and SSE2
consumer behavior remains owned by `x86-simulator` and is outside this
persistent integer circuit boundary.

## Exact persistent topology

The machine contains exactly **525,382 D flip-flops**:

| State | Bits |
|---|---:|
| 64 KiB unified memory | 524,288 |
| Sixteen 64-bit GPR storage words | 1,024 |
| 64-bit RIP | 64 |
| CF, PF, ZF, SF, OF | 5 |
| HALT latch | 1 |
| **Total** | **525,382** |

Installed-program origin and length are validated lifecycle metadata, not
persistent circuit state. `FLIP_FLOP_COUNT` publishes the exact total. Every
simulator-owned state transition reconstructs and clocks the repository
sequential primitive. Reset establishes RSP `0xfff8` and otherwise the same
stable-Q state as construction.

## Combinational networks

The shared decoder establishes the accepted instruction fields but does not
execute instructions. The gate package owns the complete integer execution
path. Repository gates, multiplexers, and ripple-carry adders implement:

- 64-bit add, ADC, subtract, SBB, carry/borrow, overflow, parity, auxiliary,
  sign, and zero flags;
- AND, OR, XOR, TEST, NOT, NEG, compare, and increment/decrement;
- six-stage logical/arithmetic barrel shifts and rotations;
- base/index/scale/displacement effective-address addition;
- all sixteen Jcc/SETcc/CMOVcc predicates;
- a fixed 64-round partial-product multiplier producing 128 bits; and
- fixed 128-round restoring signed and unsigned division of RDX:RAX.

Native integers are limited to decoder field extraction, little-endian byte
assembly, stable bit-vector conversion, range decisions, and trace bookkeeping.
They are not substitutes for the listed datapaths.

## Lifecycle and atomicity

`X86GateSimulator` exposes the complete Spec 07w checked integer boundary:
exact owned state, validated restore, deterministic origin-aware loading,
checked register access, wrapping byte access, atomic single steps, complete
traces, and transactional bounded runs. Illegal, unknown, truncated,
division-error, halted, invalid-state, and step-limit failures leave the full
machine unchanged. Successful instruction transitions clock all modified
persistent state through DFFs.

The documented boundary retains only CF, PF, ZF, SF, and OF. AF may be
computed inside an instruction's combinational network but is deliberately not
persistent because Spec 07w does not expose it and no accepted BCD instruction
consumes it.

## Conformance

Completion requires:

1. exact-topology and gate-network tests;
2. lifecycle, direct-I/O, trace, workload, and atomic-failure suites;
3. differential equality for all 262 reproducible Python full-state vectors;
4. a direct manual-correct CQO suite because the Python oracle cannot decode
   `REX.W 99` correctly;
5. strict Rust formatting, Clippy, and rustdoc checks; and
6. at least 80% package line coverage.
