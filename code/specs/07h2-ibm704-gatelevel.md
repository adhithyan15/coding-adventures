# Spec 07h2 — IBM 704 Gate-Level Simulator

**Layer**: 07h2

**Architecture**: IBM 704

**Year**: 1954

**Rust package**: `ibm704-gatelevel`

**Behavioral oracle**: `ibm704-simulator`

**Gate dependencies**: `logic-gates`, `arithmetic`

## Purpose and boundary

This package is the gate-level companion to Spec 07h. It preserves the
functional simulator's IBM 704 v1 lifecycle, instruction behavior, state, and
trace contract while routing persistent state and architectural arithmetic
through the repository's Rust digital-logic primitives.

It is an educational ISA-level netlist. Host control flow may sequence fetch,
memory-line selection, one-hot control outputs, clock edges, and trace text. It
must not calculate an architectural data result with host integer or floating
arithmetic. Host conversions are permitted at the API boundary for program
transport and owned state observation.

## Persistent topology

Every persistent bit is a master-slave D flip-flop from
`logic_gates::sequential`. Bit vectors are least-significant-bit first.

| Component | Width | Flip-flops |
|---|---:|---:|
| Core memory | 32,768 × 36 | 1,179,648 |
| AC sign, Q, P, magnitude | 1 + 1 + 1 + 35 | 38 |
| MQ | 36 | 36 |
| Index A/B/C | 3 × 15 | 45 |
| Program counter | 15 | 15 |
| Halt, overflow, divide-check, MQ-overflow | 4 × 1 | 4 |
| **Total** | | **1,179,786** |

`flip_flop_count()` is exact and scales with `with_memory_words`. The stable
educational `gate_count()` estimate is six gates per persistent flip-flop plus
73,440 combinational/control gates for decoding, memory selection, adders,
multiplication, division, floating normalization, and clock sequencing. It is
not a transistor count, timing model, or count of invoked Rust gate functions.

## Decode and address paths

Instruction bits 33–35 first enter a three-input gate decoder. Prefixes whose
low two bits are both zero select Type B; all others enter the one-hot Type A
decoder. Type B bits 24–35 are compared through XNOR/AND compositions against
the 37 defined opcodes. An unmatched prefix or opcode halts the machine and
returns a typed error.

Tags select index registers by gating each register bit and ORing the selected
lines. The resulting index value enters a 15-bit complement-plus-carry ripple
subtractor with the instruction address. PC increment and Type A index changes
use 15-bit ripple add/subtract networks and wrap modulo 2^15.

## Integer datapaths

- CLA/CAL/LDQ and the store/exchange family route word or register wires into
  destination flip-flops. CAL maps the memory sign bit into AC P.
- ADD/SUB/ADM compare magnitudes with gate networks, choose the larger
  magnitude, and ripple add or subtract 35 bits. A same-sign carry sets AC P
  and the overflow trigger.
- MPY forms 35 AND-selected shifted partial products and accumulates them with
  70-bit ripple adders. AC receives the upper magnitude; MQ receives the lower
  magnitude and product sign.
- DVP/DVH concatenate AC magnitude and MQ magnitude into a 70-bit dividend.
  Seventy restoring stages shift, compare, subtract, and emit quotient bits.
  Zero or non-larger divisors set divide check; DVH also halts after advancing.
- Transfer conditions use zero reduction, sign, and trigger gates. TOV clears
  overflow only when taken; TNO clears it on both paths, matching Spec 07h.

## Floating datapaths

IBM floating words contain sign, eight-bit excess-128 characteristic, and a
27-bit fraction. The Spec 07h oracle defines operations using binary64
intermediates. Because every input has at most 27 significant bits, the gate
model reproduces that behavior using exact dynamic bit vectors followed by a
53-bit round-to-nearest-even stage:

- FAD/FSB align binary scales, gate-select signs, ripple add/subtract, round to
  53 bits, then normalize and round to the IBM 27-bit fraction.
- FMP forms AND-selected partial products, rounds to 53 bits, and converts to
  IBM format.
- FDH/FDP use restoring division with guard and sticky information to form the
  rounded 53-bit quotient. The remainder follows the oracle's rounded
  quotient-times-divisor and dividend-minus-product sequence before IBM
conversion. A zero divisor sets divide check; FDH also halts.

Scales are fixed signed ten-bit two's-complement vectors. Scale comparison and
arithmetic use gates and ripple adders; 384-bit magnitude alignment and
normalization use comparator-controlled staged barrel shifters. A gate priority
encoder produces the normalization count. Host collection lengths and dynamic
host shifts therefore cannot influence an architectural floating result.

No host `f32` or `f64` operation participates in instruction execution. Public
conversion helpers remain boundary utilities for tests and callers.

## Public contract and failure behavior

`IBM704GateLevel` exposes construction, configurable memory, reset, canonical
transport and word loading, checked memory access, single-step, bounded run,
reset/load/execute, owned snapshots, and topology metrics. Canonical programs
use `ibm704-encoder`'s strict five-byte big-endian transport.

Length and load bounds are validated before decoding or slicing. Guest input
cannot select memory outside the configured store. Unknown decodes and failing
instruction accesses halt before returning a typed `IBM704Error`. Every run
has a mandatory `max_steps` bound; trace allocation is therefore caller
bounded.

## Conformance

Tests must compare complete architectural state and traces against
`ibm704-simulator` before and after every clock for:

- all 37 Type B opcodes and five Type A prefixes;
- all eight tag combinations and OR-then-subtract addressing;
- sign-magnitude zero, negative zero, carry overflow, multiply/divide, and
  transfer-trigger edge cases;
- canonical transport and memory/error boundaries;
- floating zero-divide, round trips, polynomial execution, and at least 512
  deterministic seeded floating vectors;
- FORTRAN-style loops and LISP car/cdr field programs;
- exact flip-flop counts and stable educational gate estimates.

The package must pass Rustfmt, tests, Clippy with warnings denied, rustdoc with
warnings denied, its `BUILD` recipe, and the affected-package repository build.
