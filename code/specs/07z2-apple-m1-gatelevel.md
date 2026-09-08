# Layer 07z2 — Apple M1 Gate-Level Simulator

## Scope

This specification is the gate-level partner to Spec 07z and the checked Rust
`apple-m1-simulator` oracle. It implements the complete documented AArch64
integer, scalar IEEE-754, vector-memory, DUP, and NEON/AdvSIMD teaching surface.
It does not model analog timing, a physical M1 pipeline, caches, unified-memory
fabric, MMU, exception levels, interrupts, FPCR/FPSR, SVE, or package layout.

## Persistent topology

The machine contains exactly **530,565 D flip-flops**:

| Bank | Width/count | DFFs |
|---|---:|---:|
| Big-endian memory | 65,536 x 8 | 524,288 |
| X0-X31 GPR slots | 32 x 64 | 2,048 |
| Stack pointer | 64 | 64 |
| Program counter | 64 | 64 |
| NZCV | 4 | 4 |
| V0-V31 vector/FP registers | 32 x 128 | 4,096 |
| Halt | 1 | 1 |
| **Total** | | **530,565** |

XZR occupies a physical slot but always reads zero, discards writes, and must
be zero in restored snapshots. Installed origin and length are checked
lifecycle metadata. Reset, restore, installation, direct writes, instruction
transitions, stores, flags, vectors, and halt clock repository DFF primitives.

## Combinational execution

The completed independent Spec 07v2 AArch64 gate engine executes the complete
integer core. This package owns an independent strict Apple extension decoder
for scalar FP, FP load/store, NEON integer/FP, DUP, and FMLA. Reserved precision,
shape, opcode, size, and structural fields fail closed.

Repository Boolean and ripple-carry networks implement NEON integer lane
selection, masking, add, two's-complement subtract, shift-add multiply, and
broadcast wiring. Checked gate-backed storage and big-endian memory surround FP
operations. Host binary32/binary64 operations are permitted only as bounded
IEEE-754 value primitives; they do not replace decode, lane routing, checked
addressing, persistent state, integer lanes, or transactional control.

## Lifecycle and conformance

The package shares the functional state, typed errors, complete traces/results,
structured encoders, and checked restore/load/direct-access contract. A failed
step preserves every bit. A failed bounded run restores its entire entry state.
Fetch and FP data accesses are aligned and bounded; reserved encodings, invalid
state, post-halt steps, and exhaustion are typed failures.

Completion requires exact topology and lifecycle suites, functional lockstep,
all 360 reproducible Python Apple-specific full-state vectors, the complete
AArch64 gate consumer, strict Rustfmt/Clippy/rustdoc, and at least 80% line
coverage. The accepted package passes eleven Rust tests, all 360 transitions in
complete functional trace/state lockstep, the 109-test Python Apple M1 suite,
the 256-test Python AArch64 gate suite, and strict checks. Package line coverage
is 90.52% (621/686).
