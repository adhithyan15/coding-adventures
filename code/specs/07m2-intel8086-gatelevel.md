# Spec 07m2 — Intel 8086 Gate-Level Simulator

## Contract

Layer 07m2 is the Rust gate-level partner of the complete Layer 07m Intel 8086
functional simulator. Both implementations expose identical architectural
state and instruction behavior. The gate implementation differs only in how
state and data paths are built: persistent bits are D flip-flops, and all
arithmetic/logic networks use `logic-gates` and `arithmetic` primitives.

The supported instruction surface is the complete 07m repository oracle:
ModRM register/memory operations and all 24 effective-address forms; segment,
REP/REPNE, and LOCK prefixes; data transfer; every arithmetic/logical group;
stack and near/far control flow; all conditional/loop branches; shifts and
rotates; unsigned/signed multiply/divide; BCD/ASCII adjustment; string
families; FLAGS control; IRET and the repository protocol's halt-on-INT
boundary; HLT/WAIT; and byte/word I/O.

## Persistent state

Every architectural bit is backed by D flip-flops:

| Component | Width/count | DFFs |
|---|---:|---:|
| Physical memory | 1,048,576 × 8 | 8,388,608 |
| AX/BX/CX/DX/SI/DI/SP/BP | 8 × 16 | 128 |
| CS/DS/SS/ES/IP | 5 × 16 | 80 |
| CF/PF/AF/ZF/SF/TF/IF/DF/OF | 9 × 1 | 9 |
| Halt | 1 × 1 | 1 |
| Input ports | 256 × 8 | 2,048 |
| Output ports | 256 × 8 | 2,048 |
| **Total** | | **8,392,922** |

Host integers may exist only as combinational input/output wires and trace
bookkeeping. At instruction boundaries, register, flag, halt, and port wires
are clocked into the listed DFFs. Memory writes clock exactly eight selected
cells. A byte cache mirrors the memory Q bus for efficient observation but is
not an additional architectural store.

## Gate networks

1. Addition and address changes use fixed ripple chains of full adders.
2. Subtraction uses bitwise NOT plus carry-in one. CF is the inverted final
   carry for borrow; AF uses the low-nibble subtract chain; OF uses the MSB
   carry relationship.
3. Logical functions use one primitive gate per result bit. ZF is a gate tree;
   PF is a low-byte XOR tree plus inversion.
4. Segment multiplication by sixteen is wiring. The shifted 16-bit segment and
   zero-extended offset feed a 20-stage ripple adder; the address bus discards
   carry above bit 19.
5. Shifts/rotates use fixed bit routing, including the carry input/output path.
6. Unsigned multiplication is a fixed 8- or 16-iteration partial-product
   network: AND gates select each shifted multiplicand and fixed-width adders
   accumulate it. Signed multiplication gate-computes magnitudes and result
   negation.
7. Unsigned division is a fixed 16- or 32-iteration restoring network with a
   one-bit-wider remainder, gate subtraction, and quotient-bit latches. Signed
   division uses gated two's-complement magnitudes and signs.
8. AAM and AAD reuse those divider/multiplier networks rather than host
   arithmetic.

## Lifecycle

The crate reuses `Intel8086State`, `StepTrace`, `ExecutionResult`, and
`Intel8086Error` from the functional package. Checked loads, restores, ports,
steps, and runs reject invalid input atomically. A successful trace contains
the complete raw prefix/operand byte sequence and full before/after state.
Bounded runs are transactional on instruction failure.

Legacy `load`, `step`, and `execute` entry points remain for source
compatibility; new code should use their checked counterparts.

## Verification

Acceptance requires:

- exact topology tests for all 8,392,922 DFFs;
- the functional package's 461 deterministic full-state vectors, including all
  256 first bytes, every dense group extension, every effective-address mode,
  and focused prefix/string/control/stack/I/O cases;
- exhaustive 65,536-pair 8-bit multiplication;
- signed-edge and seeded 16-bit multiply/divide vectors;
- atomic lifecycle tests;
- strict formatting, Clippy, and rustdoc; and
- at least 80% core line coverage, targeting 95%.
