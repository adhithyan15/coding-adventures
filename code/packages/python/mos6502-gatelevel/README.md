# coding-adventures-mos6502-gatelevel

**Layer 07j2** — MOS 6502 gate-level simulator

A fully gate-level behavioral simulation of the MOS Technology 6502 (NMOS)
microprocessor. Every data-path operation — arithmetic, logical, shifts,
rotates — routes through AND, OR, XOR, NOT, and ripple_carry_adder gate
primitives from the `logic-gates` and `arithmetic` packages. No Python integer
arithmetic appears on the execution path.

## What it is

The [MOS 6502](https://en.wikipedia.org/wiki/MOS_Technology_6502) (1975) was
one of the most influential CPUs ever made. It powered the Apple I, Apple II,
Atari 2600, Commodore 64, NES, and the BBC Micro. At ~3,510 transistors
(per the Visual6502 project), it achieved remarkable capability from very few
gates.

This package implements the **gate-level behavioral model** of the 6502,
complementing the behavioral simulator at layer 07j. Both implement the same
`Simulator[MOS6502State]` protocol — running the same programs on both must
produce identical results.

## Architecture

```
src/mos6502_gatelevel/
├── bits.py          — int↔bits, add_8bit, add_16bit, invert_8bit, compute_zero
├── alu.py           — ALUResult6502 + all ALU operations (gate-level)
├── register_file.py — RegisterFile6502: A/X/Y/S/PC as bit arrays, flag register
├── decoder.py       — Decoder6502: opcode → (mnemonic, mode) + AND/NOT group decode
└── simulator.py     — MOS6502GateLevelSimulator implementing Simulator[MOS6502State]
```

## Usage

```python
from mos6502_gatelevel import MOS6502GateLevelSimulator

sim = MOS6502GateLevelSimulator()

# Simple addition
result = sim.execute(bytes([
    0xA9, 0x0A,   # LDA #10
    0x69, 0x05,   # ADC #5
    0x00,          # BRK (halt)
]))
assert result.final_state.a == 15

# Loop: sum 1..5 = 15
result = sim.execute(bytes([
    0xA2, 0x05,   # LDX #5   — counter
    0xA9, 0x00,   # LDA #0   — accumulator
    0x18,          # CLC
    0x65, 0x10,   # ADC $10  — add counter to A (via zero page)
    0x85, 0x10,   # STA $10  — unnecessary here, just for illustration
    0xA9, 0x00,   # ...
    # Use the cross-validation tests for full programs
]))
```

## Key 6502 hardware behaviors

### JMP indirect bug

```python
# JMP ($10FF) reads high byte from $1000, not $1100
sim._memory[0x10FF] = 0x20
sim._memory[0x1000] = 0x40  # Bug: this is the high byte
sim._memory[0x1100] = 0x30  # This is NOT used
# Target = 0x4020, not 0x3020
```

### SBC carry convention

The 6502 uses inverted borrow: **C=1 means no borrow** (normal subtract).
Always `SEC` before `SBC` for straight subtraction:

```python
result = sim.execute(bytes([
    0xA9, 0x0A,   # LDA #10
    0x38,          # SEC       (set carry = no borrow)
    0xE9, 0x03,   # SBC #3    (10 - 3 = 7)
    0x00,
]))
assert result.final_state.a == 7
```

### BCD mode

```python
result = sim.execute(bytes([
    0xF8,          # SED       (decimal mode on)
    0xA9, 0x09,   # LDA #$09
    0x18,          # CLC
    0x69, 0x01,   # ADC #$01  (BCD: 09 + 01 = 10)
    0x00,
]))
assert result.final_state.a == 0x10  # BCD 10
```

## Register file

| Register | Width | Notes |
|----------|-------|-------|
| A | 8-bit | Accumulator |
| X | 8-bit | Index register |
| Y | 8-bit | Index register |
| S | 8-bit | Stack pointer (effective address 0x0100 + S) |
| PC | 16-bit | Program counter |
| N,V,B,D,I,Z,C | 1-bit each | Processor status flags |

## Memory map

| Range | Purpose |
|-------|---------|
| 0x0000–0x00FF | Zero page (fast 2-byte instructions) |
| 0x0100–0x01FF | Stack (hardware-fixed page) |
| 0xFF00–0xFFEF | Simulated I/O (ports 0–239) |
| 0xFFFA/B | NMI vector |
| 0xFFFC/D | RESET vector |
| 0xFFFE/F | IRQ/BRK vector |

## Gate-level design

Every operation routes through primitives:

```
ADC: ripple_carry_adder(A_bits, B_bits, C_flag)
SBC: ripple_carry_adder(A_bits, NOT(B_bits), C_flag)
AND: [AND(a[i], b[i]) for i in range(8)]
ORA: [OR(a[i], b[i]) for i in range(8)]
EOR: [XOR(a[i], b[i]) for i in range(8)]
ASL: shift register with MSB tap to carry
LSR: shift register with LSB tap to carry
```

## Stack notes

- Stack occupies 0x0100–0x01FF (hardware-fixed page 1)
- S points to the **next empty slot** (grows downward)
- JSR pushes **PC−1** (high byte then low byte); RTS pops and adds 1
- PHA/PHP push single bytes; PLA/PLP pop them back

## Cross-validation

Running the same program on both simulators must produce identical state:

```python
from mos6502_gatelevel import MOS6502GateLevelSimulator
from mos6502_simulator import MOS6502Simulator

gate = MOS6502GateLevelSimulator()
behav = MOS6502Simulator()

prog = bytes([0xA9, 0x42, 0x00])  # LDA #0x42; BRK
g = gate.execute(prog).final_state
b = behav.execute(prog).final_state

assert g.a == b.a == 0x42
assert g.flag_z == b.flag_z == False
assert g.flag_n == b.flag_n == False
```

## Dependencies

| Package | Layer | Role |
|---------|-------|------|
| `coding-adventures-logic-gates` | 01 | AND, OR, XOR, NOT, register |
| `coding-adventures-arithmetic` | 02 | full_adder, ripple_carry_adder |
| `coding-adventures-simulator-protocol` | SIM00 | Simulator[T], ExecutionResult |
| `coding-adventures-mos6502-simulator` | 07j | MOS6502State, cross-validation |

## Layer position

```
07j2 — MOS 6502 gate-level (this package)
 ↓ uses
07j  — MOS 6502 behavioral (for MOS6502State and cross-validation)
04   — logic-gates, arithmetic
```
