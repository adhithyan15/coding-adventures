# Spec 07o2 — DEC PDP-11 Gate-Level Simulator

**Layer:** 07o2

**Architecture:** DEC PDP-11

**Year:** 1970

**Rust package:** `pdp11-gatelevel`

**Behavioral oracle:** `pdp11-simulator`

**Gate dependencies:** `logic-gates`, `arithmetic`

## Purpose and boundary

This package preserves the complete Spec 07o functional state, trace,
lifecycle, instruction, addressing, and typed-failure contracts while routing
persistent state and every architectural data result through repository
digital-logic primitives. It is an educational ISA-level netlist, not a
cycle-accurate reconstruction of a specific PDP-11 model.

Host control may sequence clock edges, select an already checked register or
memory byte, choose an asserted gate-decode line, convert canonical program
transport, and format traces. It must not calculate architectural arithmetic,
logic, flags, effective addresses, branches, stack updates, or PC results.

## Persistent topology

Vectors are least-significant-bit first. Every persistent bit is a simulated
master-slave D flip-flop.

| Component | Width | Flip-flops |
|---|---:|---:|
| Byte memory | 65,536 × 8 | 524,288 |
| R0–R7 | 8 × 16 | 128 |
| PSW | 16 | 16 |
| Halted | 1 | 1 |
| **Total** | | **524,433** |

`flip_flop_count()` reports the exact total. `gate_count()` reports a stable
educational estimate of six primitive gates per stored bit plus 80,000 gates
for instruction comparators, register/memory selection, byte/word ALUs, NZVC,
effective-address, branch, stack, and clock networks. It is not a transistor
count or die-area claim.

## Decode and datapaths

- Instruction wires are compared through XOR/NOT/AND networks against HALT,
  NOP, RTI, RTS, SOB, all 15 branches, JMP, JSR, 25 single-operand variants,
  and 12 double-operand variants. Unknown patterns fail atomically.
- Eight- and sixteen-bit boolean, ripple add/subtract, shift, rotate, byte
  swap, sign extension, and NZVC networks produce all ALU results and flags.
- All eight addressing modes use sixteen-bit gate add/subtract for byte/word
  autoincrement, deferred pointers, autodecrement, and indexed displacement.
  SP and PC always step by two; other byte operations step by one.
- Fetch, extension-word movement, signed branch displacement doubling, SOB,
  JSR/RTS/RTI stacks, and PC transfers remain sixteen-bit gate vectors.
- Host indices select memory only after the guest address vector is complete.
  Word alignment is checked before any rising edge.

## Public and failure contract

`Pdp11GateLevel` exposes reset/load/state, checked memory and register boundary
access, PSW injection, step/run/execute, topology metrics, and the functional
crate's shared constants, encoders, snapshots, traces, results, and typed
errors. Program capacity is checked before replacement allocation. Each step
is transactional: odd word access, illegal JMP/JSR modes, and unknown opcodes
restore complete pre-clock state. Trace growth is bounded by executed steps,
not preallocated from `max_steps`.

## Conformance

Tests compare full 64 KiB state and traces with `pdp11-simulator` after every
clock across all 59 mnemonic variants, eight source/destination modes,
byte/word/SP/PC side effects, NZVC edges, all branch conditions, calls,
interrupt returns, SOB loops, malformed programs, odd addresses, illegal
modes, unknown decode, and step bounds. They assert exact topology and at least
80% core line coverage, plus Rustfmt, strict Clippy/rustdoc, the package BUILD
recipe, and the repository affected-package build.
