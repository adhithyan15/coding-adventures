# 07j — MOS 6502 Simulator

## Overview

The MOS Technology 6502 (1975) is one of the most historically significant
microprocessors ever made. At $25 (a fraction of the Intel 8080's $179), it
democratised computing and powered the Apple II (1977), Commodore 64 (1982),
Atari 2600 (1977), BBC Micro (1981), and the NES/Famicom (1983).

This package provides a complete, cycle-accurate **behavioral simulator** for
the NMOS 6502 (original variant, not 65C02). It implements all 151 official
opcodes across 13 addressing modes, correct flag behaviour including the
famous indirect JMP bug, and BCD decimal-mode arithmetic in ADC/SBC.

## Layer Position

```
logic-gates → arithmetic → simulator-protocol → [YOU ARE HERE]
                                                        ↓
                                               MOS6502Simulator
                                               (Simulator[MOS6502State])
```

**Depends on:** `simulator-protocol` (SIM00 `Simulator[T]` interface)  
**Provides:** `MOS6502Simulator`, `MOS6502State`

## Architecture

### Registers

| Register | Width | Description |
|----------|-------|-------------|
| A  | 8-bit  | Accumulator — all arithmetic/logical operations use A |
| X  | 8-bit  | Index register X — addressing offset, loop counter |
| Y  | 8-bit  | Index register Y — addressing offset, loop counter |
| S  | 8-bit  | Stack pointer — points into page 1 (0x0100–0x01FF) |
| PC | 16-bit | Program counter |
| P  | 8-bit  | Processor status (flags) |

### Processor Status Register (P)

```
Bit:  7   6   5   4   3   2   1   0
Flag: N   V   -   B   D   I   Z   C
```

| Flag | Name              | Set when…                                          |
|------|-------------------|----------------------------------------------------|
| N    | Negative          | Bit 7 of result is 1                               |
| V    | Overflow          | Signed arithmetic overflow (result out of −128..127) |
| -    | (unused)          | Always reads 1                                     |
| B    | Break             | BRK instruction was executed (set in pushed copy) |
| D    | Decimal           | BCD mode enabled (affects ADC/SBC)                |
| I    | IRQ disable       | Disables hardware interrupt requests              |
| Z    | Zero              | Result is zero                                     |
| C    | Carry             | Carry out of bit 7 (or borrow complement)         |

### Memory Map

```
0x0000–0x00FF  Zero page  — Fast 2-byte addressing (LDA $42 = 2 bytes vs. LDA $0042 = 3 bytes)
0x0100–0x01FF  Stack      — Fixed at page 1; S wraps around within page
0x0200–0xFFF9  RAM/ROM    — Program code and data
0xFFFA–0xFFFB  NMI vector (lo, hi)
0xFFFC–0xFFFD  RESET vector (lo, hi)
0xFFFE–0xFFFF  IRQ/BRK vector (lo, hi)
```

### Addressing Modes

The 6502 has 13 addressing modes. Each determines how the instruction finds
its operand.

| Mode              | Syntax         | Size  | Description |
|-------------------|----------------|-------|-------------|
| Implied           | `CLC`          | 1     | Operand implicit (no extra bytes) |
| Accumulator       | `ASL A`        | 1     | Operand is the accumulator |
| Immediate         | `LDA #$42`     | 2     | Operand is the literal byte |
| Zero Page         | `LDA $42`      | 2     | Address is 0x0000 + zp byte |
| Zero Page,X       | `LDA $42,X`    | 2     | Address is (zp + X) & 0xFF |
| Zero Page,Y       | `LDA $42,Y`    | 2     | Address is (zp + Y) & 0xFF |
| Absolute          | `LDA $1234`    | 3     | Full 16-bit address |
| Absolute,X        | `LDA $1234,X`  | 3     | Address = abs + X |
| Absolute,Y        | `LDA $1234,Y`  | 3     | Address = abs + Y |
| Relative          | `BEQ $±127`    | 2     | PC-relative signed offset (branches) |
| (Indirect,X)      | `LDA ($42,X)`  | 2     | Zero-page pointer at (zp+X)&0xFF |
| (Indirect),Y      | `LDA ($42),Y`  | 2     | Zero-page pointer + Y |
| Absolute Indirect | `JMP ($1234)`  | 3     | JMP only — reads 16-bit pointer |

### Instruction Set

The 6502 has 56 official instructions (151 opcodes including all addressing
mode variants).

**Load/Store:**
```
LDA  Load accumulator      A ← M           N,Z
LDX  Load X                X ← M           N,Z
LDY  Load Y                Y ← M           N,Z
STA  Store accumulator     M ← A
STX  Store X               M ← X
STY  Store Y               M ← Y
```

**Register Transfers:**
```
TAX  Transfer A→X          X ← A           N,Z
TAY  Transfer A→Y          Y ← A           N,Z
TXA  Transfer X→A          A ← X           N,Z
TYA  Transfer Y→A          A ← Y           N,Z
TSX  Transfer S→X          X ← S           N,Z
TXS  Transfer X→S          S ← X           (no flags)
```

**Stack:**
```
PHA  Push A                M[0x100+S] ← A; S--
PLA  Pull A                S++; A ← M[0x100+S]    N,Z
PHP  Push P                M[0x100+S] ← P; S--    (B always set in pushed byte)
PLP  Pull P                S++; P ← M[0x100+S]
```

**Arithmetic (critical: carry flag semantics):**
```
ADC  Add with carry    A ← A + M + C            N,V,Z,C
SBC  Subtract w/borrow A ← A - M - (1-C)        N,V,Z,C
```
Note: `SBC` is equivalent to `ADC` with the operand inverted. The carry flag
acts as a *not-borrow*: C=1 means no borrow. For subtraction always `SEC`
first to clear the borrow.

Decimal mode (D=1): ADC/SBC perform Binary Coded Decimal arithmetic.
Each nibble is treated as one decimal digit (0-9). The 6502 NMOS chip does
not set N, V, Z correctly in decimal mode (they reflect the binary result
before BCD correction). The 65C02 fixes this; we match NMOS behaviour.

**Increment/Decrement:**
```
INC  M ← M + 1          N,Z   (memory operand)
INX  X ← X + 1          N,Z
INY  Y ← Y + 1          N,Z
DEC  M ← M - 1          N,Z
DEX  X ← X - 1          N,Z
DEY  Y ← Y - 1          N,Z
```

**Logical:**
```
AND  A ← A & M           N,Z
ORA  A ← A | M           N,Z
EOR  A ← A ^ M           N,Z
```

**Shift/Rotate (all set C from the bit shifted out):**
```
ASL  Arithmetic Shift Left  C ← [7] A/M << 1 [0] ← 0    N,Z,C
LSR  Logical Shift Right    0 → [7] A/M >> 1 → C         N,Z,C
ROL  Rotate Left            C ← [7] A/M << 1 [0] ← C    N,Z,C
ROR  Rotate Right           C → [7] A/M >> 1 → C         N,Z,C
```

**Compare (sets flags as if subtraction, discards result):**
```
CMP  A - M    N,Z,C
CPX  X - M    N,Z,C
CPY  Y - M    N,Z,C
```

**Bit Test:**
```
BIT  N ← M[7]; V ← M[6]; Z ← (A & M == 0)
```

**Branches (all 2-byte, take PC-relative signed offset if condition met):**
```
BCC  Branch if C=0      (Carry Clear)
BCS  Branch if C=1      (Carry Set)
BEQ  Branch if Z=1      (Equal / Zero)
BNE  Branch if Z=0      (Not Equal)
BPL  Branch if N=0      (Plus / Positive)
BMI  Branch if N=1      (Minus / Negative)
BVC  Branch if V=0      (Overflow Clear)
BVS  Branch if V=1      (Overflow Set)
```
Branch offset range: −128 to +127 bytes from the byte after the branch
instruction (i.e., from PC+2).

**Jumps/Calls:**
```
JMP  Jump absolute           PC ← addr
JMP  Jump indirect           PC ← (addr)     [*bug: wraps on page boundary]
JSR  Jump to subroutine      push PC-1; PC ← addr
RTS  Return from subroutine  pop PC; PC ← PC+1
RTI  Return from interrupt   pop P; pop PC    (no +1)
BRK  Software interrupt      push PC+1; push P(B=1); PC ← (0xFFFE)
```

**The indirect JMP bug:** `JMP ($10FF)` reads the low byte from `$10FF` but
the high byte from `$1000` (not `$1100`). This is a known silicon bug in all
NMOS 6502 chips; this simulator **must** replicate it.

**Flag Instructions:**
```
CLC  C ← 0    CLD  D ← 0    CLI  I ← 0    CLV  V ← 0
SEC  C ← 1    SED  D ← 1    SEI  I ← 1
```

**Other:**
```
NOP  No operation (1 byte)
```

## Flag Update Rules

### N (Negative)
Set to bit 7 of the result for: LDA, LDX, LDY, TAX, TAY, TXA, TYA, TSX,
PLA, PLP (from loaded P), ADC, SBC, AND, ORA, EOR, ASL, LSR, ROL, ROR,
INC, INX, INY, DEC, DEX, DEY, CMP, CPX, CPY, BIT.

### V (Overflow)
Set by ADC when the sign of the result differs from both inputs' signs:
`V = (A7 ^ result7) & (M7 ^ result7)` where 7 = bit 7.
Set by SBC analogously.
Set by BIT to bit 6 of the memory operand.

### Z (Zero)
Set when the result byte is zero.

### C (Carry)
- ADC: set if binary sum > 0xFF (unsigned overflow).
- SBC: set if *no* borrow — i.e., A ≥ M+(1-C) unsigned. Equivalent:
  C = NOT(borrow) = 1 if A ≥ M in the non-borrow interpretation.
- ASL/ROL: the old bit 7.
- LSR/ROR: the old bit 0.
- CMP/CPX/CPY: set if register ≥ operand unsigned.

## Public API

### MOS6502State (frozen dataclass)

```python
@dataclass(frozen=True)
class MOS6502State:
    # Registers
    a: int            # Accumulator (0–255)
    x: int            # Index X (0–255)
    y: int            # Index Y (0–255)
    s: int            # Stack pointer (0–255; effective address = 0x100+S)
    pc: int           # Program counter (0–65535)

    # Status flags (P register bits)
    flag_n: bool      # Negative
    flag_v: bool      # Overflow
    flag_b: bool      # Break (reflects pushed B; not a "real" flip-flop)
    flag_d: bool      # Decimal
    flag_i: bool      # Interrupt disable
    flag_z: bool      # Zero
    flag_c: bool      # Carry

    # Simulator metadata
    halted: bool      # True after BRK (treated as halt)
    memory: tuple[int, ...]   # Full 64KB snapshot (65536 bytes)
```

### MOS6502Simulator

```python
class MOS6502Simulator(Simulator[MOS6502State]):
    def reset(self) -> None:
        """Set all registers to power-on state.
        A=X=Y=0, S=0xFD, PC=0x0000, P=0x24 (I=1, unused=1).
        Clears memory. Does NOT read the RESET vector — caller sets PC
        directly via load().
        """

    def load(self, program: bytes, origin: int = 0x0000) -> None:
        """Write program bytes into memory at origin.
        Sets PC = origin. Length must be ≤ 65536 - origin.
        """

    def step(self) -> StepTrace:
        """Execute one instruction. Raises RuntimeError if halted."""

    def execute(
        self,
        program: bytes,
        origin: int = 0x0000,
        max_steps: int = 100_000,
    ) -> ExecutionResult[MOS6502State]:
        """Load and run until BRK or max_steps."""

    def get_state(self) -> MOS6502State: ...
    def set_input_port(self, port: int, value: int) -> None: ...
    def get_output_port(self, port: int) -> int: ...
```

The 6502 has no dedicated I/O ports (unlike the 8080). Ports are implemented
as memory-mapped I/O: reads from 0xFF00–0xFFEF read from input_ports (0–239);
writes to 0xFF00–0xFFEF write to output_ports. Port numbers map to addresses
0xFF00+port (ports 0–239). This gives a clean interface without changing the
core instruction set.

**Halt condition:** `BRK` (opcode 0x00) is treated as the halt instruction.
The simulator sets `halted=True` after executing BRK, consistent with how
all other simulators in this stack treat their halt instruction.

## SIM00 Protocol Compliance

| Method    | Behaviour |
|-----------|-----------|
| `reset()` | Clears all registers and memory; `S=0xFD`, `PC=0`, `P=0x24` |
| `load(b)` | Writes `b` at origin, sets `PC=origin` |
| `step()`  | Executes one instruction; raises `RuntimeError` if halted |
| `execute()` | Runs to BRK or max_steps; returns `ExecutionResult` |
| `get_state()` | Returns frozen `MOS6502State` snapshot |

## File Layout

```
code/packages/python/mos6502-simulator/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
├── src/
│   └── mos6502_simulator/
│       ├── __init__.py
│       ├── flags.py       # Flag computation helpers
│       ├── state.py       # MOS6502State dataclass
│       └── simulator.py   # MOS6502Simulator implementation
└── tests/
    ├── test_load_store.py     # LDA/LDX/LDY/STA/STX/STY all modes
    ├── test_arithmetic.py     # ADC/SBC (binary + decimal), INC/DEC
    ├── test_logical.py        # AND/ORA/EOR/BIT/ASL/LSR/ROL/ROR
    ├── test_branch.py         # All 8 branches, BEQ/BNE/etc.
    ├── test_stack.py          # PHA/PLA/PHP/PLP/JSR/RTS/RTI/BRK
    ├── test_transfer.py       # TAX/TAY/TXA/TYA/TSX/TXS
    ├── test_compare.py        # CMP/CPX/CPY
    ├── test_flags.py          # CLC/SEC/CLD/SED/CLI/SEI/CLV
    ├── test_addressing.py     # All 13 addressing modes
    ├── test_programs.py       # End-to-end programs
    └── test_protocol.py       # SIM00 protocol compliance
```

## Test Programs

### Fibonacci
Compute the first N Fibonacci numbers and store in memory:
```
LDX  #0        ; index = 0
LDA  #0        ; fib[0] = 0
STA  $10,X     ; store
INX
LDA  #1        ; fib[1] = 1
STA  $10,X
INX
; loop: fib[n] = fib[n-1] + fib[n-2]
```

### Multiply (8-bit)
```
; A = multiplicand, X = multiplier, result in $00-$01
LDA  #0
STA  $00
STA  $01
; loop: $00 += multiplicand * X using shift-add
```

### Sum 1..N
```
LDA  #0
LDX  #$0A    ; N=10
loop:
  STX  $00
  CLC
  ADC  $00
  DEX
  BNE  loop
BRK
; A = 55
```

## Comparison with Intel 8080

| Feature       | Intel 8080  | MOS 6502     |
|---------------|-------------|--------------|
| Year          | 1974        | 1975         |
| Price (1977)  | $179        | $25          |
| Registers     | 7 × 8-bit   | A, X, Y (8-bit) |
| Stack         | 16-bit SP   | 8-bit SP (page 1 only) |
| Flags         | S,Z,AC,P,CY | N,V,B,D,I,Z,C |
| Addressing    | Limited     | 13 modes, zero page |
| Decimal mode  | No          | Yes (BCD ADC/SBC) |
| I/O           | IN/OUT ports | Memory-mapped |
| Transistors   | ~6,000      | ~3,510        |
| Word width    | 8-bit       | 8-bit         |
| Bus width     | 16-bit addr | 16-bit addr   |

The 6502's elegant design — fewer transistors than the 8080, yet more
addressing modes — reflects Chuck Peddle's philosophy: make the common
case fast (zero page), keep the chip cheap, and trust programmers to use
registers efficiently.

## Historical Notes

- Designed by Chuck Peddle and his team at MOS Technology after leaving Motorola
- First shown publicly at WESCON 1975 for $25 (Motorola 6800 cost $175)
- Steve Wozniak bought one on the show floor and designed the Apple I around it
- The 6502 core (as 65C02/65816) still ships in billions of embedded chips today
- The Ricoh 2A03 (NES) is a 6502 variant with the decimal mode disabled

See implementation: `code/packages/python/mos6502-simulator/`
