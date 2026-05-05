# mips-r2000-simulator

Behavioral simulator for the **MIPS R2000 (1985)** microprocessor — Layer 07q in the coding-adventures CPU simulator series.

## What is the MIPS R2000?

The MIPS R2000 is the first commercially successful RISC processor, designed by John Hennessy's team at Stanford University and manufactured by MIPS Computer Systems starting in 1985.  It proved that the "Reduced Instruction Set Computer" philosophy could outperform complex CISC designs in real workloads.

Key facts:
- **First commercial RISC processor** (1985)
- Used in SGI IRIS workstations, DEC DECstations, PlayStation 1, PlayStation 2, Nintendo 64
- Foundation of Patterson & Hennessy *Computer Organization and Design*
- 32-bit word-addressable, big-endian
- 32 general-purpose registers (R0 hardwired zero)
- HI:LO registers for 64-bit multiply results and divide quotient/remainder
- Fixed-width 32-bit instructions in three formats: R, I, J
- Load-store architecture (only LW/SW variants touch memory)

## How it fits in the stack

This package implements the `Simulator[MIPSState]` protocol from `coding-adventures-simulator-protocol`.  It is purely behavioral — no cycle counting, no branch delay slots, no pipeline simulation.

```
coding-adventures-simulator-protocol  (SIM00 interface)
└── coding-adventures-mips-r2000-simulator  (this package)
```

## Usage

```python
from mips_r2000_simulator import MIPSSimulator

sim = MIPSSimulator()

# Encode a simple program: ADDIU $t0, $zero, 42; HALT
import struct
ADDIU = struct.pack(">I", (0x09 << 26) | (0 << 21) | (8 << 16) | 42)
HALT  = struct.pack(">I", 0x0000_000C)   # SYSCALL

result = sim.execute(ADDIU + HALT)
print(result.ok)        # True
print(result.steps)     # 2
print(sim._regs[8])     # 42  ($t0)
```

### Instruction formats

```
R-type:  [op:6=0][rs:5][rt:5][rd:5][shamt:5][funct:6]
I-type:  [op:6][rs:5][rt:5][imm16:16]
J-type:  [op:6][target26:26]
```

### HALT convention

`SYSCALL` (opcode 0, funct 0x0C = `0x0000000C`) halts the simulator.  This matches real MIPS Linux convention where `$v0=4001` exits the process via SYSCALL.

### Simplifications

- **No branch delay slots** — branches take effect immediately
- **64 KB memory** — PC and addresses wrap at 16 bits
- **No CP0 / TLB** — exceptions raise Python `ValueError` instead
- **No FPU (COP1)**

## Development

```bash
uv venv .venv --quiet --no-project
uv pip install --python .venv -e ../simulator-protocol -e .[dev] --quiet
.venv/bin/python -m pytest
```
