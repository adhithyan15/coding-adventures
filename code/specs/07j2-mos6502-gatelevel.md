# Spec 07j2 — MOS 6502 Gate-Level Simulator

**Layer:** 07j2 (gate-level CPU, depends on 07j behavioral and 04-level logic/arithmetic)
**Status:** Implemented
**Package:** `coding-adventures-mos6502-gatelevel`

---

## Purpose

Implement the MOS Technology 6502 (NMOS, 1975) as a gate-level behavioral simulator.
Every data-path operation routes through AND, OR, XOR, NOT, and ripple_carry_adder
primitives. No Python integer arithmetic appears on the execution path (helper
functions that convert between Python ints and gate-level bit-lists are permitted in
bits.py, but the ALU computations use gate primitives exclusively).

This serves as:

1. **Educational model** — demonstrates how the real silicon works at the gate level.
2. **Cross-validation target** — running the same programs on both the behavioral
   (07j) and gate-level (07j2) simulators must produce identical final state.
3. **Architecture study** — the 6502's minimal transistor count (~3,510 per Visual6502)
   stems from clever encoding and sharing of logic.

---

## Architecture Overview

### Register File

| Register | Width | Purpose |
|----------|-------|---------|
| A        | 8-bit | Accumulator — all arithmetic targets this |
| X        | 8-bit | Index register X — address offset |
| Y        | 8-bit | Index register Y — address offset |
| S        | 8-bit | Stack pointer — effective addr = 0x0100 + S |
| PC       | 16-bit | Program counter |
| P        | 8-bit | Processor status (flags) |

**Flags in P (MSB to LSB):**

```
Bit 7  N  Negative   — bit 7 of result
Bit 6  V  Overflow   — signed two's-complement overflow
Bit 5  -  (always 1, no physical flip-flop)
Bit 4  B  Break      — set only in the copy of P pushed by BRK/PHP
Bit 3  D  Decimal    — BCD mode for ADC/SBC
Bit 2  I  Interrupt disable
Bit 1  Z  Zero       — result was zero
Bit 0  C  Carry      — carry out (SBC: C=1 means no borrow)
```

### Memory Map

```
0x0000–0x00FF  Zero page     — 2-byte instruction format, fast
0x0100–0x01FF  Stack         — hardware-fixed; S is offset into this page
0x0200–0xFEFF  General RAM
0xFF00–0xFFEF  Simulated I/O — reads → input_ports, writes → output_ports
0xFFFA/B       NMI vector    — low/high byte of NMI handler
0xFFFC/D       RESET vector  — low/high byte of reset handler
0xFFFE/F       IRQ/BRK vector
```

### Transistor Count (per Visual6502)

~3,510 transistors total — significantly fewer than the Z80 (~8,500) or 8080 (~6,000).
This is achieved through:
- No alternate register bank
- No block-move instructions
- PLA-based decode (compact compared to random logic)
- Shared adder used for both arithmetic and address calculation

---

## Addressing Modes

| Mode | Syntax | Bytes | Description |
|------|--------|-------|-------------|
| Implied | IMP | 1 | No operand |
| Accumulator | A | 1 | Operand is A register |
| Immediate | #$nn | 2 | Literal operand byte follows opcode |
| Zero Page | $nn | 2 | 8-bit address in zero page |
| Zero Page,X | $nn,X | 2 | Zero page + X (wraps in page) |
| Zero Page,Y | $nn,Y | 2 | Zero page + Y (wraps in page) |
| Absolute | $nnnn | 3 | 16-bit address |
| Absolute,X | $nnnn,X | 3 | Absolute + X |
| Absolute,Y | $nnnn,Y | 3 | Absolute + Y |
| (Indirect,X) | ($nn,X) | 2 | Pre-indexed indirect via zero page |
| (Indirect),Y | ($nn),Y | 2 | Post-indexed indirect via zero page |
| Indirect | ($nnnn) | 3 | JMP only; reads 16-bit vector |
| Relative | $nn | 2 | Signed 8-bit PC offset for branches |

---

## Hardware Quirks

### JMP Indirect Bug

`JMP ($xxFF)` reads the high byte of the target from `$xx00` instead of `$xx01`.
The page is not crossed — the address wraps within the same page. Example:

```
mem[$10FF] = 0x20, mem[$1100] = 0x30, mem[$1000] = 0x40
JMP ($10FF) → PC = 0x4020  (NOT 0x3020)
```

### SBC Carry Convention

The 6502 uses "inverted borrow" for subtraction:
- `C = 1` means **no borrow** (subtract succeeded without underflow)
- `C = 0` means **borrow occurred**

SBC computes: `A + NOT(M) + C` using the ripple-carry adder.

### NMOS BCD Quirks

In decimal mode (D=1):
- ADC: N, V, Z flags reflect the *binary* result before BCD correction
- SBC: N, V, Z flags reflect the *binary* result before BCD correction
- Only C is computed correctly from the BCD result

The 65C02 CMOS version fixes these; NMOS does not.

### BRK vs IRQ

BRK (opcode 0x00) pushes `PC+2` and sets `B=1` in the pushed P copy.
IRQ hardware interrupt pushes current PC and sets `B=0`.
Both load the vector from 0xFFFE/F and set `I=1`.

In this simulator, BRK halts execution (sets `halted=True`) rather than
jumping through the interrupt vector, matching the convention of other simulators
in the stack.

### NMI

NMI pushes PC and P (with B=0), loads 0xFFFA/B, sets I=1.
NMI cannot be masked by the I flag.

---

## ALU Gate Structure

### Addition (ADC)

```
Bit 0: full_adder(A[0], B[0], C_in)  → (S[0], C[0])
Bit 1: full_adder(A[1], B[1], C[0])  → (S[1], C[1])
...
Bit 7: full_adder(A[7], B[7], C[6])  → (S[7], C[7] = carry_out)

Overflow (V flag) = XOR(carry_into_bit7, carry_out_of_bit7)
         = XOR(C[6], C[7])
```

### Subtraction (SBC)

```
A - B - (1 - C) = A + NOT(B) + C
```

The carry-in is the C flag directly (C=1 means no borrow).
Gate path: NOT gates on B → ripple_carry_adder(A, NOT_B, C).

### Logical Operations

Each uses 8 parallel gates (one per bit):
- AND: `result[i] = AND(A[i], B[i])`
- ORA: `result[i] = OR(A[i], B[i])`
- EOR: `result[i] = XOR(A[i], B[i])`

### Shifts and Rotates

```
ASL: {A[6:0], 0} → carry = A[7]    (shift left, 0 in)
LSR: {0, A[7:1]} → carry = A[0]    (shift right, 0 in)
ROL: {A[6:0], C} → carry = A[7]    (rotate left through carry)
ROR: {C, A[7:1]} → carry = A[0]    (rotate right through carry)
```

### Compare (CMP/CPX/CPY)

```
diff = reg - mem = reg + NOT(mem) + 1
N = diff[7]
Z = NOR(diff[7:0])
C = 1 if reg >= mem  (= carry out of subtractor)
V flag: NOT affected by compare
```

### BIT Test

```
N = mem[7]   (bit 7 of memory operand goes directly to N)
V = mem[6]   (bit 6 of memory operand goes directly to V)
Z = NOR(AND(A[7:0], mem[7:0]))  (AND of A and mem, then zero-test)
```

---

## Module Structure

```
src/mos6502_gatelevel/
├── __init__.py           Public API
├── py.typed              PEP 561 marker
├── bits.py               int↔bits, add_8bit, add_16bit, invert_8bit, compute_zero
├── alu.py                ALUResult6502 + all ALU operations
├── register_file.py      RegisterFile6502 (all registers as bit arrays)
├── decoder.py            Decode6502: opcode → (mnemonic, mode, operation)
└── simulator.py          MOS6502GateLevelSimulator implementing Simulator[MOS6502State]
```

---

## Instruction Set

All 151 official NMOS 6502 opcodes are implemented. Organized by group:

- **Load/Store**: LDA, LDX, LDY, STA, STX, STY
- **Transfer**: TAX, TAY, TXA, TYA, TSX, TXS
- **Stack**: PHA, PLA, PHP, PLP
- **Arithmetic**: ADC, SBC (with BCD mode)
- **Logical**: AND, ORA, EOR
- **Bit test**: BIT
- **Increment/Decrement**: INC, DEC, INX, INY, DEX, DEY
- **Shift/Rotate**: ASL, LSR, ROL, ROR (accumulator and memory)
- **Compare**: CMP, CPX, CPY
- **Branch**: BCC, BCS, BEQ, BNE, BPL, BMI, BVC, BVS
- **Jump/Call**: JMP (absolute + indirect), JSR, RTS, RTI
- **Flag ops**: CLC, SEC, CLD, SED, CLI, SEI, CLV
- **Interrupts**: BRK (halt), NMI, IRQ (interrupt method)
- **No-op**: NOP

---

## Cross-Validation Protocol

The `test_equivalence.py` suite runs 50+ programs on both:
1. `MOS6502Simulator` (behavioral, Layer 07j)
2. `MOS6502GateLevelSimulator` (gate-level, this package)

For each program, final state fields are compared:
- `a`, `x`, `y`, `s`, `pc`
- `flag_n`, `flag_v`, `flag_d`, `flag_i`, `flag_z`, `flag_c`
- `memory[0x00:0x10]` (zero page spot-check)
- `halted`

Programs cover: arithmetic, BCD, logical ops, shifts, branches, subroutines,
stack operations, indexed addressing, indirect addressing, and I/O ports.

---

## Dependencies

| Package | Layer | Used for |
|---------|-------|---------|
| `coding-adventures-logic-gates` | 01 | AND, OR, XOR, NOT, register, mux2, XOR_N, AND_N, OR_N |
| `coding-adventures-arithmetic` | 02 | half_adder, full_adder, ripple_carry_adder |
| `coding-adventures-simulator-protocol` | SIM00 | Simulator[T], ExecutionResult, StepTrace |
| `coding-adventures-mos6502-simulator` | 07j | MOS6502State (shared state type), cross-validation |

---

## Test Requirements

- Minimum 300 tests
- Target ≥90% coverage (≥80% required)
- All ruff checks pass

### Test Files

| File | Coverage target |
|------|----------------|
| `test_bits.py` | Round-trip, edge cases, 16-bit |
| `test_alu.py` | All ALU ops, all flags, BCD, overflow |
| `test_register_file.py` | Read/write all registers, pack/unpack P |
| `test_decoder.py` | All addressing modes, illegal opcodes |
| `test_equivalence.py` | 50+ cross-validated programs |
| `test_programs.py` | End-to-end: loops, subroutines, BCD, I/O |
| `test_simulator_coverage.py` | BRK, NMI, IRQ, JMP bug, all modes |

---

## Divergences from Behavioral Simulator

1. **Gate routing**: All data-path operations route through logic gate primitives.
   The behavioral simulator uses Python integer arithmetic directly.

2. **BRK handling**: Both treat BRK as halt (sets `halted=True`). Gate-level
   simulator preserves the same halt convention.

3. **NMI/IRQ methods**: Gate-level simulator exposes `nmi()` and `interrupt()`
   methods per the Simulator protocol. Behavioral simulator does not.

4. **Interrupt vectors**: Gate-level simulator reads NMI/IRQ vectors from memory
   (0xFFFA/B and 0xFFFE/F). Behavioral simulator does not implement vector dispatch
   for hardware interrupts.
