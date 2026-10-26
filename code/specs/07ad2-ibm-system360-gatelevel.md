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

### Branch predicate authority

Every modeled branch clocks the instruction address from a gate-derived select
wire. `BCR` and `BC` select the condition-code mask bit with a gate mux tree.
`BALR` suppresses a zero target-register field with an OR reduction, while
`BCTR` and `BCT` combine that field predicate where applicable with a gate
zero reduction of the decremented count. `BXH` and `BXLE` consume the
gate-comparator condition code and the one-hot opcode decode to distinguish
high from not-high. The settled select wire is then applied by the 24/32-bit
gate mux; host equality, ordering, Boolean short-circuit operators, and dynamic
mask shifts must not reconstruct any architectural branch decision.

Completion evidence exhausts every four-bit branch mask, both condition-code
bits, every target-register field, zero/nonzero count boundaries, and both
index-branch opcodes, with a structural guard over the production executor.

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

The accepted implementation passes eleven Rust differential suites and three
gate-integrity suites, including every first-byte classification, every
supported hexadecimal-floating order, exhaustive branch predicate truth
tables, both architectural branch paths, overlap semantics, lifecycle, typed
atomic faults, and token-normalized exact-body structural checks over the full
branch-select, fixed-width-adder, signed-comparator, and top-level dispatch
dependency chain, including the exact four-byte router, RR/RX/RS dispatcher
shells, complete validation router, and branch-validator shell, with hostile
helper-rewrite, dead-dispatch, expectation-shadowing, oracle-substitution, and
post-dispatch-overwrite fixtures. The complete public `step`, `run`, and
`execute` bridge is exact-locked so only the independently computed,
validated, oracle-compared gate result reaches the DFF commit path.
The crate root must bind `executor` directly to the reviewed file-backed
module; attributed paths, aliases, duplicate routing modules, and inline
oracle-backed replacements are structural-test failures. Module identity is
resolved from parsed Rust syntax and only exact top-level external declarations,
so a literal declaration discarded inside macro input cannot authorize a
macro-generated live `executor` or `gates` module, and delimiter-shaped
character literals cannot counterfeit top-level scope.
Rustfmt, strict Clippy, rustdoc, lesson validation, and the
package BUILD are green. Package line coverage is 93.01% (1,662/1,787).
