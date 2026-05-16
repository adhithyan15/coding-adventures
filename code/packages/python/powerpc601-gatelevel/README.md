# powerpc601-gatelevel

**Layer 07u2** — PowerPC 601 (1992) gate-level simulator.

Every 32-bit ALU operation routes through `AND`, `OR`, `XOR`, `NOT` gate
primitives (from `logic_gates`) and `ripple_carry_adder` (from `arithmetic`).
No Python arithmetic operators appear in the data-path execution paths.

## Architecture

The PowerPC 601 was the first chip produced by the AIM alliance (Apple, IBM,
Motorola) in 1992.  It powered the original Power Macintosh line (1994) and
introduced RISC principles — large register file, fixed-width instructions,
load-store architecture — to the consumer desktop market.

**Register set:**
- GPR0–GPR31: 32 × 32-bit general-purpose registers
- LR: Link Register (stores return address after bl)
- CTR: Count Register (decremented by bdnz branches; indirect branch target)
- XER: Fixed-Point Exception Register (SO/OV/CA flags)
- CR: Condition Register (8 × 4-bit fields CR0–CR7)
- CIA: Current Instruction Address (the program counter)

**Memory:** 64 KiB flat, big-endian byte-addressed.

**Instruction width:** Fixed 32-bit (4 bytes).

## Gate-level constraint

All operations on register values route through gate primitives:

```python
from logic_gates import AND, OR, XOR, NOT
from arithmetic import ripple_carry_adder
```

Examples:
- `ADD` → `ripple_carry_adder(a_bits, b_bits, 0)`
- `SUB` → `ripple_carry_adder(a_bits, invert(b_bits), 1)`
- `AND` → 32 individual `AND(a[i], b[i])` calls
- `MULLW` → 32-iteration shift-and-add via `add_32bit`
- `DIVWU` → 32-iteration long division via `sub32`

## Module structure

| File | Purpose |
|------|---------|
| `bits.py` | 32-bit int ↔ bit-list conversion, shifts, rotate, zero/parity |
| `alu.py` | Gate-level ALU: add, sub, and, or, xor, shifts, mul, div |
| `register_file.py` | GPRs, LR, CTR, XER, CR, CIA as 32-bit bit lists |
| `decoder.py` | Combinational instruction decode (pure function) |
| `simulator.py` | Top-level simulator implementing `Simulator[PowerPC601State]` |

## Usage

```python
import struct
from powerpc601_gatelevel import PowerPC601GateLevelSimulator

sim = PowerPC601GateLevelSimulator()

# Encode: ADDI r3, 0, 42; HALT
prog = struct.pack(">II",
    (14 << 26) | (3 << 21) | (0 << 16) | 42,  # addi r3, 0, 42
    0x00000000,                                  # halt
)
result = sim.execute(prog)
print(result.final_state.gpr[3])  # 42
```

## Instruction coverage

- **Arithmetic:** ADD, ADDI, ADDIS, ADDC, ADDE, ADDME, ADDZE, SUBF, SUBFC,
  SUBFE, SUBFME, SUBFZE, SUBFIC, ADDIC, NEG, MULLW, MULHW, MULHWU, DIVW, DIVWU
- **Logic:** AND, OR, XOR, NAND, NOR, EQV, ANDC, ORC + immediate variants
- **Shifts:** SLW, SRW, SRAW, SRAWI + CNTLZW
- **Rotate/mask:** RLWIMI, RLWINM, RLWNM
- **Compare:** CMP, CMPL, CMPI, CMPLI
- **Branch:** B, BC, BL, BLR (BCLR), BCTR (BCCTR) with full BO/BI decoding
- **CR ops:** CRAND, CRNAND, CROR, CRNOR, CRXOR, CREQV, CRANDC, CRORC, MCRF
- **Loads:** LWZ, LWZU, LBZ, LBZU, LHZ, LHZU, LHA, LHAU, LMW + indexed variants
- **Stores:** STW, STWU, STB, STBU, STH, STHU, STMW + indexed variants
- **SPR:** MFSPR, MTSPR (LR=8, CTR=9, XER=1), MFCR, MTCRF
- **Sync:** ISYNC (no-op), LWARX/STWCX. (simplified)

## Cross-validation

`test_equivalence.py` cross-validates against the behavioral
`PowerPC601Simulator` from `powerpc601-simulator`.

## Installation

```bash
pip install coding-adventures-powerpc601-gatelevel
```

## Development

```bash
cd powerpc601-gatelevel
uv venv
uv pip install -e ../logic-gates -e ../arithmetic -e ../simulator-protocol \
               -e ../powerpc601-simulator -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
