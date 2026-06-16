# mos6502-gatelevel

Gate-level simulator for the **MOS Technology 6502** (1975) microprocessor.

Every arithmetic and logic operation routes through real gate primitives from the
`logic-gates` and `arithmetic` crates — no host integer arithmetic in the execution
path. Registers are modelled as D flip-flop arrays.

## Architecture

```
bits.rs        — integer ↔ LSB-first bit-vector helpers; 8-bit + 16-bit adders
alu.rs         — GateAlu6502: ADD/SUB/AND/OR/XOR/shifts/rotates through gate chains
decoder.rs     — AND/NOT/OR gate-level group decode + PLA lookup (151 opcodes)
registers.rs   — Register8, Register16, FlagRegister, RegisterFile6502
cpu.rs         — GateLevelCpu: fetch-decode-execute loop; full NMOS 6502 instruction set
```

## Quick start

```rust
use coding_adventures_mos6502_gatelevel::GateLevelCpu;

let mut cpu = GateLevelCpu::new();
// LDA #10 ; ADC #5 ; BRK
let (traces, state) = cpu.run(&[0xA9, 0x0A, 0x69, 0x05, 0x00], 100);
assert_eq!(state.a, 15);
assert!(!state.flag_c);
```

## Gate count estimate

| Component                | Gates |
|--------------------------|-------|
| 8-bit ALU (add/sub/log)  | ~97   |
| Registers (4×8-bit)      | 384   |
| PC (1×16-bit)            | 192   |
| 16-bit adder (PC/addr)   | 80    |
| Instruction decoder      | ~140  |
| Control + wiring         | ~200  |
| **Total**                | **~1,093** |

Real 6502: ~3,510 transistors (~878 gate equivalents).

## Key differences from Intel 8080

| Feature           | Intel 8080        | MOS 6502              |
|-------------------|-------------------|-----------------------|
| Flags             | S, Z, AC, P, CY   | N, V, B, D, I, Z, C   |
| Half-carry (AC)   | Yes               | No                    |
| SBC carry-in      | NOT(C) = borrow   | C directly (C=1 = no borrow) |
| BCD mode          | DAA instruction   | D flag (ADC/SBC auto-adjust) |
| Stack             | Memory-backed SP  | Page 1 (0x0100–0x01FF) |
| I/O               | IN/OUT ports      | Memory-mapped (0xFF00–0xFFEF) |

## NMOS hardware quirks implemented

1. **JMP ($xxFF) bug** — high byte fetched from `$xx00`, not `$xx01`
2. **BCD mode (NMOS)** — N/V/Z flags from binary result; only C from BCD correction
3. **SBC convention** — `A - M - (1-C)` = `A + NOT(M) + C`; C=1 means no borrow
4. **BRK** — pushes PC+2 and P with B=1; treated as halt in this simulator
5. **Overflow** — `V = XOR(carry_into_bit7, carry_out_of_bit7)` — single XOR gate

## Instruction coverage

All 151 official NMOS 6502 opcodes:

- **Load/Store**: LDA, LDX, LDY, STA, STX, STY
- **Register transfers**: TAX, TAY, TXA, TYA, TSX, TXS
- **Stack**: PHA, PLA, PHP, PLP
- **ALU**: ADC, SBC, AND, ORA, EOR, BIT (with BCD mode for ADC/SBC)
- **Shifts/Rotates**: ASL, LSR, ROL, ROR (accumulator and memory)
- **Increment/Decrement**: INC, DEC, INX, INY, DEX, DEY
- **Compare**: CMP, CPX, CPY
- **Branches**: BCC, BCS, BEQ, BNE, BPL, BMI, BVC, BVS (relative mode)
- **Jumps**: JMP (absolute + indirect), JSR, RTS, RTI
- **Flags**: CLC, SEC, CLD, SED, CLI, SEI, CLV
- **System**: BRK, NOP

## Addressing modes

IMM, ZP, ZPX, ZPY, ABS, ABX, ABY, INX, INY, IMP, ACC, REL, IND (13 modes)

## Package layout

Part of the `coding-adventures` gate-level simulator series:

| Layer | Package                     | Processor      | Year |
|-------|-----------------------------|----------------|------|
| 07d2  | `intel4004-gatelevel`       | Intel 4004     | 1971 |
| 07f2  | `intel8008-gatelevel`       | Intel 8008     | 1972 |
| 07i2  | `intel8080-gatelevel`       | Intel 8080     | 1974 |
| **07j2** | **`mos6502-gatelevel`** | **MOS 6502**   | **1975** |
