# Spec 07ab2 — EDSAC Gate-Level Simulator

**Architecture:** Cambridge EDSAC (1949 order code)

**Rust package:** `edsac-gatelevel`

**Behavioral oracle:** `edsac-simulator`

**Gate dependencies:** `logic-gates`, `arithmetic`

## Purpose and fidelity boundary

This package is the digital-logic partner for Spec 07ab. It implements the
complete checked EDSAC order-code surface with repository Boolean gates,
ripple-carry arithmetic, and master/slave D flip-flops. It models the stable
instruction-boundary state of the 1949 machine; mercury delay-line propagation,
valve analogue behavior, Initial Orders assembly, and wall-clock timing remain
outside this layer.

Host control may sequence clock edges, own unbounded paper-tape and teleprinter
buffers, retain traces, and validate installed-program metadata. It must not
calculate an architectural Boolean, arithmetic, shift, branch, address, memory,
or sequence-control result.

## Persistent topology

Only fixed-width machine state is counted as flip-flops. The input tape,
teleprinter transcript, their host cursors, and installed origin/length are
peripheral or lifecycle metadata, but are still included in complete snapshots
and transactional rollback.

| Component | Flip-flops |
|---|---:|
| 512 even 18-bit store locations | 9,216 |
| 512 odd 17-bit store locations | 8,704 |
| 10-bit sequence control register | 10 |
| 17-bit order tank | 17 |
| 71-bit accumulator | 71 |
| 35-bit multiplier | 35 |
| 35-bit multiplicand | 35 |
| Multiplicand-length latch | 1 |
| Five-bit last-output register | 5 |
| Halt latch | 1 |
| **Total** | **18,095** |

The public `FLIP_FLOP_COUNT` constant and `flip_flop_count()` method report
that exact total. Public register snapshots retain Spec 07ab's 15-bit tank
slices; the accumulator's low four padding bits and each 35-bit operand's low
ten padding bits are hard-wired zero and do not create fictitious storage.

`gate_count()` is a stable educational estimate: six primitive gates per DFF
plus 36,000 combinational gates for order decoding, store selection, tank
routing, serial arithmetic, product/collate networks, shifts, I/O selection,
branching, and clock control. It is neither a valve count nor a physical die
or delay-line model.

## Combinational and sequential paths

- Five opcode wires feed a NOT/AND decoder with one line for every implemented
  order. Invalid or short-only/long mismatches are rejected before a rising
  edge.
- Ten address wires select the short store through a gate-built equality and
  multiplexer network. Long operands clear the low address wire and select the
  adjacent even/odd pair.
- Repository full adders form 15-bit ripple chains for accumulator addition,
  rounding, collation, sequence-control movement, and product accumulation.
- Negation, sign tests, E/G branch selection, I/O field replacement, T/U store
  selection, and halt are Boolean-gate networks.
- R/L orders are explicit tank-wire permutations repeated for the decoded
  pulse count. The multiplier network is built from AND partial products and
  ripple-carry reductions; host multiplication is not an architectural path.
- Every fixed-width persistent update clocks through repository D flip-flops.
  Host buffers and lifecycle metadata commit at the same instruction boundary.

## Lifecycle, traces, and differential acceptance

The public checked lifecycle mirrors Spec 07ab: clear construction and reset,
strict little-endian transport, checked cell and long-word access, complete
owned state/restore, paper-tape input, atomic `step`, transactional `run`, and
transactional `execute`/`execute_words`. Errors and successful traces use the
functional package's public types so callers can compare machines directly.

Tests lock the gate machine against `edsac-simulator` for every order family,
both legal operand lengths, short/long store packing, signed branches, serial
shifts, multiplication, collation, rounding, paper-tape and teleprinter I/O,
halt, complete snapshots, and every typed lifecycle/decode failure. Faults
must leave all DFFs and host metadata unchanged. Debug and release tests,
Clippy and rustdoc with warnings denied, the package BUILD recipe, and at least
80% line coverage must pass.
