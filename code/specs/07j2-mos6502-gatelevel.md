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

### HALT storage authority

The one-bit `StateRegister` Q output is the sole architectural HALT authority.
Every observation, snapshot, reset, load, restore, instruction-step guard,
bounded-run guard/result, BRK transition, IRQ/NMI helper, and transactional
rollback must read or clock that Q bit. A host Boolean may remain only as a
source-compatible observation mirror. Deliberately corrupting that mirror in
either direction must not change execution or returned state, and the next
simulator-owned HALT transition must repair it from Q.

No lifecycle or execution path may assign the packed HALT bit directly after
construction. All changes pass through one reviewed helper that clocks the
one-bit `StateRegister`; ordinary instruction commit and interrupt entry may
only synchronize an observer from Q. The physical-storage inventory must be
derived independently from the lengths of the owned memory, register, flag,
HALT, and port containers and must total exactly 528,184 D flip-flops.

Structural conformance tests exact-lock the complete HALT authority chain and
must reject compile-valid hostile variants that make the observer decisive,
write HALT Q directly, hide either bypass behind a helper, retain a dead clock
anchor, or overwrite a clocked result after commit. The hostile variants must
be type-checked by Rust, not merely accepted by a parser.

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
4. two-way observer/Q divergence, reset/load/restore, BRK, IRQ/NMI,
   checked-step and transactional-run rollback tests that prove Q authority;
5. exact source-chain locks plus compile-valid hostile observer, direct-Q,
   helper-indirection, dead-anchor, and post-commit bypass fixtures;
6. at least 80% core line coverage; and
7. clean formatting, Clippy with warnings denied, and rustdoc with warnings
   denied.

The completion audit records 81 unit, 5 integration, and 7 documentation tests
with 96.60% core line coverage (994/1,029).

## Dependencies

| Package | Responsibility |
|---|---|
| `logic-gates` | combinational gates and D flip-flops |
| `arithmetic` | ripple-adder primitives |
| `mos6502-simulator` | shared public contracts and functional oracle |
