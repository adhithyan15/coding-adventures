# Spec 07k — Zilog Z80 Behavioral Simulator

## Overview

The **Zilog Z80** (1976) is an 8-bit microprocessor designed by Federico Faggin,
Masatoshi Shima, and Ralph Ungermann after they left Intel. It was designed as a
superset of the Intel 8080: every valid 8080 opcode is a valid Z80 opcode with
identical semantics, so all 8080 software runs unmodified. The Z80 then adds a
richer instruction set, two index registers, an alternate register bank, and a
more flexible interrupt system.

The Z80 powered the most popular 8-bit personal computers:
- **TRS-80** (1977, Tandy) — first mass-market home computer
- **Sinclair ZX80 / ZX81 / ZX Spectrum** (1980–1982)
- **CP/M machines** — the dominant business platform before IBM PC
- **MSX** (1983) — Japanese standard
- **Coleco ColecoVision** / **Sega Master System** game consoles

Microsoft BASIC (the direct descendant of the Altair BASIC written for the 8080)
was ported to all of these Z80 platforms. Altair BASIC itself ran on the Z80
without modification because the Z80 is binary-compatible with the 8080.

This spec defines Layer **07k** — a Python behavioral simulator for the Z80
following the SIM00 `Simulator[Z80State]` protocol.

---

## Architecture

### Registers

#### Main register bank
| Name | Width | Description |
|------|-------|-------------|
| A    | 8-bit | Accumulator |
| F    | 8-bit | Flags register (see below) |
| B, C | 8-bit | General-purpose; BC = 16-bit pair |
| D, E | 8-bit | General-purpose; DE = 16-bit pair |
| H, L | 8-bit | General-purpose; HL = 16-bit pair (often used as memory pointer) |

#### Alternate register bank (unique to Z80)
A second set of A', F', B', C', D', E', H', L' that can be swapped with the main
bank using `EX AF, AF'` and `EXX`. Only one bank is active at a time; the
alternate bank is NOT directly addressable — you can only reach it by swapping.

#### Special registers
| Name | Width | Description |
|------|-------|-------------|
| IX   | 16-bit | Index register X — base for `(IX+d)` addressing |
| IY   | 16-bit | Index register Y — base for `(IY+d)` addressing |
| SP   | 16-bit | Stack pointer |
| PC   | 16-bit | Program counter |
| I    | 8-bit  | Interrupt vector base (used in IM 2) |
| R    | 8-bit  | Memory refresh counter (incremented each instruction; low 7 bits) |

### Flags register (F)

```
Bit:  7   6   5   4   3   2   1   0
Flag: S   Z   Y   H   X   P/V N   C
```

| Bit | Name | Description |
|-----|------|-------------|
| 7   | S    | Sign — copy of bit 7 of the result |
| 6   | Z    | Zero — result is zero |
| 5   | Y    | Undocumented — copy of bit 5 of the result |
| 4   | H    | Half-carry — carry from bit 3 to bit 4 |
| 3   | X    | Undocumented — copy of bit 3 of the result |
| 2   | P/V  | Parity (logical ops) / Overflow (arithmetic ops) |
| 1   | N    | Add/Subtract — last operation was subtraction |
| 0   | C    | Carry |

**Differences from Intel 8080 flags:**
- Z80 has N (subtract) and H (half-carry) as named flags (8080 has undocumented equivalents)
- Z80 P/V combines parity (for AND/OR/XOR) and overflow (for ADD/SUB) into one flag
- Bits 3 and 5 are "undocumented" but have defined behaviour in the Z80 silicon

**For this simulator:** we track S, Z, H, P/V, N, C accurately. Bits Y (5) and X (3)
are tracked for completeness but their exact values are not tested exhaustively.

### Addressing modes

| Mode | Syntax | Description |
|------|--------|-------------|
| Immediate byte | `n` | 8-bit constant in next byte |
| Immediate word | `nn` | 16-bit constant in next two bytes (little-endian) |
| Register | `r` | One of A, B, C, D, E, H, L |
| Register indirect | `(HL)` | Memory at address in HL |
| Indexed | `(IX+d)`, `(IY+d)` | Memory at IX/IY + signed 8-bit displacement |
| Direct | `(nn)` | Memory at 16-bit address |
| Register pair | `rp` | BC, DE, HL, SP, AF |
| Relative | `e` | PC-relative signed offset (branches) |
| Bit | `b, r` | Bit number 0–7 in register or memory |
| Implied / Accumulator | — | No explicit operand |

### Instruction encoding

The Z80 uses a single-byte opcode for most instructions. Four prefix bytes extend
the opcode space:

| Prefix | Purpose |
|--------|---------|
| `0xCB` | Bit manipulation and rotate/shift on all registers |
| `0xED` | Extended instructions: block moves, I/O, 16-bit arithmetic |
| `0xDD` | Use IX instead of HL (e.g. `DD 46 05` = `LD B, (IX+5)`) |
| `0xFD` | Use IY instead of HL |
| `0xDD CB` | Bit operations on `(IX+d)` |
| `0xFD CB` | Bit operations on `(IY+d)` |

---

## Instruction set

### Unprefixed (compatible with Intel 8080)

All 244 documented Intel 8080 opcodes have identical semantics on the Z80.
The following are the main groups:

**Load 8-bit:** `LD r, r'` — `LD r, n` — `LD r, (HL)` — `LD (HL), r` — `LD (HL), n`
`LD A, (BC)` — `LD A, (DE)` — `LD A, (nn)` — `LD (BC), A` — `LD (DE), A` — `LD (nn), A`
`LD A, I` — `LD A, R` — `LD I, A` — `LD R, A`

**Load 16-bit:** `LD rp, nn` — `LD HL, (nn)` — `LD (nn), HL` — `LD SP, HL`
`PUSH rp` — `POP rp`

**Arithmetic 8-bit:** `ADD A, r/n/(HL)` — `ADC A, r/n/(HL)` — `SUB r/n/(HL)`
`SBC A, r/n/(HL)` — `AND r/n/(HL)` — `OR r/n/(HL)` — `XOR r/n/(HL)`
`CP r/n/(HL)` — `INC r` — `INC (HL)` — `DEC r` — `DEC (HL)`

**Arithmetic 16-bit:** `ADD HL, rp` — `INC rp` — `DEC rp`

**Accumulator/flag:** `RLCA` — `RRCA` — `RLA` — `RRA` — `DAA` — `CPL` — `SCF` — `CCF`

**Branch:** `JP nn` — `JP cc, nn` — `JR e` — `JR cc, e` (cc: NZ, Z, NC, C)
`DJNZ e` — `CALL nn` — `CALL cc, nn` — `RET` — `RET cc` — `RETI` — `RETN`
`RST p` (restart: p = 0, 8, 16, 24, 32, 40, 48, 56)

**Exchange:** `EX DE, HL` — `EX AF, AF'` — `EXX` — `EX (SP), HL`

**Misc:** `NOP` — `HALT` — `DI` — `EI`

### CB-prefixed: bit manipulation

`RLC r/(HL)` — `RRC r/(HL)` — `RL r/(HL)` — `RR r/(HL)`
`SLA r/(HL)` — `SRA r/(HL)` — `SRL r/(HL)` — `SLL r/(HL)` (undocumented but present)
`BIT b, r/(HL)` — `RES b, r/(HL)` — `SET b, r/(HL)`

### ED-prefixed: extended

**16-bit arithmetic with carry:** `ADC HL, rp` — `SBC HL, rp`
**16-bit load:** `LD rp, (nn)` — `LD (nn), rp`
**Special accumulator:** `NEG` — `RLD` — `RRD`
**Interrupt:** `IM 0` — `IM 1` — `IM 2`
**Special load:** `LD A, I` — `LD A, R` — `LD I, A` — `LD R, A`

**Block operations (unique to Z80):**
| Mnemonic | Description |
|----------|-------------|
| `LDIR`   | Block copy (HL)→(DE), inc HL/DE, dec BC, repeat until BC=0 |
| `LDDR`   | Block copy backwards (dec HL/DE), repeat until BC=0 |
| `LDI`    | Single copy step (HL)→(DE), inc, dec BC |
| `LDD`    | Single copy step backwards |
| `CPIR`   | Block search: compare A with (HL), inc HL, dec BC, repeat until match or BC=0 |
| `CPDR`   | Block search backwards |
| `CPI`    | Single compare step |
| `CPD`    | Single compare step backwards |
| `INIR`   | Block input from port (C) to (HL), repeat |
| `OTIR`   | Block output from (HL) to port (C), repeat |
| `INI`    | Single input step |
| `OUTI`   | Single output step |
| `INDR`   | Block input backwards |
| `OTDR`   | Block output backwards |
| `IND`    | Single input step backwards |
| `OUTD`   | Single output step backwards |

**I/O:** `IN A, (n)` — `IN r, (C)` — `OUT (n), A` — `OUT (C), r`

### DD/FD-prefixed: index register instructions

The DD prefix replaces HL with IX throughout most instructions:
- `LD r, (IX+d)` — `LD (IX+d), r` — `LD (IX+d), n`
- `ADD A, (IX+d)` — `ADC A, (IX+d)` — `SUB (IX+d)` — etc.
- `ADD IX, rp` — `INC IX` — `DEC IX`
- `LD IX, nn` — `LD IX, (nn)` — `LD (nn), IX`
- `PUSH IX` — `POP IX` — `EX (SP), IX` — `JP (IX)`

FD prefix does the same for IY.

---

## Flag behaviour

### S, Z flags
Set from the result: S = bit 7, Z = (result == 0).

### H (half-carry)
- ADD/ADC: carry from bit 3 to bit 4
- SUB/SBC/CP: borrow from bit 4 (set if lower nibble of A < lower nibble of operand + borrow)
- AND: set; OR/XOR: reset
- INC: carry from bit 3; DEC: borrow from bit 3

### P/V (parity / overflow)
- Logical ops (AND, OR, XOR): parity of result (1 if even number of 1-bits)
- Arithmetic (ADD, ADC, SUB, SBC): overflow (result outside −128..127)
- IN r, (C): parity of result

### N (subtract)
- Set for SUB, SBC, NEG, CP, DEC, CPIR/CPDR, etc.
- Reset for ADD, ADC, INC, AND, OR, XOR, etc.

### C (carry)
Standard carry/borrow out of bit 7.

### DAA (decimal adjust accumulator)
After ADD/ADC/SUB/SBC on BCD values, DAA corrects A to valid BCD:
- Uses H, N, C flags to determine what correction to apply
- Adds 0x06 to low nibble if H=1 or low nibble > 9
- Adds 0x60 to high nibble if C=1 or value > 0x99
- Updates S, Z, P (parity), H, C; N unchanged

---

## I/O

The Z80 has a separate 8-bit I/O address space (ports 0x00–0xFF):
- `IN A, (n)` — read from port `n` into A (n is immediate; port = n)
- `OUT (n), A` — write A to port `n`
- `IN r, (C)` — read from port BC low byte (C) into register r
- `OUT (C), r` — write register r to port C

This simulator maps ports 0–255 to `input_ports` / `output_ports` arrays per
the SIM00 protocol. Block I/O instructions (INIR, OTIR, etc.) use the same arrays.

---

## Interrupts (simplified for simulation)

The Z80 has three interrupt modes:
- **IM 0**: Device places an opcode on the data bus during interrupt; execute it
- **IM 1**: Jump to address 0x0038 (simplest mode, used by many home computers)
- **IM 2**: Vectored — I register (high byte) + device byte (low byte) = address of handler pointer

For this simulator, interrupts are **not triggered autonomously**. The `interrupt(data)`
method allows callers to fire a maskable interrupt manually. The simulator:
1. Checks if interrupts are enabled (IFF1=True)
2. Pushes PC, clears IFF1/IFF2
3. In IM 0: executes the provided byte as an RST or NOP
4. In IM 1: sets PC = 0x0038
5. In IM 2: reads vector from memory[I*256 + data] and jumps there

NMI (non-maskable interrupt): push PC, jump to 0x0066, IFF2 = IFF1, IFF1 = False.

**HALT** suspends the CPU until an interrupt occurs. In this simulator, HALT sets
`halted=True` to terminate the execution loop (same convention as BRK on 6502,
HLT on 8080).

---

## Public API

```python
from dataclasses import dataclass
from simulator_protocol import Simulator, ExecutionResult, StepTrace

@dataclass(frozen=True)
class Z80State:
    # Main registers
    a: int          # 0–255
    b: int; c: int
    d: int; e: int
    h: int; l: int

    # Alternate registers
    a_: int         # A'
    f_: int         # F' (packed flags byte)
    b_: int; c_: int
    d_: int; e_: int
    h_: int; l_: int

    # Index / special registers
    ix: int         # 0–65535
    iy: int         # 0–65535
    sp: int         # 0–65535
    pc: int         # 0–65535
    i:  int         # 0–255, interrupt vector base
    r:  int         # 0–255, memory refresh (low 7 bits increment per instruction)

    # Flags (main bank, unpacked for convenience)
    flag_s: bool    # Sign
    flag_z: bool    # Zero
    flag_h: bool    # Half-carry
    flag_pv: bool   # Parity / Overflow
    flag_n: bool    # Add/Subtract
    flag_c: bool    # Carry

    # Interrupt flip-flops
    iff1: bool      # Maskable interrupt enable
    iff2: bool      # Shadow of IFF1 (saved during NMI)
    im:   int       # Interrupt mode 0/1/2

    # Simulator state
    halted: bool
    memory: tuple[int, ...]   # 65536 bytes

    def f_byte(self) -> int:
        """Pack main flags into the F register byte."""
        ...

class Z80Simulator(Simulator[Z80State]):
    def reset(self) -> None: ...
    def load(self, program: bytes, origin: int = 0x0000) -> None: ...
    def step(self) -> StepTrace: ...
    def execute(self, program: bytes, origin: int = 0x0000,
                max_steps: int = 100_000) -> ExecutionResult[Z80State]: ...
    def get_state(self) -> Z80State: ...
    def set_input_port(self, port: int, value: int) -> None: ...
    def get_output_port(self, port: int) -> int: ...
    def interrupt(self, data: int = 0xFF) -> None:
        """Fire a maskable interrupt (IRQ). data is the byte placed on the bus (IM 0)."""
        ...
    def nmi(self) -> None:
        """Fire a non-maskable interrupt."""
        ...
```

### Reset state
- All main and alternate registers: 0
- IX, IY, SP, PC: 0
- I, R: 0
- F = 0xFF (all flags set — matches real Z80 power-on)
- F' = 0xFF
- IFF1 = IFF2 = False (interrupts disabled)
- IM = 0
- Memory: 64 KiB of zeros
- halted = False

### Load
- Validates `0 <= origin <= 0xFFFF`
- Copies program bytes into memory with wrap at 0xFFFF
- Sets PC = origin, halted = False

---

## Package layout

```
code/packages/python/z80-simulator/
├── BUILD
├── pyproject.toml          # name="coding-adventures-z80-simulator"
├── README.md
├── CHANGELOG.md
└── src/
    └── z80_simulator/
        ├── __init__.py
        ├── state.py         # Z80State frozen dataclass
        ├── flags.py         # Flag helpers: compute_sz, compute_parity,
        │                    #   compute_overflow, daa, pack_f, unpack_f
        ├── simulator.py     # Z80Simulator — full instruction dispatch
        └── py.typed
tests/
    ├── test_flags.py
    ├── test_protocol.py
    ├── test_load_store.py
    ├── test_arithmetic.py
    ├── test_logical.py
    ├── test_rotate_shift.py
    ├── test_bit_ops.py          # CB-prefix: BIT/SET/RES
    ├── test_block_ops.py        # LDIR/LDDR/CPIR etc.
    ├── test_index_regs.py       # (IX+d), (IY+d) addressing
    ├── test_branch.py           # JP/JR/DJNZ/CALL/RET/RST
    ├── test_exchange.py         # EX, EXX, alternate bank
    ├── test_io.py               # IN/OUT/INIR/OTIR
    ├── test_interrupts.py       # IM 0/1/2, NMI
    └── test_programs.py         # End-to-end: bubble sort, block copy, search
```

---

## Implementation notes

### 8080 compatibility
The simplest approach: implement all Z80 instructions, and verify that the 8080
subset produces identical results to the 8080 simulator (07i). Any instruction
with an 8080 opcode that was already tested in 07i should give the same result.

### Prefix handling
Read one byte at a time. If it is `0xDD`, `0xFD`, `0xED`, or `0xCB`, read
the next byte as the actual opcode (and for `0xDD CB` / `0xFD CB`, read the
displacement and then the opcode). A clean approach:

```python
def _fetch_and_execute(self) -> str:
    prefix = self._read_pc()
    if prefix == 0xCB:
        return self._exec_cb()
    if prefix == 0xED:
        return self._exec_ed()
    if prefix == 0xDD:
        return self._exec_dd()   # may itself consume CB for DDCB
    if prefix == 0xFD:
        return self._exec_fd()   # may itself consume CB for FDCB
    return self._exec_main(prefix)
```

### Index register displacement
`(IX+d)` reads a **signed** 8-bit displacement. If `d >= 0x80`, it is negative:
`d = d - 256`.  Effective address: `(ix + d) & 0xFFFF`.

### Block instructions (LDIR etc.)
These loop inside a single `step()` call. The real Z80 takes multiple machine
cycles per iteration, but for a behavioral simulator, complete the full loop in
one step and return a description showing how many bytes were moved.

### DAA
Follow the standard Z80 DAA table exactly:
- After ADD: if C or A > 0x99, add 0x60 to A and set C; if H or (A & 0x0F) > 9, add 6 to A
- After SUB: if C, subtract 0x60; if H, subtract 6
- Update S, Z, P (parity), H; N unchanged; C updated

### R register
Increment low 7 bits of R after each instruction (main opcode fetch). Prefixed
instructions each increment R once. Not tested exhaustively but tracked in state.

---

## Coverage target

≥ 80% (target 90%+). The Z80 has a large instruction set but most instructions
share flag logic, so high coverage is achievable with systematic test design.

---

## Dependencies

- `coding-adventures-simulator-protocol` (SIM00)

---

## Relationship to other layers

| Layer | Machine | Z80 relationship |
|-------|---------|-----------------|
| 07i   | Intel 8080 | Z80 is a superset — every 8080 opcode is valid Z80 |
| 07j   | MOS 6502   | Contemporary (1975 vs 1976), rival 8-bit architecture |
| 07k   | **Z80**    | This spec |
| 07k2  | Z80 gate-level | Future: transistor-level simulation of Z80 silicon |
