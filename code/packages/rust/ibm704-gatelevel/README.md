# IBM 704 Gate-Level Simulator

An educational gate-level companion to `ibm704-simulator`. It implements the
complete 42-instruction Rust IBM 704 v1 surface while storing every persistent
architectural bit in simulated master-slave D flip-flops.

## Circuit model

- 32K × 36-bit core memory, a 38-bit AC (sign, Q, P, magnitude), 36-bit MQ,
  three 15-bit index registers, 15-bit PC, halt/overflow/divide-check/MQ
  overflow triggers: **1,179,786 exact flip-flops**.
- Type A prefixes and 12-bit Type B opcodes are selected through one-hot
  NOT/AND/XOR decoder networks.
- Effective-address subtraction, PC increments, sign-magnitude ADD/SUB/ADM,
  and index arithmetic use ripple-carry circuits from `arithmetic`.
- MPY uses 35 AND-selected partial products and ripple adders. DVP/DVH use a
  restoring shift/compare/subtract network.
- FAD/FSB/FMP/FDH/FDP use fixed 384-bit magnitude paths, signed ten-bit scale
  adders, gate priority encoders, comparator-controlled barrel shifters,
  restoring division, and round-to-nearest-even. A 53-bit intermediate stage
  reproduces the functional oracle's binary64 contract without host arithmetic
  selecting an exponent, shift, or instruction result.

The model is an ISA-level teaching netlist, not a timing-accurate or
vacuum-tube-accurate reconstruction. Host control flow sequences clock edges,
memory-line selection, and trace formatting; architectural results flow
through the digital networks.

## Instruction surface

The Type B family is HTR, HPR, NOP, CLA, CAL, ADD, SUB, ADM, STO, STZ, STQ,
LDQ, XCA, MPY, DVP, DVH, TRA, TZE, TNZ, TPL, TMI, TOV, TNO, TQO, TQP, LXA,
LXD, SXA, SXD, PAX, PDX, PXA, FAD, FSB, FMP, FDH, and FDP. Type A implements
TXI, TIX, TXH, TXL, and TNX. All eight index tags obey the IBM OR-then-subtract
effective-address rule.

## Public lifecycle

`IBM704GateLevel` provides `new`, `with_memory_words`, `reset`, `load`,
`load_words`, `read_word`, `write_word`, `step`, `run`, `execute`, and
`get_state`. Programs use the strict canonical five-byte big-endian transport
from `ibm704-encoder`. All runs require a caller-supplied step bound, bad
decodes and addresses fail closed with `IBM704Error`, and snapshots own their
memory.

`flip_flop_count()` reports exact instantiated persistent state.
`gate_count()` reports a stable educational topology estimate and must not be
interpreted as a transistor count or dynamic gate-call count.

## Verification

The integration suite runs every instruction family and all index tags in
lockstep with `ibm704-simulator`, comparing complete state and every trace at
each clock. It also covers canonical transport rejection, small-memory bounds,
unknown decodes, trigger behavior, FORTRAN-style loops, LISP cons fields,
floating programs, topology metrics, and 512 seeded public-machine floating
instruction vectors compared against the functional simulator's complete state
and traces.
Package-local Tarpaulin instrumentation covers 773 of 798 source lines
(96.9%).

```bash
bash code/packages/rust/ibm704-gatelevel/BUILD
```

See `code/specs/07h2-ibm704-gatelevel.md` for the normative model.
