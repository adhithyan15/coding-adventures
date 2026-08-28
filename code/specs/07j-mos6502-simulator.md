# 07j — MOS 6502 Functional Simulator

## Purpose

`mos6502-simulator` is the Rust behavioral reference for the original NMOS
MOS Technology 6502. It implements all 151 official encodings across the 13
addressing modes and provides the oracle for `mos6502-gatelevel`.

The model is instruction accurate, not cycle accurate. `BRK` is the repository's
documented halt sentinel: it performs the existing stack/status side effects and
then halts instead of entering an interrupt handler.

## Architectural state

The complete owned `Mos6502State` contains:

- A, X, Y, and S 8-bit registers;
- the 16-bit program counter;
- N, V, B, D, I, Z, and C status flags;
- halt state;
- the full 65,536-byte address space; and
- 240 input and 240 output latches.

The constructor keeps its historical size argument for source compatibility,
but the architecture always allocates the complete 16-bit address space.

## Memory and I/O

```text
$0000-$00FF  zero page
$0100-$01FF  hardware stack page
$0200-$FEFF  ordinary memory
$FF00-$FFEF  memory-mapped I/O
$FFF0-$FFFF  ordinary memory, including vector locations
```

Reads from `$FF00-$FFEF` return the corresponding input latch. Writes update
the corresponding output latch without changing backing RAM. Instruction and
operand fetches obey the same mapping. Public port indexes are checked against
`0..240`.

Sixteen-bit addresses wrap exactly as the NMOS processor does. That includes
zero-page pointer wrap, indexed address wrap, instruction fetch across `$FFFF`,
and the original indirect-JMP page bug: `JMP ($10FF)` reads its high target byte
from `$1000`, not `$1100`.

## Registers and flags

```text
P bit:  7  6  5  4  3  2  1  0
        N  V  1  B  D  I  Z  C
```

- N is the high bit of the relevant result.
- V follows signed overflow rules and BIT's operand bit 6.
- Bit 5 is always set in packed status values.
- B is set in the copy pushed by PHP/BRK.
- D enables NMOS BCD adjustment for ADC/SBC.
- I is set at power-on.
- Z indicates a zero result.
- C is carry for addition and not-borrow for subtraction.

In NMOS decimal mode, N, V, and Z reflect the pre-correction binary result;
only A and C use the BCD-corrected result.

## Addressing modes

| Mode | Bytes | Rule |
|------|------:|------|
| Implied / accumulator | 1 | Operand is implicit or A |
| Immediate | 2 | Next byte is the value |
| Zero page | 2 | `$00nn` |
| Zero page,X / Y | 2 | `(nn + index) & $FF` |
| Absolute | 3 | Little-endian 16-bit address |
| Absolute,X / Y | 3 | Absolute address plus index, wrapping at 16 bits |
| (Indirect,X) | 2 | Zero-page pointer selected after adding X |
| (Indirect),Y | 2 | Zero-page pointer followed by Y addition |
| Relative | 2 | Signed offset from the post-instruction PC |
| Absolute indirect | 3 | JMP pointer with the NMOS page-wrap bug |

## Instruction families

- Loads/stores: LDA, LDX, LDY, STA, STX, STY
- Transfers: TAX, TAY, TXA, TYA, TSX, TXS
- Stack: PHA, PLA, PHP, PLP
- Arithmetic: ADC, SBC, INC, INX, INY, DEC, DEX, DEY
- Logic/test: AND, ORA, EOR, BIT
- Shifts/rotates: ASL, LSR, ROL, ROR
- Compares: CMP, CPX, CPY
- Branches: BCC, BCS, BEQ, BNE, BPL, BMI, BVC, BVS
- Control: JMP, JSR, RTS, RTI, BRK
- Flags: CLC, SEC, CLD, SED, CLI, SEI, CLV
- Other: NOP

The remaining 105 first-byte values are undocumented/illegal for this model and
return `Mos6502Error::UnknownOpcode` without mutation.

## Lifecycle contract

- `load_program` and `load_program_at` reject images larger than 64 KiB
  atomically. A valid image may wrap through address zero.
- `step` rejects halted or undefined states atomically and returns raw bytes,
  mnemonic, and complete before/after state on success.
- `run` and `run_loaded_with_limit` are bounded and restore the complete entry
  state if any instruction fails.
- `snapshot`/`state` and `restore` transfer complete owned state.
- `reset` restores CPU and memory defaults while preserving external I/O latch
  configuration, matching the repository's execution convention.

## Completion contract

The Rust functional simulator is complete when:

1. all 256 first bytes classify as 151 official and 105 undefined;
2. every official encoding, from a non-trivial common state, matches the full
   state hash generated from the repository's Python reference;
3. undefined, oversized, halted, invalid-port, wraparound-fetch, and
   transactional-run boundaries are pinned by tests;
4. memory-mapped input and output behavior is observable in complete state;
5. the backend consumer passes against the checked API; and
6. unit, integration, documentation, strict Clippy, strict rustdoc, formatting,
   coverage, and package build checks pass.

## Example

```rust
use mos6502_simulator::Mos6502Simulator;

let mut simulator = Mos6502Simulator::new(65_536);
let result = simulator.run(&[0xA9, 42, 0x00]).unwrap();
assert!(result.halted);
assert_eq!(result.final_state.a, 42);
```
