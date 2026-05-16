# 07k2 — Zilog Z80 Gate-Level Simulator

## Overview

The gate-level Z80 simulator models the Zilog Z80 (1976) at the hardware level.
Every arithmetic and logic operation routes through actual gate-primitive functions —
`AND`, `OR`, `XOR`, `NOT` from `logic-gates`, and `ripple_carry_adder` from
`arithmetic` — exactly as the real Z80's ~8,500 transistors did in 1976.

This is **not** the same as the behavioral simulator (`07k-z80-simulator.md`). The
behavioral simulator executes instructions directly with host-language integers. This
simulator routes everything through the gate abstractions we built from scratch.

Both produce bit-for-bit identical output for any program. The difference is the
execution path:

```
Behavioral:  opcode → match → host integer arithmetic → result
Gate-level:  opcode → decoder gates → ALU gate chain → ripple-carry adder → result
```

## Layer Position

```
[Logic Gates] → [Arithmetic] → [CPU] → [Intel 4004 gatelevel] → ...
                                      → [Intel 8008 gatelevel] → ...
                                      → [ARM1 gatelevel] → ...
                                      → [Intel 8080 gatelevel] → [YOU ARE HERE]
```

This package composes packages from layers below:
- `logic-gates`: AND, OR, XOR, NOT, mux2, XOR_N, AND_N, OR_N
- `arithmetic`: half_adder, full_adder, ripple_carry_adder

## Why Z80 Gate-Level?

The Z80 had approximately 8,500 transistors — roughly twice the 8080's ~6,000.
Building it gate-level lets us:

1. **See Z80 vs 8080 complexity**: The Z80's alternate register bank adds flip-flops
   for the shadow registers (A', F', B'C', D'E', H'L'). The IX/IY index registers
   add 32 more flip-flops each. DAD → ADC HL,rp gains a full 16-bit ripple-carry
   path.

2. **Count the transistors accounted for**: The register file alone uses
   ~3,200 flip-flops (25 registers × 8 bits × 16 transistors per flip-flop).
   The 8-bit ALU uses ~200 gate calls per ADD. The CB-prefix barrel rotator
   adds another ~160 gate calls per shift/rotate operation.

3. **Understand prefix dispatch**: The Z80 uses opcode prefixes (CB, DD, ED, FD)
   decoded in the first instruction-fetch cycle. Each prefix changes how the
   following byte is interpreted — a real PLA expansion technique.

4. **Appreciate Z80's innovations**: The EXX shadow bank was a novel way to
   provide a "fast interrupt context" without a dedicated call/return overhead.
   The IX/IY displacement addressing (IX+d) allowed structured data access with
   a single instruction instead of a load-add-indirect sequence.

## Architecture — Block Diagram

```
                ┌────────────────────────────────────────────────┐
                │              Z80 (Gate-Level)                   │
                │                                                  │
  Memory ─────→ │ ┌──────┐  ┌──────────────────────┐             │
  (64 KiB)      │ │  PC  │→ │  Instruction Decoder  │             │
                │ │ 16-bit│  │   (gate trees + PLA)   │             │
                │ └──────┘  └──────────┬───────────┘             │
                │                      │ control signals          │
                │  ┌──────┐  ┌─────────┴──────────┐             │
                │  │ ALU  │←─┤  Register File       │             │
                │  │ 8-bit│  │  Main: A F BC DE HL  │             │
                │  │gates │  │  Alt:  A'F'B'C'D'E'H'L'│          │
                │  └──┬───┘  │  Index: IX IY SP     │             │
                │     │      │  Special: I R IFF1/2  │             │
                │   flags    └─────────────────────┘             │
                │  SZHPVNC                                         │
                └────────────────────────────────────────────────┘
```

## Components

### 1. `bits.py` — Bit Conversion Helpers

Converts between integers and LSB-first bit lists. The bridge between the
integer world (external API, test programs) and the gate world (lists of 0/1).

```python
def int_to_bits(value: int, width: int) -> list[int]:
    """Convert integer to LSB-first bit list. int_to_bits(5, 8) → [1,0,1,0,0,0,0,0]"""

def bits_to_int(bits: list[int]) -> int:
    """Convert LSB-first bit list to integer."""

def add_8bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int, int]:
    """Add two 8-bit values via ripple_carry_adder. Returns (result, carry_out, aux_carry)."""

def add_16bit(a: int, b: int, carry_in: int = 0) -> tuple[int, int]:
    """Add two 16-bit values via ripple_carry_adder. Returns (result, carry_out)."""

def invert_8bit(value: int) -> int:
    """Bitwise NOT via 8 NOT gates. Used for two's complement subtraction."""

def compute_parity(bits: list[int]) -> int:
    """Even parity via XOR gate tree. Returns 1 if even count of 1-bits."""

def compute_zero(bits: list[int]) -> int:
    """Zero detection via NOR tree. Returns 1 if all bits are 0."""
```

### 2. `alu.py` — 8-bit and 16-bit ALU

The heart of the gate-level simulation. Every operation routes through actual
gate function calls.

**8-bit operations** (on the accumulator A and one operand):

```python
# ADD A,r: routes through ripple_carry_adder(a_bits, b_bits, 0)
def add8(a: int, b: int, carry_in: int) -> ALUResultZ80: ...

# SUB A,r: two's complement: A + NOT(B) + 1 via ripple_carry_adder
def sub8(a: int, b: int, borrow_in: int) -> ALUResultZ80: ...

# AND A,r: 8 AND gates in parallel
def and8(a: int, b: int) -> ALUResultZ80: ...

# OR A,r: 8 OR gates in parallel
def or8(a: int, b: int) -> ALUResultZ80: ...

# XOR A,r: 8 XOR gates in parallel
def xor8(a: int, b: int) -> ALUResultZ80: ...

# INC r: add8(r, 1, 0) but C flag unchanged
def inc8(a: int) -> ALUResultZ80: ...

# DEC r: sub8(r, 1, 0) but C flag unchanged
def dec8(a: int) -> ALUResultZ80: ...

# NEG: 0 - A = NOT(A) + 1 via ripple_carry_adder
def neg8(a: int) -> ALUResultZ80: ...

# CPL: NOT A (8 NOT gates), H=1, N=1
def cpl8(a: int) -> ALUResultZ80: ...

# DAA: Binary-Coded Decimal adjust
def daa8(a: int, flag_n: int, flag_h: int, flag_c: int) -> ALUResultZ80: ...
```

**Rotate/shift operations** (CB-prefix):

```python
def rlc8(a: int) -> tuple[int, int]: ...   # rotate left through accumulator
def rrc8(a: int) -> tuple[int, int]: ...
def rl8(a: int, carry_in: int) -> tuple[int, int]: ...   # through carry
def rr8(a: int, carry_in: int) -> tuple[int, int]: ...
def sla8(a: int) -> tuple[int, int]: ...   # shift left arithmetic
def sra8(a: int) -> tuple[int, int]: ...   # shift right arithmetic (sign-extend)
def srl8(a: int) -> tuple[int, int]: ...   # shift right logical
def bit_test(a: int, bit_n: int) -> int: ...  # BIT b,r: AND gate test
```

**16-bit operations**:

```python
def add16(hl: int, rp: int) -> tuple[int, int, int]: ...    # ADD HL,rp
def adc16(hl: int, rp: int, c: int) -> ALUResultZ80: ...    # ADC HL,rp
def sbc16(hl: int, rp: int, b: int) -> ALUResultZ80: ...    # SBC HL,rp
```

**Z80 Flag register (F):**

```
Bit 7  S  — Sign: bit 7 of result
Bit 6  Z  — Zero: result == 0
Bit 5  Y  — Undocumented: copy of result bit 5
Bit 4  H  — Half-carry: carry from bit 3 to 4
Bit 3  X  — Undocumented: copy of result bit 3
Bit 2  P/V — Parity (logical) or Overflow (arithmetic)
Bit 1  N  — Add/Subtract: 0 after ADD, 1 after SUB
Bit 0  C  — Carry
```

### 3. `register_file.py` — Register File

All 26 physical registers stored as bit arrays (flip-flop state):

```
Main bank (8 registers × 8 bits = 64 flip-flops):
    A[8], F[8], B[8], C[8], D[8], E[8], H[8], L[8]

Alternate bank (8 registers × 8 bits = 64 flip-flops):
    A'[8], F'[8], B'[8], C'[8], D'[8], E'[8], H'[8], L'[8]

Index registers (2 registers × 16 bits = 32 flip-flops):
    IX[16], IY[16]

Stack / program counter (2 registers × 16 bits = 32 flip-flops):
    SP[16], PC[16]

Special (2 registers × 8 bits = 16 flip-flops):
    I[8], R[8]

Interrupt (3 flip-flops):
    IFF1, IFF2, IM[2]
```

Total: ~208 flip-flop bits. At ~16 transistors per flip-flop: ~3,328 transistors just
for registers. This is consistent with the real Z80's ~8,500 transistor total when
combined with the ALU, decoder, and control logic.

### 4. `decoder.py` — Instruction Decoder (Gate Trees)

The Z80's instruction decoder is a PLA with 5 main instruction groups, classified
using bits 7:6 of the opcode. We model the primary classification as AND/OR/NOT gates:

```
group_00 = AND(NOT(bit7), NOT(bit6))  # 00xxxxxx: LD rr, rotates, INC/DEC
group_01 = AND(NOT(bit7), bit6)       # 01xxxxxx: LD r,r (or HALT if 0x76)
group_10 = AND(bit7, NOT(bit6))       # 10xxxxxx: ALU A,r
group_11 = AND(bit7, bit6)            # 11xxxxxx: JP/CALL/RET/PUSH/POP/RST
```

Additionally, the following opcode bytes trigger prefix mode:
- `0xCB` → CB-prefix: BIT/SET/RES + rotates/shifts
- `0xDD` → DD-prefix: IX-displacement instructions
- `0xED` → ED-prefix: extended instructions (LDIR, LDDR, NEG, etc.)
- `0xFD` → FD-prefix: IY-displacement instructions
- `0xDD 0xCB` → DDCB-prefix: indexed bit operations with IX
- `0xFD 0xCB` → FDCB-prefix: indexed bit operations with IY

### 5. `simulator.py` — Top-Level Gate-Level CPU

```python
class Z80GateLevelSimulator:
    """Gate-level Zilog Z80 simulator implementing Simulator[Z80State].

    Routes ALL data-path operations (ALU, barrel rotate, register reads/writes,
    flag computation) through gate-level primitives. No Python integer arithmetic
    appears in the execution path — only in int_to_bits / bits_to_int conversions.
    """

    def reset(self) -> None: ...
    def load(self, program: bytes, origin: int = 0) -> None: ...
    def step(self) -> StepTrace: ...
    def execute(self, program: bytes, origin: int = 0) -> ExecutionResult[Z80State]: ...
    def get_state(self) -> Z80State: ...
```

## Execution Flow (One ADD A,B Instruction)

Here's what happens when the gate-level simulator executes `ADD A, B` (opcode 0x80):

```
1. FETCH
   Read byte at PC → 0x80
   Increment PC via 16-bit ripple_carry_adder chain (16 gate calls)

2. DECODE
   bits 7:6 = 10 → ALU A,r group
   group_10 = AND(bit7, NOT(bit6)) = AND(1, NOT(0)) = AND(1, 1) = 1
   alu_op = bits 5:3 = 000 → ADD
   src_reg = bits 2:0 = 000 → B register

3. REGISTER READ
   Read A: 8-bit array [a7, a6, ..., a0] from flip-flop store
   Read B: 8-bit array [b7, b6, ..., b0] from flip-flop store
   ~32 gate calls (mux for register selection)

4. ALU EXECUTE (add8)
   ripple_carry_adder(a_bits, b_bits, 0):
     full_adder(a[0], b[0], 0)
       half_adder(a[0], b[0]) → XOR, AND
       half_adder(sum, 0)     → XOR, AND
       OR(carry1, carry2)
     full_adder(a[1], b[1], c0) → ...
     ...
     full_adder(a[7], b[7], c6) → carry_out = C flag
   Flag S = result[7]
   Flag Z = compute_zero(result) = NOR tree
   Flag H = carry_out of bit 3 (re-run 4-bit adder)
   Flag P/V = overflow: XOR(carry_in_to_bit7, carry_out)
   Flag N = 0 (ADD clears it)
   ~200 gate calls

5. REGISTER WRITE
   Write A = result: 8 flip-flop updates (bit-array store)
   Write F = packed flags: 8 flip-flop updates
   ~16 gate calls

GRAND TOTAL: ~280 gate calls for one ADD A,B
```

## Gate Count Estimates

| Component | Gates (approx) | Transistors (approx) |
|-----------|----------------|---------------------|
| 8-bit ALU (all ops) | ~200 | ~400 |
| 16-bit adder (ADD HL,rp) | ~320 | ~640 |
| CB rotate/shift | ~80 | ~160 |
| Register file (26 × 8-16 bit) | ~208 flip-flops | ~3,328 |
| Instruction decoder | ~80 | ~160 |
| Prefix dispatcher | ~40 | ~80 |
| Control FSM | ~100 | ~200 |
| Memory address logic | ~100 | ~200 |
| **Total** | **~1,128** | **~5,168** |

Note: The real Z80 has ~8,500 transistors. The gap is accounted for by analog timing
circuits, clock distribution, bus drivers, I/O logic, and interrupt circuitry that
our behavioral approximation doesn't model.

## Dependencies

```
z80-gatelevel
├── logic-gates (AND, OR, XOR, NOT, mux2, XOR_N, AND_N, OR_N)
├── arithmetic (half_adder, full_adder, ripple_carry_adder)
├── simulator-protocol (Simulator, ExecutionResult, StepTrace)
└── z80-simulator (Z80State — shared state type for cross-validation)
```

## Implementation Structure

```
z80-gatelevel/
├── bits.py           Bit conversion helpers
├── alu.py            8/16-bit ALU via gate primitives
├── register_file.py  Z80 register file with main + alternate bank
├── decoder.py        Instruction decoder (gate trees)
└── simulator.py      Top-level Z80GateLevelSimulator (SIM00 protocol)
```

## Test Strategy

### Unit Tests

- `test_bits.py` — bit conversion round-trips, add_8bit, add_16bit edge cases
- `test_alu.py` — all 8-bit ALU ops, all rotate/shift ops, 16-bit ops, flag verification
- `test_register_file.py` — read/write all registers, exchange_af, exchange_bank (EXX)
- `test_decoder.py` — representative opcodes from each instruction group

### Cross-Validation

- `test_equivalence.py` — runs the same program on both gate-level and behavioral Z80
  simulators, asserts identical final state (all registers, all flags, all memory)

### End-to-End Programs

- `test_programs.py` — full programs: arithmetic, loops, subroutine call/return,
  block memory copy, 16-bit arithmetic, CB-prefix rotate/shift operations

### Gate Count / Integration

- `test_simulator_coverage.py` — covers IX/IY indexed addressing, ED-prefix block
  operations (LDIR/LDDR), conditional branches, I/O ports, interrupt state

## Coverage and Quality

- Tests: 355 tests
- Coverage: 96.16% line coverage (>80% required)
- Linting: ruff clean (E/W/F/I/UP/B/SIM rules)
