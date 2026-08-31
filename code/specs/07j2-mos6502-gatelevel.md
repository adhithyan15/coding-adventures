# Spec 07j2 — MOS 6502 Gate-Level Simulator

**Layer:** 07j2

**Status:** Implemented

**Rust package:** `coding-adventures-mos6502-gatelevel`

**Functional oracle:** `mos6502-simulator`

## Purpose

Provide an educational instruction-level NMOS MOS 6502 whose stored machine
state is built from D flip-flops and whose datapath uses the repository's logic
and arithmetic gates. The package is not a transistor-accurate die simulation;
host values remain acceptable for instruction-control decisions, addresses,
and trace bookkeeping.

## Architectural contract

The machine has:

- 64 KiB of byte-addressable memory;
- 8-bit A, X, Y, and S registers and a 16-bit PC;
- stored N, V, B, D, I, Z, and C flags;
- a stored halt bit;
- 240 stored input and 240 stored output bytes; and
- memory-mapped I/O at `$FF00..=$FFEF`.

It implements all 151 official NMOS 6502 opcode encodings and the implied,
accumulator, immediate, zero-page, zero-page-X, zero-page-Y, absolute,
absolute-X, absolute-Y, indexed-indirect, indirect-indexed, indirect, and
relative addressing modes. Undefined encodings return a typed error.

Required NMOS behavior includes inverted-borrow SBC, decimal ADC/SBC flags,
zero-page wrapping, 16-bit address wrapping, stack wrapping within page one,
and the JMP indirect `$xxFF` page-wrap quirk. BRK uses the repository simulator
convention: it performs the modeled stack/status effects and halts.

## Gate and state contract

Arithmetic, bitwise logic, shifts, rotates, comparisons, address addition,
decimal correction, and zero detection must use logic-gate or ripple-adder
paths. Persistent state must use D flip-flops with this exact topology:

| Component | D flip-flops |
|---|---:|
| Memory | 524,288 |
| A, X, Y, S | 32 |
| PC | 16 |
| Flags | 7 |
| Halt | 1 |
| Input latches | 1,920 |
| Output latches | 1,920 |
| **Total** | **528,184** |

The public `FLIP_FLOP_COUNT` constant must equal that total.

## Lifecycle contract

The Rust API must provide deterministic reset, checked load, single-step,
bounded execution, port access, full owned snapshots, and restore. It shares
`Mos6502State`, `StepTrace`, `ExecutionResult`, and `Mos6502Error` with the
functional simulator.

- Invalid program sizes and port numbers return typed errors.
- Undefined opcodes and stepping a halted machine return typed errors.
- Failed load and step operations do not change architectural state.
- A run that encounters an error restores its complete pre-run state.
- Instruction traces own complete before and after states.
- IRQ and NMI helpers push the modeled state and load the standard vectors.

## Conformance requirements

Completion requires:

1. full-state differential execution against `mos6502-simulator` for every one
   of the 151 official opcode bytes;
2. identical typed, atomic rejection of all 105 undefined bytes;
3. exact topology, lifecycle, multi-instruction, stack, control-flow, decimal,
   memory, and I/O tests;
4. at least 80% core line coverage; and
5. clean formatting, Clippy with warnings denied, and rustdoc with warnings
   denied.

The completion audit records 81 unit, 5 integration, and 7 documentation tests
with 96.60% core line coverage (994/1,029).

## Dependencies

| Package | Responsibility |
|---|---|
| `logic-gates` | combinational gates and D flip-flops |
| `arithmetic` | ripple-adder primitives |
| `mos6502-simulator` | shared public contracts and functional oracle |
