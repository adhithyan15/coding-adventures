# Spec 07k2 — Zilog Z80 Gate-Level Simulator

**Layer:** 07k2

**Status:** Implemented

**Rust package:** `coding-adventures-z80-gatelevel`

**Functional oracle:** `z80-simulator`

## Purpose

Provide an educational instruction-level Zilog Z80 whose complete stored
machine state is built from D flip-flops and whose data path uses the
repository's logic and arithmetic gates. This is not a transistor-accurate die
simulation. Host values are acceptable for decode/control decisions, address
selection, and trace bookkeeping, but not as persistent architectural state.

## Architectural contract

The machine implements the repository's Spec 07k surface: all defined base,
CB, ED, DD, FD, DDCB, and FDCB forms represented by the Python and Rust
functional simulators. This includes both register banks, IX/IY displacement
forms, all transfer/compare/input/output block operations, I/R transfers,
nibble rotates, interrupt control, and the separate 256-byte input and output
port spaces. Genuinely unassigned prefixed bytes return a typed error.

The address space is exactly 64 KiB. PC, stack, direct-memory, indexed, block,
and vector accesses wrap through the 16-bit address space. The low seven bits
of R advance for every fetched instruction byte while bit seven is preserved.

## Gate and state contract

Arithmetic, bitwise logic, shifts, rotates, comparisons, flag generation,
decimal adjustment, and zero/parity detection use logic-gate or ripple-adder
paths. Persistent state uses this exact topology:

| Component | D flip-flops |
|---|---:|
| Memory | 524,288 |
| Main and alternate A/F/B/C/D/E/H/L banks | 128 |
| IX and IY | 32 |
| SP and PC | 32 |
| I and R | 16 |
| IFF1, IFF2, IM, and halt | 5 |
| Input latches | 2,048 |
| Output latches | 2,048 |
| **Total** | **528,597** |

The public `FLIP_FLOP_COUNT` constant equals this total.

## Lifecycle contract

The gate and functional simulators share `Z80State`, `StepTrace`,
`ExecutionResult`, and `Z80Error`. The gate API provides deterministic reset,
checked wrapping load, single-step, bounded execution, checked ports, complete
owned snapshot/restore, maskable interrupt delivery for modes 0/1/2, and NMI.

- Images larger than 64 KiB, invalid state memory, and invalid ports return
  typed errors.
- Undefined opcodes and halted steps return typed errors without state change.
- A run that encounters an error restores its complete pre-run state.
- Instruction traces own complete before and after state, including memory and
  both port banks.

## Conformance requirements

Completion requires:

1. a deterministic full-state differential for all 1,160 defined opcode
   vectors across the base and six prefixed spaces;
2. identical typed, atomic rejection of unassigned prefixed bytes;
3. exact topology, lifecycle, wrapping, port, interrupt, block, indexed, and
   multi-instruction tests;
4. at least 80% core line coverage; and
5. clean formatting, Clippy with warnings denied, and rustdoc with warnings
   denied.

The completion audit records 58 unit, 4 integration, and 8 documentation tests
with 97.64% core line coverage (1,489/1,525).

## Dependencies

| Package | Responsibility |
|---|---|
| `logic-gates` | combinational gates and D flip-flops |
| `arithmetic` | ripple-adder primitives |
| `z80-simulator` | shared contracts and functional/Python-oracle hashes |
