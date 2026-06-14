# intel8051-simulator — Layer 07p

A Python behavioral simulator for the **Intel 8051** (MCS-51, 1980)
microcontroller, part of the `coding-adventures` simulator series.

## What is the 8051?

The Intel 8051 is the most-manufactured CPU architecture in history, with
over **20 billion units produced**.  Introduced in 1980 as Intel's first
single-chip microcontroller, it integrated CPU + RAM + ROM + I/O ports +
timers + serial port on a single die — defining the microcontroller as a
product category.

The 8051 uses a **Harvard architecture**: code memory and data memory are
separate address spaces accessed via separate buses.  This allows instruction
fetch and data access to occur simultaneously.

## Features implemented

- Full Harvard memory model: 64 KB code, 256-byte internal RAM (including
  SFRs at 0x80–0xFF), 64 KB external data memory
- All 8051 addressing modes: register, direct, register-indirect, immediate,
  indexed (MOVC), external (MOVX)
- Complete instruction set: data transfer, arithmetic, logic, bit manipulation,
  branches (JZ/JNZ/JC/JNC/JB/JNB/JBC/CJNE/DJNZ), unconditional jumps
  (LJMP/AJMP/SJMP/JMP @A+DPTR), subroutines (LCALL/ACALL/RET/RETI)
- Four register banks (selected via PSW.RS1:RS0)
- Bit-addressable RAM area (0x20–0x2F) and bit-addressable SFRs
- All PSW flags: CY, AC, OV, P (parity auto-recomputed after every ACC change)
- SIM00 protocol: `Simulator[I8051State]`

## Usage

```python
from intel8051_simulator import I8051Simulator

sim = I8051Simulator()

# Encode a small program: sum 1+2+3 = 6
program = bytes([
    0x78, 0x03,    # MOV R0, #3
    0x28,          # ADD A, R0  (loop top)
    0xD8, 0xFD,    # DJNZ R0, -3
    0xA5,          # HALT
])

result = sim.execute(program)
print(result.ok)           # True
print(sim._iram[0xE0])    # 6  (accumulator = 1+2+3)
```

## Architecture in brief

| Feature          | Value                                      |
|------------------|--------------------------------------------|
| Architecture     | Harvard (separate code/data buses)          |
| Data width       | 8-bit                                      |
| Address width    | 16-bit (code + xdata), 8-bit (iram)        |
| Internal RAM     | 256 bytes (128 general + 128 SFR)          |
| External data    | 64 KB (XDATA via MOVX)                     |
| Code memory      | 64 KB                                      |
| Register banks   | 4 (R0–R7 each, selected by PSW)            |
| Flags            | CY, AC, F0, OV, P (in PSW SFR)            |
| Bit-addressable  | RAM 0x20–0x2F, bit-addressable SFRs        |
| HALT sentinel    | Opcode 0xA5 (undefined on real hardware)   |

## Package layout

```
src/intel8051_simulator/
├── __init__.py      # exports: I8051Simulator, I8051State
├── state.py         # I8051State frozen dataclass + SFR constants
├── flags.py         # add8_flags, sub8_flags, da_flags arithmetic helpers
└── simulator.py     # I8051Simulator implementation
tests/
├── test_protocol.py    # SIM00 compliance
├── test_instructions.py # per-instruction tests
├── test_programs.py    # end-to-end programs
└── test_coverage.py    # edge cases and flag arithmetic
```

## How it fits in the stack

This is Layer 07p in the `coding-adventures` simulator series:

```
07d  Intel 4004 (1971)   — first commercial microprocessor
07i  Intel 8080 (1974)   — 8-bit general-purpose CPU
07m  Intel 8086 (1978)   — 16-bit, ancestor of x86
07p  Intel 8051 (1980)   — 8-bit microcontroller (this package)
```

See `code/specs/07p-intel-8051-simulator.md` for the full specification.
