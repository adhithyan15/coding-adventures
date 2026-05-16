# Spec 07m2 — Intel 8086 Gate-Level Simulator

## Overview

Layer **07m2** defines a **gate-level** simulator for the Intel 8086 (1978) 16-bit
microprocessor.  Every data-path operation (ADD, SUB, AND, OR, XOR, shifts,
rotates) routes through logic gate primitives — AND, OR, XOR, NOT gates and
ripple-carry adder chains — from the `logic-gates` and `arithmetic` packages.

This is the gate-level companion to the behavioral simulator (07m).  It
implements the same `Simulator[X86State]` protocol and produces identical
observable results; the difference is that the arithmetic path is modeled at
the transistor/gate level rather than using Python's integer arithmetic.

The Intel 8086 contained approximately **29,000 transistors** — the most
complex CPU in this simulator collection so far.  The 16-bit data path requires
twice the gate count of the 8-bit 6502 or Z80 gate-level simulators.

---

## Architecture

### Register file

All registers stored as 16-element lists of 0/1 bits (LSB-first, matching the
`arithmetic` package convention):

| Register | Width | Description |
|----------|-------|-------------|
| AX       | 16    | Accumulator; AL = bits[0:8], AH = bits[8:16] |
| BX       | 16    | Base register |
| CX       | 16    | Counter (LOOP, REP, shift counts) |
| DX       | 16    | Data; high word of 32-bit MUL/DIV; I/O port |
| SI       | 16    | Source Index |
| DI       | 16    | Destination Index |
| SP       | 16    | Stack Pointer |
| BP       | 16    | Base Pointer |
| CS       | 16    | Code Segment |
| DS       | 16    | Data Segment |
| SS       | 16    | Stack Segment |
| ES       | 16    | Extra Segment |
| IP       | 16    | Instruction Pointer |
| FLAGS    | 9 individual bits | CF, PF, AF, ZF, SF, TF, IF, DF, OF |

### Memory model

Segmented 20-bit address space: `physical = (seg × 16 + offset) & 0xFFFFF`

The gate-level `add_20bit()` function routes segment × 16 + offset through the
ripple-carry adder (after computing `seg << 4` via wiring, not arithmetic).

### FLAGS layout

```
Bit 11: OF    Bit 10: DF    Bit  9: IF    Bit  8: TF
Bit  7: SF    Bit  6: ZF    Bit  4: AF    Bit  2: PF
Bit  0: CF    Bit  1: always 1
```

---

## Module structure

### `bits.py` — Bit conversion and low-level helpers

- `int_to_bits(value, width)` — integer → LSB-first bit list
- `bits_to_int(bits)` — LSB-first bit list → integer
- `add_8bit(a, b, carry_in)` → `(result, carry_out, aux_carry)` — via ripple_carry_adder; aux_carry = carry out of bit 3
- `add_16bit(a, b, carry_in)` → `(result, carry_out, aux_carry)` — aux_carry = carry out of bit 3
- `add_20bit(a, b)` → `(result, carry_out)` — for effective-address computation
- `invert_8bit(value)` → int — 8 NOT gates
- `invert_16bit(value)` → int — 16 NOT gates
- `compute_parity(bits)` → int — XOR tree over low 8 bits; 1 if even number of 1s
- `compute_zero(bits)` → int — NOR tree; 1 if all bits are 0

### `alu.py` — 16-bit (and 8-bit) ALU

`ALUResult8086` dataclass: `result`, `flag_cf`, `flag_of`, `flag_sf`, `flag_zf`, `flag_af`, `flag_pf`

All data-path operations route through gate primitives.  MUL/DIV use host
arithmetic internally (gate-level 16×16 multiplier is out of scope).

16-bit operations: `add16`, `sub16`, `and16`, `or16`, `xor16`, `inc16`, `dec16`, `neg16`, `not16`
8-bit operations: `add8`, `sub8`, `and8`, `or8`, `xor8`, `inc8`, `dec8`, `neg8`, `not8`
Shifts/rotates: `shl`, `shr`, `sar`, `rol`, `ror`, `rcl`, `rcr`
Multiply/divide: `mul8`, `mul16`, `imul8`, `imul16`, `div8`, `div16`, `idiv8`, `idiv16`
BCD: `daa`, `das`, `aaa`, `aas`, `aam`, `aad`

### `register_file.py` — Register file

`RegisterFile8086`: 13 × 16-bit registers stored as bit arrays; 9 individual flag bits.

Methods: `read16`, `write16`, `read8_high`, `read8_low`, `write8_high`, `write8_low`,
`pack_flags`, `unpack_flags`, `physical_address`

### `decoder.py` — Instruction decoder

`decode_instruction(memory, cs, ip)` → `DecodedInstr` with mnemonic, operands, length.

Decodes the variable-length 8086 instruction format including:
- 1-byte and 2-byte opcodes
- ModRM byte (mod/reg/r/m fields)
- Displacement (disp8, disp16)
- Immediate (imm8, imm16)
- Prefix bytes (segment override, REP/REPNE, LOCK)

### `simulator.py` — Full simulator

`Intel8086GateLevelSimulator` implementing `Simulator[X86State]`.

All data-path operations delegate to `alu.py`.  Memory access masked to 20 bits.
REP string operations, segment override prefixes, INT/IRET, hardware stack all implemented.

---

## Cross-validation

`test_equivalence.py` runs 40+ programs on both `Intel8086GateLevelSimulator` and
the behavioral `X86Simulator`, asserting identical final register/memory state.

---

## Implementation notes

1. **ADD/SUB via two's complement**: SUB uses `invert_16bit(b)` + `add_16bit(a, not_b, 1)`.
   The carry out of bit 3 is captured as `aux_carry` (AF flag).
2. **MUL/DIV**: Host arithmetic used internally (16-bit multiplier/divider would
   require hundreds of gate primitives — out of scope for educational purposes).
3. **Segment multiplication**: `seg × 16` is a 4-bit left shift of the 16-bit
   segment register, yielding a 20-bit value.  Implemented as bit rewiring.
4. **Overflow (OF)**: `XOR(carry_into_msb, carry_out_of_msb)` — one XOR gate.
5. **Parity (PF)**: XOR tree over bits 0–7 of result; 1 if even popcount.

---

## Divergence from spec during implementation

None — this spec was written to match the implementation.
