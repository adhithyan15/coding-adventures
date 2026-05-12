# powerpc601-simulator

**Layer 07u** — Behavioral simulator for the **PowerPC 601 (1992)** integer
instruction set, implementing the SIM00 `Simulator[PowerPC601State]` protocol.

## Background

The PowerPC 601 was the first processor from the AIM alliance (Apple, IBM,
Motorola) and powered the original Power Macintosh line in March 1994.
It brought RISC principles — clean 32-bit fixed-width instructions,
large register file, load/store architecture — to consumer desktops at a time
when Intel's x86 still dominated with CISC complexity.

The PowerPC ISA introduced:
- **32 general-purpose 32-bit registers** (vs. 8 on x86)
- **Fixed 32-bit instruction encoding** — no variable-length prefixes
- **Condition Register (CR)** — 8 × 4-bit fields (LT, GT, EQ, SO per field)
- **Count Register (CTR)** — hardware loop counter (bdnz/bdz)
- **Link Register (LR)** — dedicated return address register (bl/blr)
- **Big-endian** byte ordering

## Installation

```bash
pip install coding-adventures-powerpc601-simulator
```

Or from source with uv:

```bash
uv pip install -e ".[dev]"
```

## Quick Start

```python
from powerpc601_simulator import (
    PowerPC601Simulator,
    HALT, d_form, xo_form, b_form,
    BO_BDNZ, SPR_CTR,
)
from powerpc601_simulator.simulator import (
    PO_ADDI, PO_BC, PO_X31, XO_MTSPR, XO_ADD,
)

# Sum 1 + 2 + ... + 10 = 55
prog = (
    d_form(PO_ADDI, 5, 0, 10)                    # r5 = 10
    + xfx_form(PO_X31, 5, SPR_CTR, XO_MTSPR)    # CTR = 10
    + d_form(PO_ADDI, 4, 0, 1)                   # r4 = 1 (counter)
    + d_form(PO_ADDI, 3, 0, 0)                   # r3 = 0 (accumulator)
    # loop:
    + xo_form(PO_X31, 3, 3, 4, 0, XO_ADD)       # r3 += r4
    + d_form(PO_ADDI, 4, 4, 1)                   # r4++
    + b_form(PO_BC, BO_BDNZ, 0, -8)             # bdnz
    + HALT
)

sim = PowerPC601Simulator()
result = sim.execute(prog)
assert result.final_state.r3 == 55
```

## Architecture Summary

| Feature          | Value                                        |
|------------------|----------------------------------------------|
| Year             | 1992                                         |
| Transistors      | 2.8 million                                  |
| Clock speed      | 50–80 MHz                                    |
| Word width       | 32-bit                                       |
| Instruction size | 32-bit fixed                                 |
| GPRs             | 32 × 32-bit (GPR0–GPR31)                   |
| Special regs     | LR, CTR, XER, CR                            |
| Endianness       | Big-endian                                   |
| Memory (sim)     | 64 KiB byte-addressed flat                  |

## Simulated Instructions

| Category          | Instructions                                      |
|-------------------|---------------------------------------------------|
| Arithmetic        | add, addc, adde, subf, subfic, neg, mullw, divw, divwu |
| Immediate arith   | addi, addis                                       |
| Logical           | and, or, xor, nand, nor, cntlzw                 |
| Logical immediate | andi., andis., ori, oris, xori                  |
| Shift             | slw, srw, sraw, srawi                            |
| Compare           | cmpw, cmplw, cmpwi, cmplwi                       |
| Load              | lwz, lwzu, lbz, lbzu, lhz, lhzu, lha           |
| Store             | stw, stwu, stb, stbu, sth                        |
| Branch            | b, bl, bc (blt/bge/bgt/ble/beq/bne/bdnz/bdz)  |
| Branch LR/CTR     | blr, bctr, bctrl                                 |
| Special regs      | mfspr, mtspr (LR, CTR, XER), mfcr, mtcrf        |
| HALT              | 0x00000000 (all-zeros word)                      |

## SIM00 Protocol

Implements `Simulator[PowerPC601State]` from `coding-adventures-simulator-protocol`:

```python
sim.reset()                        # Zero all state
sim.load(program: bytes)           # Reset + load program at address 0
trace = sim.step()                 # Execute one instruction
result = sim.execute(program)      # Load + run to HALT
state = sim.get_state()            # Frozen PowerPC601State snapshot
```

## Simplifications

- No floating-point (FPR0–31 not simulated)
- No MMU / virtual memory translation
- OE=0 / Rc=0: arithmetic never sets XER[SO/OV] or CR0 automatically
- No hardware exceptions or interrupts
- GPR0-as-zero applies only in EA calculations (loads/stores/addi)
- Memory: 64 KiB (vs. 4 GiB on real hardware)
