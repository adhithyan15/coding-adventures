# Spec 07ad2 — IBM System/360 Gate-Level Simulator

**Architecture:** IBM System/360 base processor

**Rust package:** `ibm-system360-gatelevel`

**Behavioral oracle:** `ibm-system360-simulator`

**Gate dependencies:** `logic-gates`, `arithmetic`

## Purpose and fidelity boundary

This package is the gate-level partner to Spec 07ad. It implements that
specification's complete checked fixed-point, branch, shift, decimal-character,
storage-to-storage, and hexadecimal-floating instruction boundary with
repository digital-logic primitives. It models the architecture shared by the
System/360 family rather than one model's physical SLT-card layout or timing.

Host control may sequence clock edges, retain installed-program metadata,
index already-selected storage, and format traces. Architectural state must be
held in simulated D flip-flops. Opcode selection, effective-address addition,
fixed-point arithmetic and comparisons, condition-code selection, Boolean
operations, shifts, register-pair multiplication/division, and data selection
must flow through explicit gate networks. Hexadecimal floating-point uses
fixed-width sign/exponent/fraction networks; host integers may only transport
the settled bit vectors.

Channel I/O, interruption controls, protection keys, interval timers, model-
specific microcode, analog delay, package pins, and storage technology are
outside this CPU cell. They belong to later SoC/interconnect and lower-level
electrical composition.

## Persistent topology

| Component | Flip-flops |
|---|---:|
| 1 MiB byte-addressed main storage | 8,388,608 |
| 16 × 32-bit general registers | 512 |
| Four 64-bit floating registers (architectural FPR 0, 2, 4, 6) | 256 |
| 24-bit instruction address | 24 |
| Two-bit condition code | 2 |
| Teaching halt latch | 1 |
| **Total** | **8,389,403** |

`flip_flop_count()` reports that exact architectural storage count. Installed
origin and length are checked lifecycle metadata, matching the functional
oracle. `gate_count()` is a stable educational estimate: six primitive gates
per master-slave D flip-flop plus a documented combinational allowance for
decode, selection, arithmetic, shifts, floating point, and storage controls.
It is not a transistor or SLT-module count.

## Lifecycle and fault contract

Reset clocks zero through every state element. Program loading, complete-state
restore, direct register/storage writes, instruction transitions, condition-
code updates, and halt all pass through repository sequential primitives.
Program bytes and scalar storage values use System/360 big-endian order.

The public state, trace, result, and error contract is shared with
`ibm-system360-simulator`. Every step is fail closed: malformed, unsupported,
misaligned, out-of-range, overflow, divide, specification, and post-halt faults
preserve every entry-state bit. A bounded run either returns all traces and the
complete final state or restores the complete entry snapshot.

## Completion gate

Completion requires:

1. exact topology and lifecycle tests, including all DFF-backed state banks;
2. functional lockstep across every RR, RX, RS, SI, SS, and floating-point
   classification implemented by Spec 07ad;
3. reserved-neighbor, malformed, alignment, range, arithmetic, divide,
   register-pair, restore, post-halt, and bounded-run atomicity witnesses;
4. exact raw bytes, width, mnemonic, description, complete state, and error
   parity with the functional oracle;
5. strict Rustfmt, Clippy with warnings denied, rustdoc with warnings denied,
   affected-package BUILD checks, security review, and at least 80% package
   line coverage before publication.

The accepted implementation passes eleven Rust differential suites, including
every first-byte classification, every supported hexadecimal-floating order,
both branch paths, overlap semantics, lifecycle, and typed atomic faults.
Rustfmt, strict Clippy, rustdoc, lesson validation, and the package BUILD are
green. Package line coverage is 92.86% (1,626/1,751).
