# motorola-68000-simulator

**Layer 07n** — Motorola 68000 (1979) behavioral simulator.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
series of from-scratch computer science implementations.

---

## What is this?

A Python behavioral simulator for the **Motorola MC68000** microprocessor —
the clean-ISA 32-bit CPU that powered the Apple Macintosh, Commodore Amiga,
Atari ST, Sega Genesis, and countless Unix workstations of the 1980s.

The simulator implements the `Simulator[M68KState]` protocol defined in
`simulator-protocol` (SIM00), making it interchangeable with every other
architecture simulator in this repo.

Key properties simulated:
- 8 × 32-bit data registers (D0–D7)
- 8 × 32-bit address registers (A0–A7, A7 = supervisor stack pointer)
- Linear 24-bit address space (16 MB, no segments)
- Big-endian byte ordering
- Full CCR (X/N/Z/V/C flags) updated per-instruction
- All 14 addressing modes
- ~50 instructions covering the complete 68000 programming model

---

## Quick start

```python
from motorola_68000_simulator import M68KSimulator

sim = M68KSimulator()

# MOVEQ #10, D0   (0x700A)
# MOVEQ #20, D1   (0x720E... wait, 0x7214)
# ADD.L D1, D0    (0xD081)
# STOP #0x2700    (0x4E72 0x2700)
prog = bytes([
    0x70, 0x0A,             # MOVEQ #10, D0
    0x72, 0x14,             # MOVEQ #20, D1
    0xD0, 0x81,             # ADD.L D1, D0
    0x4E, 0x72, 0x27, 0x00, # STOP #0x2700
])
result = sim.execute(prog)
assert result.ok
assert result.final_state.d0 == 30
```

---

## Architecture overview

### The 68000's orthogonal ISA

Unlike the 8086 where certain operations only work on certain registers
(e.g. `LOOP` only uses CX, `MUL` always targets AX:DX), the 68000's
instruction set is nearly **fully orthogonal**: almost any instruction can
work with any data register and any addressing mode.

```
MOVE.W D0, D1       ; register → register
MOVE.W D0, (A1)     ; register → memory (indirect)
MOVE.W D0, (A1)+    ; register → memory (indirect, postincrement)
MOVE.W D0, -(A1)    ; register → memory (indirect, predecrement)
MOVE.W D0, 8(A1)    ; register → memory (base + displacement)
MOVE.W D0, 0x1000   ; register → absolute address
```

All of the above use the same `MOVE` instruction with different addressing modes.

### The 68000 vs the 8086

The 8086 (Intel, 1978) and 68000 (Motorola, 1979) were the dominant CPUs of
the early PC era. Their design philosophies could not be more different:

```
Feature          8086                      68000
─────────────────────────────────────────────────────────
Byte order       Little-endian             Big-endian
Memory model     Segmented (1 MB)          Linear flat (16 MB)
Registers        4 GP regs, specialized    8 data + 8 address
Register widths  16-bit (8-bit halves)     32-bit (full internal)
Arithmetic       Accumulator-centric       Orthogonal
Addressing       6 modes (practical)       14 modes
Stack           PUSH/POP instructions     MOVE -(SP) / (SP)+
```

### Status register layout

```
Bit: 15  14  13  12  11  10   9   8   7   6   5   4   3   2   1   0
      T1  T0   S   M   0  I2  I1  I0   0   0   0   X   N   Z   V   C
      ─────────── system ──────────────    ───────── CCR ─────────────
```

- **X** (extend): same as C on ADD/SUB; used by ADDX/SUBX extended arithmetic
- **N** (negative): copy of MSB of result
- **Z** (zero): result is zero
- **V** (overflow): signed overflow
- **C** (carry): unsigned carry/borrow

### Memory model

```
0x000000 – 0x0003FF   Exception vector table (256 × 4-byte vectors)
0x001000              Program load address (PC starts here after reset)
0x00F000              Stack grows down from here (A7 / SSP)
0xFFFFFF              Top of 16 MB address space
```

---

## Running tests

```bash
cd code/packages/python/motorola-68000-simulator
uv venv && uv pip install -e ../simulator-protocol -e .[dev]
python -m pytest tests/ -v
```

Expected output: 280+ tests pass, ≥95% coverage.

---

## Package structure

```
src/motorola_68000_simulator/
├── __init__.py     Public API: M68KSimulator, M68KState
├── state.py        M68KState frozen dataclass + documentation
├── flags.py        CCR flag computation helpers
└── simulator.py    M68KSimulator: decode/execute loop
```

---

## Layer context

This package is part of a learning series on CPU architectures:

| Layer | Chip               | Year | Key innovation |
|-------|--------------------|------|----------------|
| 07d   | Intel 4004         | 1971 | First commercial microprocessor |
| 07f   | Intel 8008         | 1972 | 8-bit, first byte-oriented uP |
| 07i   | Intel 8080         | 1974 | Modern 8-bit ISA, CP/M era |
| 07j   | MOS 6502           | 1975 | Apple II, NES, Atari 2600 |
| 07k   | Zilog Z80          | 1976 | CP/M, ZX Spectrum, Game Boy |
| 07m   | Intel 8086         | 1978 | x86 origin, segmented memory |
| **07n** | **Motorola 68000** | **1979** | **32-bit orthogonal ISA, flat memory** |
