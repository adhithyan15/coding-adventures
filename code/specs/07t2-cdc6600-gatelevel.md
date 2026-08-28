# Spec 07t2 — CDC 6600 Gate-Level Simulator

**Layer:** 07t2

**Architecture:** CDC 6600

**Year:** 1964

**Rust package:** `cdc6600-gatelevel`

**Behavioral oracle:** `cdc6600-simulator`

**Gate dependencies:** `logic-gates`, `arithmetic`

## Purpose and boundary

This package is the gate-level companion to Spec 07t. It preserves the
functional simulator's complete lifecycle, state, trace, instruction, and
failure contracts while routing persistent state and architectural data paths
through the repository's Rust digital-logic primitives.

It is an educational ISA-level netlist, not a cycle-accurate reconstruction of
the physical 6600 scoreboard and functional units. Host control flow may select
checked memory lines, sequence clocks, choose an asserted one-hot control line,
and format traces. Host integers must not calculate an architectural arithmetic
or logical result. Conversion at program-transport and owned-state boundaries is
permitted.

## Persistent topology

All vectors are least-significant-bit first. Every persistent bit is a
master-slave D flip-flop, except B0, which is hardwired to zero.

| Component | Width | Flip-flops |
|---|---:|---:|
| Core memory | 4,096 × 60 | 245,760 |
| X0–X7 | 8 × 60 | 480 |
| A0–A7 | 8 × 18 | 144 |
| B1–B7 | 7 × 18 | 126 |
| P | 18 | 18 |
| Halted | 1 | 1 |
| **Total** | | **246,529** |

`flip_flop_count()` reports that exact total. `gate_count()` is the stable
educational estimate of six primitive gates per stored bit plus 40,000 gates
for opcode decoding, register and memory selection, ripple arithmetic, compare,
barrel-shift, partial-product multiply, branch, and clock networks. It is not a
transistor count or die-area claim.

## Gate paths

- Six opcode wires feed a 64-output NOT/AND one-hot decoder. Exactly one line is
  asserted for every parcel; unsupported lines return a typed error atomically.
- X, A, and B addition uses 60- or 18-bit ripple-carry adders. Subtraction uses
  gate inversion plus carry-in one. B0 never receives storage.
- Boolean instructions map each bit through AND, OR, XOR, or NOT gates.
- Signed comparisons use XNOR equality reduction and a sign-aware magnitude
  comparator composed from NOT/AND/OR gates.
- Logical shifts use six controlled mux stages for distances 1, 2, 4, 8, 16,
  and 32. Shift controls are Bk bits 0–5.
- Multiply forms 60 AND-selected shifted partial products and accumulates them
  with 60-bit ripple adders. Only the low 60 product bits are retained.
- Effective addresses, P+1/P+2, call links, and branch targets travel as 18-bit
  vectors. Zero tests and branch mux decisions use reductions and gates.
- Fetch and guest memory access are range-checked before any clock edge. Memory
  selection and trace bookkeeping may use host indices after that check.

## Public contract and failure behavior

`Cdc6600GateLevel` exposes construction, reset, canonical byte and parcel loads,
checked memory/register access, P selection, single-step, bounded run,
reset/load/execute, immutable owned snapshots, and topology metrics. State,
trace, execution-result, constants, encoders, and typed errors are shared with
the functional crate so callers can compare models directly.

Transport length, canonical parcel values, and capacity are checked before
allocation or mutation. Instruction fetch, long-pair fetch, next P, branch
target, and data-memory addresses are preflighted before any state write. Errors
therefore preserve complete state. A mandatory `max_steps` bounds execution and
trace growth.

## Conformance

Tests compare complete state and traces with `cdc6600-simulator` before and
after every clock for all 22 short and 14 long instructions, B0 suppression,
60/18-bit wrap, signed compare edges, all six barrel stages, widened multiply
vectors, taken/fall-through branches, subroutines, memory boundaries, malformed
transport, unknown opcodes, and step bounds. Tests also assert one-hot decode,
flip-flop persistence, exact topology metrics, and atomic errors.

The package must pass Rustfmt, tests, Clippy with warnings denied, rustdoc with
warnings denied, its `BUILD` recipe, the affected-package repository build, and
at least 80% line coverage.
