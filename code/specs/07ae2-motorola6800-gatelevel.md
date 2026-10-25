# Spec 07ae2 — Motorola MC6800 Gate-Level Simulator

**Architecture:** Motorola MC6800

**Rust package:** `motorola6800-gatelevel`

**Behavioral oracle:** `motorola6800-simulator`

**Gate dependencies:** `logic-gates`, `arithmetic`

## Purpose and fidelity boundary

This package is the gate-level partner to Spec 07ae. It implements the entire
checked base-MC6800 instruction and lifecycle boundary with repository digital
logic primitives. It models the documented architecture rather than the
physical transistor layout, clock phases, or timing of one manufactured die.

Host control may sequence clock edges, retain installed-program metadata,
index already-selected memory, and format traces. Architectural state is held
in simulated D flip-flops. Opcode selection, effective-address addition,
arithmetic, comparisons, condition-code selection, Boolean operations, shifts,
branch predicates, stack-pointer movement, and data selection flow through
explicit gate networks. Host integers transport settled bit vectors only.

External IRQ/NMI pins, devices, buses, cycle timing, package pins, analog delay,
and transistor technology are outside this CPU cell. They belong to later
SoC/interconnect and electrical-composition layers.

## Persistent topology

| Component | Flip-flops |
|---|---:|
| 64 KiB byte-addressed memory | 524,288 |
| Accumulators A and B | 16 |
| X, stack pointer, and program counter | 48 |
| Condition-code register | 8 |
| Wait/halt latch | 1 |
| **Total** | **524,361** |

Installed origin and length are checked lifecycle metadata rather than
architectural storage. `gate_count()` is a stable educational estimate of six
primitive gates per master-slave DFF plus a documented combinational allowance;
it is not a transistor count.

## Execution and fault contract

The public state, trace, result, and error types are shared with the functional
simulator. All 198 documented first-byte encodings, seven addressing modes,
condition-code rules, big-endian 16-bit transfers, descending stack, subroutine
and interrupt frames, SWI/RTI, and WAI are implemented by the independent gate
transition engine.

Each step first obtains a checked trace or typed fault from the functional
oracle. A successful trace drives the gate engine, whose complete settled state
must match the oracle before commit. Any mismatch fails closed. Unknown and
truncated instructions, invalid state, fetch outside the installed image,
post-WAI execution, and exhausted bounded runs preserve every entry-state bit.

## Completion gate

Completion requires:

1. exact topology and complete-state transport tests;
2. full-state lockstep for every first-byte classification and documented
   addressing/operation family;
3. both outcomes of every conditional branch plus stack, call, SWI/RTI, WAI,
   BCD, big-endian, wrap, and read-modify-write witnesses;
4. typed atomic lifecycle and transactional-run failures;
5. strict Rustfmt, Clippy with warnings denied, rustdoc with warnings denied,
   package BUILD, lesson validation, security review, and at least 80% line
   coverage before publication.

The accepted implementation passes eight Rust differential suites. Rustfmt,
strict Clippy, rustdoc, lesson validation, and the package BUILD are green.
Package line coverage is 95.70% (801/837).
