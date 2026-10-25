# Spec 07aa2 — DEC PDP-8/E Gate-Level Simulator

**Architecture:** DEC PDP-8/E base processor

**Rust package:** `pdp8-gatelevel`

**Behavioral oracle:** `pdp8-simulator`

**Gate dependencies:** `logic-gates`, `arithmetic`

## Purpose and fidelity boundary

This package implements the complete Spec 07aa base processor with repository
digital-logic primitives. It is an educational ISA-visible gate topology, not
a timing-accurate netlist of DEC's M8300-series boards. The 1973 DEC processor
maintenance manual is the primary source for processor flow, memory transfer,
I/O pulses, and the major hardware boundaries.

Host control may sequence clocks, select a completed 12-bit memory address,
format traces, and carry installed-program metadata. It must not calculate an
architectural Boolean, arithmetic, skip, rotate, address, or PC result.

## Persistent topology

| Component | Flip-flops |
|---|---:|
| 4,096 × 12-bit core words | 49,152 |
| AC, PC, MQ, switch register | 48 |
| Link, interrupt enable/request/delay, halt | 5 |
| **Total** | **49,205** |

`flip_flop_count()` reports the exact architectural storage count. Lifecycle
origin and length are host metadata, as in the functional oracle. The stable
`gate_count()` educational estimate is six primitive gates per flip-flop plus
24,000 gates for decode, selection, address, ALU, skip, rotate, interrupt, and
clock networks. It is not a die transistor count.

## Combinational and sequential paths

- Three opcode wires drive an eight-line NOT/AND decoder.
- Current-page selection uses 2:1 gate multiplexers. Indirect and auto-index
  reads select a core word only after the 12-bit address network settles.
- Repository ripple-carry adders implement PC movement, TAD, ISZ, IAC,
  auto-index, JMS, and skip increments. TAD carry toggles link through XOR.
- AND, OPR complements, zero/sign/link predicates, reverse skip sense, and OSR
  use Boolean gates. RAR/RTR/RAL/RTL and BSW are explicit wire permutations.
- Every persistent update is clocked through D flip-flops.

Unsupported EAE/extended-flag operations are decoded before any rising edge,
so their typed errors preserve the exact pre-fetch state. Tests compare full
memory, registers, lifecycle metadata, trace, IOT event, errors, and interrupt
cycles with `pdp8-simulator`.
