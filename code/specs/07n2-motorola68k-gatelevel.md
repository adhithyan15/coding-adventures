# Spec 07n2: Motorola 68000 Gate-Level Simulator

## Overview

This package implements a gate-level behavioral simulator for the Motorola 68000
(1979) CPU.  Every ALU data-path operation (ADD, SUB, AND, OR, XOR, NOT, shifts,
rotates) routes through logic gate primitives from the `logic-gates` and
`arithmetic` packages.  No Python integer arithmetic is used in the critical ALU
path; all arithmetic is performed by ripple-carry adder chains operating on
bit arrays.

The package cross-validates against the behavioral Motorola 68000 simulator
(`motorola-68000-simulator`) using the shared `M68KState` snapshot type and the
`Simulator[M68KState]` protocol from `simulator-protocol`.

## Layer

Layer 07n2 — gate-level Motorola 68000 (builds on 07n behavioral simulator)

## Dependencies

- `coding-adventures-logic-gates` — AND, OR, XOR, NOT gate primitives
- `coding-adventures-arithmetic` — half_adder, full_adder, ripple_carry_adder
- `coding-adventures-simulator-protocol` — Simulator protocol, ExecutionResult, StepTrace
- `coding-adventures-motorola-68000-simulator` — M68KState, used for cross-validation

## Architecture

### Motorola 68000 Hardware Summary

The Motorola 68000 contains approximately 68,000 transistors:

- **Data registers:** D0–D7 (32-bit each, fully orthogonal)
  - Byte ops affect bits 7–0 only (upper bits preserved)
  - Word ops affect bits 15–0 only (upper 16 bits preserved)
  - Longword ops affect all 32 bits
- **Address registers:** A0–A6 (32-bit), A7 = supervisor stack pointer (SSP)
  - No byte-size access to address registers
- **Program counter:** 32-bit (only bits 23–0 are significant — 24-bit address bus)
- **Status Register (SR):**
  - Bit 13: S — supervisor mode (always 1 in this simulator)
  - Bits 10–8: I2 I1 I0 — interrupt priority mask
  - Bit 4: X — extend (set same as C for ADD/SUB; used by ADDX/SUBX)
  - Bit 3: N — negative (MSB of result)
  - Bit 2: Z — zero (result == 0)
  - Bit 1: V — overflow (signed overflow)
  - Bit 0: C — carry (unsigned carry/borrow)

### Memory Model

- Flat 24-bit address space: 0x000000–0xFFFFFF (16 MB)
- Big-endian: MSB at lowest address
- Word and longword accesses must be even-aligned
- 0x000000–0x0003FF: Exception vector table (256 vectors × 4 bytes)
- 0x001000: Program load address
- 0x00F000: Initial supervisor stack pointer (A7)

## Gate-Level ALU Design

### Bit Representation

All register values and intermediate ALU results are represented internally as
lists of bits, LSB-first (index 0 = bit 0).  This matches the `logic-gates` and
`arithmetic` package conventions.

### Addition (add_8bit / add_16bit / add_32bit)

Uses `ripple_carry_adder` from the `arithmetic` package.  The adder chains
individual `full_adder` gates:

```
bit 0: full_adder(a[0], b[0], carry_in)   → (sum[0], carry[0])
bit 1: full_adder(a[1], b[1], carry[0])   → (sum[1], carry[1])
...
bit N: full_adder(a[N], b[N], carry[N-1]) → (sum[N], carry_out)
```

### Subtraction (two's complement)

SUB A, B = A + NOT(B) + 1 (no borrow)
SUBX A, B, X = A + NOT(B) + NOT(X)  (with extend/borrow)

The N NOT gates and 1 add are the gate-level path.  CF (carry = borrow) is
derived as NOT(carry_out_of_adder).

### Overflow Detection

For N-bit signed addition A + B = R:

```
OF = XOR(carry_into_bit[N-1], carry_out_of_bit[N-1])
```

This is a single XOR gate at the MSB stage of the adder.

### Logical Operations (AND / OR / XOR / NOT)

N parallel gates (one per bit), operating on bit arrays.  No flags: V=0, C=0,
X unchanged.

### Shifts and Rotates

Implemented as bit-array rewiring — gates model the routing of bits to new
positions.  For each shift of count C:
- LSL/ASL: bits[C:width] → result[0:width-C]; zeros fill from right
- LSR/ASR: bits[0:width-C] → result[C:width]; ASR replicates sign bit
- ROL: circular left rotation of bit array
- ROR: circular right rotation of bit array
- ROXL/ROXR: rotation through the X flag (width+1 bit rotation)

## Module Structure

### `bits.py`

Core bit-manipulation utilities:

```
int_to_bits(value, width) → list[int]   # unsigned integer → LSB-first bit list
bits_to_int(bits) → int                  # bit list → unsigned integer
add_8bit(a, b, carry_in=0) → (result, carry_out, aux_carry)
add_16bit(a, b, carry_in=0) → (result, carry_out, aux_carry)
add_32bit(a, b, carry_in=0) → (result, carry_out)
invert_8bit(value) → int
invert_16bit(value) → int
invert_32bit(value) → int
compute_parity(bits) → int   # XOR tree over low 8 bits; 1=even parity
compute_zero(bits) → int     # NOR tree; 1 if all bits are 0
```

### `alu.py`

ALUResult68k dataclass + all ALU operations:

```
add8/add16/add32(a, b, extend_in=0) → ALUResult68k
sub8/sub16/sub32(a, b, extend_in=0) → ALUResult68k
and8/and16/and32(a, b) → ALUResult68k
or8/or16/or32(a, b) → ALUResult68k
xor8/xor16/xor32(a, b) → ALUResult68k
not8/not16/not32(a) → int
neg8/neg16/neg32(a) → ALUResult68k
cmp8/cmp16/cmp32(a, b) → ALUResult68k
asl/asr/lsl/lsr/rol/ror/roxl/roxr(value, count, width, [x]) → (result, c, [v])
muls/mulu(d_val, src_val) → (result32, n, z)
divs/divu(d_val, src_val) → (q16, r16, overflow)
```

### `register_file.py`

`RegisterFile68k` class — maintains all CPU registers as bit arrays:

- `_d[0..7]` — 32-bit data registers (LSB-first bit lists)
- `_a[0..7]` — 32-bit address registers
- `_pc` — 32-bit program counter
- Individual flag bits: `_flag_c`, `_flag_v`, `_flag_z`, `_flag_n`, `_flag_x`, `_flag_s`
- `_int_mask` — 3-bit interrupt mask

Methods: `read_dn(n, size)`, `write_dn(n, value, size)`, `read_an(n)`, `write_an(n, value)`,
`read_pc()`, `write_pc(value)`, `pack_ccr()`, `unpack_ccr(ccr)`, `pack_sr()`, `unpack_sr(sr)`.

### `decoder.py`

`decode(memory, pc) → DecodedInstr68k` — decodes a 68000 instruction from memory:

- Reads the 16-bit opword at `pc`
- Dispatches on opword bits 15–12 (instruction class)
- Returns: mnemonic, size, src_ea, dst_ea, byte_length

### `simulator.py`

`Motorola68kGateLevelSimulator` — implements the `Simulator[M68KState]` protocol:

- `reset()` — power-on state
- `load(program)` — load bytes at 0x001000
- `step()` → `StepTrace` — execute one instruction
- `execute(program, max_steps)` → `ExecutionResult[M68KState]`
- `get_state()` → `M68KState`
- `set_input_port(port, value)` / `get_output_port(port)` — no-op (68k has no I/O ports)
- `interrupt(level)` / `nmi()` — set pending interrupt / NMI

## Instruction Set Coverage

### Data Movement
- MOVE.B/.W/.L (all EA modes)
- MOVEA.W/.L — move to address register
- MOVEQ — quick immediate to Dn
- MOVEM — move multiple registers to/from memory
- MOVE to/from CCR, to/from SR
- EXG — exchange registers

### Arithmetic
- ADD, ADDA, ADDI, ADDQ, ADDX (all sizes)
- SUB, SUBA, SUBI, SUBQ, SUBX (all sizes)
- MULS, MULU — signed/unsigned 16×16→32 multiply
- DIVS, DIVU — signed/unsigned 32÷16→(quotient,remainder)
- NEG, NEGX — negate (with extend)
- CLR — clear operand
- EXT — sign-extend
- ABCD, SBCD, NBCD — BCD arithmetic

### Logical
- AND, ANDI, OR, ORI, EOR, EORI (all sizes)
- NOT — bitwise complement

### Shifts and Rotates
- ASL, ASR — arithmetic shift left/right
- LSL, LSR — logical shift left/right
- ROL, ROR — rotate left/right (without carry)
- ROXL, ROXR — rotate left/right through X flag

### Bit Operations
- BTST, BCHG, BCLR, BSET — bit test, change, clear, set

### Comparison and Test
- CMP, CMPA, CMPI, CMPM — compare
- TST — test (sets N/Z flags, clears V/C)
- CHK — check register against bounds

### Control Flow
- BRA — branch always
- BSR — branch to subroutine
- Bcc — conditional branch (16 conditions: T/F/HI/LS/CC/CS/NE/EQ/VC/VS/PL/MI/GE/LT/GT/LE)
- DBcc — decrement and branch
- Scc — set byte on condition
- ADDQ/SUBQ — quick add/subtract (3-bit immediate)

### Subroutine and Stack
- JSR — jump to subroutine
- JMP — jump
- RTS — return from subroutine
- RTR — return and restore CCR
- RTE — return from exception
- LINK — link and allocate stack frame
- UNLK — unlink

### Miscellaneous
- NOP — no operation
- SWAP — swap halves of Dn
- PEA — push effective address
- LEA — load effective address
- TRAP #n — software exception
- ILLEGAL — undefined instruction exception
- STOP — stop and wait for interrupt
- RESET — assert RESET line (no-op in simulator)

## Effective Address Modes

| Mode | Notation | Description |
|------|----------|-------------|
| 000 Dn | Dn | Data register direct |
| 001 An | An | Address register direct |
| 010 An | (An) | Address register indirect |
| 011 An | (An)+ | Postincrement indirect |
| 100 An | -(An) | Predecrement indirect |
| 101 An | d16(An) | 16-bit displacement indirect |
| 110 An | d8(An,Xn) | 8-bit displacement + index |
| 111 000 | (xxx).W | Absolute short (sign-extended 16-bit) |
| 111 001 | (xxx).L | Absolute long (32-bit) |
| 111 010 | d16(PC) | PC-relative + 16-bit displacement |
| 111 011 | d8(PC,Xn) | PC-relative + index |
| 111 100 | #imm | Immediate data |

## Flag Rules

| Operation | C | X | N | Z | V |
|-----------|---|---|---|---|---|
| ADD | carry | =C | MSB | zero | signed OVF |
| ADDA | — | — | — | — | — |
| ADDQ | carry | =C | MSB | zero | signed OVF |
| ADDX | carry | =C | MSB | only clears | signed OVF |
| SUB | borrow | =C | MSB | zero | signed OVF |
| AND/OR/XOR | 0 | — | MSB | zero | 0 |
| MOVE | 0 | — | MSB | zero | 0 |
| CMP | borrow | — | MSB | zero | signed OVF |
| NEG | ≠0 | =C | MSB | zero | =0x80 |
| SHIFT | last out | =C | MSB | zero | see note |
| ROTATE | last out | — | MSB | zero | 0 |
| ROXL/ROXR | last out | =C | MSB | zero | 0 |

Note: For ASL/ASR/LSL/LSR, V=1 if any bits shifted out differ from MSB (overflow).
For count=0, V=0; for ROXL/ROXR, V=0.

## Cross-Validation

The `test_equivalence.py` test module runs 40+ programs on both the gate-level
and behavioral simulators, comparing final `M68KState` values for identical
register state and flag values.  Programs include:
- Arithmetic loops (factorial, Fibonacci)
- Subroutine calls (BSR/RTS/LINK/UNLK)
- MOVEM register save/restore
- DBcc counted loops
- String copy (MOVE.B with postincrement)
- All shift/rotate sizes

## Implementation Divergence from Spec

None — this is the initial implementation.

## Testing Requirements

- At least 300 tests, >80% coverage
- `ruff check` must pass
- Literate-style docstrings throughout
