# coding-adventures-aarch64-gatelevel

**AArch64 (ARMv8-A, 2011) gate-level simulator — Layer 07v2**

This package implements a behaviorally correct AArch64 processor simulator
where **every data-path operation routes through logic gate primitives**
(`AND`, `OR`, `XOR`, `NOT`) and a ripple-carry adder.  Registers are stored
as lists of bits (flip-flop arrays), not Python integers.

## What it is

Layer 07v2 in the coding-adventures simulator stack.  It sits alongside the
behavioral AArch64 simulator (Layer 07) but replaces Python native arithmetic
with gate-level operations:

```
Layer 01 — logic-gates       (AND/OR/XOR/NOT primitives)
Layer 02 — arithmetic        (ripple_carry_adder)
   ...
Layer 07 — aarch64-simulator (behavioral, Python arithmetic)
Layer 07v2 — aarch64-gatelevel (this package — gate-level)
```

## What it simulates

ARMv8-A AArch64, circa 2011 (the first 64-bit ARM architecture).

Instruction set subset:
- **Data processing (immediate)**: ADD, ADDS, SUB, SUBS, MOVZ, MOVN, MOVK
- **Data processing (register)**: ADD, ADDS, SUB, SUBS (shifted register)
- **Logical (immediate)**: AND, ORR, EOR, ANDS (bitmask immediate)
- **Logical (register)**: AND, ORR, EOR, ANDS, BIC, ORN, EON, BICS
- **Branches**: B, BL, B.cond, CBZ, CBNZ, TBZ, TBNZ, BR, BLR, RET
- **Load/Store**: STRB, LDRB, LDRSB, STRH, LDRH, LDRSH, STR32, LDR32, LDRSW, STR, LDR
- **Multiply/Accumulate**: MADD, MSUB (MUL, MNEG aliases), SMULH, UMULH
- **Division**: UDIV, SDIV
- **Variable shifts**: LSLV, LSRV, ASRV, RORV
- **1-source**: CLZ, RBIT, REV, REV16, REV32
- **Conditional select**: CSEL, CSINC, CSINV, CSNEG

## Gate-level constraint

The critical constraint: **no Python operators (+, -, &, |, ^, ~) on register
values**.  All data-path arithmetic/logic routes through:

- `add_64bit` / `sub_64bit` / `add_32bit` / `sub_32bit` — via `ripple_carry_adder`
- `and_64bit` / `or_64bit` / `xor_64bit` / `not_64bit` (and 32-bit variants)
- `apply_shift` — bit-list slicing (barrel shifter model)
- `mul_64` / `umulh_64` / `smulh_64` — shift-and-add, 64 iterations
- `udiv_64` / `sdiv_64` — restoring long division, 64 iterations

Python arithmetic is **only** used for host bookkeeping:
- Memory addresses (EA = base + imm12 * scale)
- Memory array indexing
- Loop control (for i in range(N))
- PC advancement / branch targets

## Architecture

```
AArch64GateLevelSimulator
  ├── RegisterFile          — 32×64-bit GPRs + SP as bit lists
  ├── decode()              — combinational decoder (pure function)
  ├── alu.py                — add64/sub64/and64/or64/xor64/not64/...
  └── bits.py               — int↔bit-list bridge, shift/rotate, multiply, divide
```

### Memory layout
64 KiB flat, big-endian.  Programs load at address 0x0000.

### Register file
- X0–X30: general-purpose 64-bit registers, stored as `list[int]` of 64 bits
- X31 / XZR: hardwired zero — reads always return 0, writes are discarded
- SP: separate 64-bit stack pointer register
- NZCV: 4-bit flag nibble (N=bit3, Z=bit2, C=bit1, V=bit0)

## Usage

```python
import struct
from aarch64_gatelevel import AArch64GateLevelSimulator

sim = AArch64GateLevelSimulator()

# MOVZ X0, #42 (64-bit, hw=0)
# Encoding: sf=1, opc=0b10, hw=0, imm16=42, Rd=0
v = (1 << 31) | (0b10 << 29) | (0b100101 << 23) | (0 << 21) | (42 << 5) | 0
prog = struct.pack(">II", v, 0)  # instruction + HALT (0 = halt)

result = sim.execute(prog)
print(result.final_state.gpr[0])  # → 42
```

## Running tests

```bash
uv venv
uv pip install -e ../logic-gates -e ../arithmetic -e ../simulator-protocol \
               -e ../aarch64-simulator -e ".[dev]"
.venv/bin/python -m pytest tests/ -v --cov=aarch64_gatelevel
```

## Test coverage

Target: ≥80% (configured in `pyproject.toml`).

Test modules:
- `test_bits.py` — bit-list conversion, arithmetic, shifts, multiply, divide
- `test_alu.py` — ALU operations with NZCV flag correctness
- `test_register_file.py` — XZR, W-register zero-extension, SP
- `test_decoder.py` — all instruction encoding classes
- `test_programs.py` — complete multi-instruction programs
- `test_equivalence.py` — cross-validation against behavioral simulator

## Package layout

```
src/aarch64_gatelevel/
  __init__.py        — public API
  bits.py            — bit-list helpers and gate-level arithmetic
  alu.py             — ALU operations (ALUResult64, add64, sub64, ...)
  register_file.py   — RegisterFile class
  decoder.py         — combinational instruction decoder
  simulator.py       — AArch64GateLevelSimulator
```

## Dependencies

- `coding-adventures-logic-gates` — AND, OR, XOR, NOT gate primitives
- `coding-adventures-arithmetic` — ripple_carry_adder
- `coding-adventures-simulator-protocol` — Simulator[State], StepTrace, ExecutionResult
- `coding-adventures-aarch64-simulator` — AArch64State, shared state dataclass

## Changelog

See [CHANGELOG.md](CHANGELOG.md).
