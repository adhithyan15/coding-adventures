# x86-64 Simulator (Layer 07w)

Behavioral simulator for the x86-64 (AMD64) instruction set architecture.
Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
CPU simulator suite.

## What it is

x86-64 (also called AMD64 or Intel 64) is the dominant 64-bit server and
desktop ISA.  AMD introduced it in 2003 as a 64-bit extension of the 32-bit
x86 architecture; Intel adopted it as EM64T in 2004.  As of 2026 it is the
target ISA for GCC, Clang, and MSVC on Linux, Windows, and macOS.

This simulator implements the **integer ISA in 64-bit long mode** only:

- 16 × 64-bit GPRs with REX-extended register encoding (R8–R15)
- Full ModRM + SIB + displacement addressing
- All 16 Jcc / CMOVcc / SETcc condition codes
- RFLAGS: CF PF ZF SF OF
- 64 KiB flat little-endian memory

## Quick start

```python
from x86_64_simulator import X86_64Simulator

sim = X86_64Simulator()

# MOV RAX, 42 (REX.W B8 imm64)
# HLT
prog = bytes([
    0x48, 0xB8, 42, 0, 0, 0, 0, 0, 0, 0,  # MOV RAX, 42
    0xF4,                                   # HLT
])
state = sim.execute(prog)
print(state.rax)   # → 42
```

## Architecture

### Registers

| Name | Index | Width | Role |
|------|-------|-------|------|
| RAX  |  0    | 64    | Accumulator / return value |
| RCX  |  1    | 64    | Counter (LOOP, REP) |
| RDX  |  2    | 64    | Data / IDIV high-half |
| RBX  |  3    | 64    | Base (callee-saved) |
| RSP  |  4    | 64    | Stack pointer |
| RBP  |  5    | 64    | Frame pointer (callee-saved) |
| RSI  |  6    | 64    | Source index |
| RDI  |  7    | 64    | Destination index |
| R8–R15 | 8–15 | 64 | Extra GPRs (need REX prefix) |

### Instruction encoding summary

```
[legacy prefix] [REX 40–4F] opcode [0F ext] [ModRM] [SIB] [disp8/32] [imm8/32/64]
```

REX.W=1 selects 64-bit operand size.  REX.R/X/B extend the register fields
in ModRM and SIB by one bit, giving access to R8–R15.

### RFLAGS

| Bit | Flag | Meaning |
|-----|------|---------|
|  0  | CF   | Carry (unsigned overflow/borrow) |
|  2  | PF   | Parity (low-byte even popcount) |
|  6  | ZF   | Zero |
|  7  | SF   | Sign (result MSB) |
| 11  | OF   | Overflow (signed) |

## Simplifications

1. 64-bit long mode only (no real/protected/compatibility mode)
2. No x87 FPU, no SSE/AVX/MMX
3. No privilege levels (ring 0–3)
4. No segmentation (FS/GS for TLS ignored)
5. Flat 64 KiB memory wrapping modulo 65 536
6. No paging / virtual memory
7. SYSCALL/INT treated as NOP
8. String ops: REP STOSQ/STOSD only (no MOVS, CMPS, SCAS)
9. AF (auxiliary carry) flag not tracked

## Layer lineage

```
Layer 07a  RISC-V (RV32I)
  …
Layer 07v  AArch64 (ARMv8-A, 2011)          ← previous
Layer 07w  x86-64 / AMD64 (2003)            ← this package
Layer 07x  ARMv7-A / Thumb-2 (2004)         ← next
```
