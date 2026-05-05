# sparc-v8-simulator

A behavioral simulator for the **SPARC V8** (Scalable Processor ARChitecture,
Version 8, 1987) by Sun Microsystems — Layer 07r in the historical CPU series.

## What is SPARC V8?

SPARC V8 was Sun Microsystems' answer to the RISC revolution.  Published as an
open, licensable standard in 1987, it powered Sun's SPARCstation workstations
and became one of the most commercially successful RISC architectures of the
late 1980s and 1990s.

SPARC's defining innovation is **register windows**: instead of a single flat
register file, the CPU has a rotating bank of overlapping windows.  Each
`SAVE` instruction creates a new "stack frame" in hardware by rotating a
Current Window Pointer (CWP) — no memory writes needed to save caller
registers.  The caller's `%o` (out) registers automatically become the
callee's `%i` (in) registers through the overlap.

## Architecture at a Glance

| Feature | Detail |
|---------|--------|
| ISA width | 32-bit fixed-length |
| Endianness | Big-endian |
| Registers | 56 physical: 8 globals + 3 × 16 windowed |
| Windows | 3 (this implementation); standard chips had 8–32 |
| Condition codes | N, Z, V, C in the Processor State Register (PSR) |
| Multiplier reg | Y — holds the high 32 bits of MUL results |
| HALT convention | `ta 0` (trap always, vector 0) — word `0x91D02000` |

### Register Windows

Virtual registers `%r0–%r31` are visible to running code; physical registers
0–55 are the hardware backing store.

```
Virtual   Name     Role                      Physical (CWP=0)
───────────────────────────────────────────────────────────
%r0       %g0      Global (always 0)         0
%r1–%r7   %g1–%g7  Globals (caller-saved)    1–7
%r8–%r15  %o0–%o7  Outs / callee args        8–15   (window 0 base = 8)
%r16–%r23 %l0–%l7  Locals (private)          16–23
%r24–%r31 %i0–%i7  Ins / return values       24–31  (window 1 base = 8)
```

`SAVE` decrements CWP (mod 3); `RESTORE` increments it.  The callee's `%i`
registers physically *are* the caller's `%o` registers — no copying occurs.

## Instruction Set (subset implemented)

| Group | Instructions |
|-------|-------------|
| Data move | `SETHI`, `NOP` |
| Arithmetic | `ADD`, `ADDcc`, `ADDX`, `ADDXcc`, `SUB`, `SUBcc`, `SUBX`, `SUBXcc` |
| Logic | `AND`, `ANDcc`, `ANDN`, `ANDNcc`, `OR`, `ORcc`, `ORN`, `ORNcc`, `XOR`, `XORcc`, `XNOR`, `XNORcc` |
| Shift | `SLL`, `SRL`, `SRA` |
| Multiply | `UMUL`, `UMULcc`, `SMUL`, `SMULcc`, `MULScc` |
| Divide | `UDIV`, `UDIVcc`, `SDIV`, `SDIVcc` |
| Mul reg | `RDY`, `WRY` |
| Branch | `Bicc` (BA, BN, BE, BNE, BG, BLE, BGE, BL, BGU, BLEU, BCC, BCS, BPOS, BNEG, BVC, BVS) |
| Call | `CALL` |
| Jump | `JMPL` |
| Window | `SAVE`, `RESTORE` |
| Load | `LD`, `LDUB`, `LDUH`, `LDSB`, `LDSH` |
| Store | `ST`, `STB`, `STH` |
| Trap | `Ticc` (`ta 0` = HALT) |

## SIM00 Protocol

`SPARCSimulator` implements the `Simulator[SPARCState]` protocol from
`coding-adventures-simulator-protocol`:

```python
from sparc_v8_simulator import SPARCSimulator, SPARCState

sim = SPARCSimulator()

# Single-call convenience: load, run to HALT, return ExecutionResult
result = sim.execute(program_bytes)

print(f"Steps: {result.steps}")
print(f"Final PC: {result.final_state.pc:#010x}")
for t in result.traces:
    print(f"  {t.pc_before:#010x}  {t.mnemonic}")
```

### Step-by-step execution

```python
sim = SPARCSimulator()
sim.load(program_bytes)
while not sim._halted:
    trace = sim.step()
    print(trace)
state = sim.get_state()   # frozen SPARCState snapshot
```

### SPARCState fields

```python
@dataclasses.dataclass(frozen=True)
class SPARCState:
    pc:     int               # program counter
    npc:    int               # next-PC (branch target staging)
    regs:   tuple[int, ...]   # 56 physical registers
    cwp:    int               # current window pointer (0–NWINDOWS-1)
    psr_n:  bool              # negative flag
    psr_z:  bool              # zero flag
    psr_v:  bool              # overflow flag
    psr_c:  bool              # carry flag
    y:      int               # multiply/divide Y register
    memory: tuple[int, ...]   # 65 536-byte address space
    halted: bool
```

Convenience properties: `.g0–.g7`, `.o0–.o7`, `.l0–.l7`, `.i0–.i7`, `.sp`, `.fp`, `.o7`.

## Usage Example — Sum 1 to 10

```python
import struct
from sparc_v8_simulator import SPARCSimulator

def w(v): return struct.pack(">I", v & 0xFFFF_FFFF)

# %g1 = 0, %g2 = 0, %g3 = 10
# loop: %g1 += 1;  %g2 += %g1;  BNE loop
# result in %g2

SETHI = lambda rd, v: w((0 << 30) | (rd << 25) | (4 << 22) | (v & 0x3FFFFF))
ADD_I = lambda rd, rs1, imm: w((2 << 30) | (rd << 25) | (0x00 << 19) | (rs1 << 14) | (1 << 13) | (imm & 0x1FFF))
ADDcc_I = lambda rd, rs1, imm: w((2 << 30) | (rd << 25) | (0x10 << 19) | (rs1 << 14) | (1 << 13) | (imm & 0x1FFF))
ADD_R = lambda rd, rs1, rs2: w((2 << 30) | (rd << 25) | (0x00 << 19) | (rs1 << 14) | rs2)
BNE  = lambda disp: w((0 << 30) | (9 << 25) | (2 << 22) | (disp & 0x3FFFFF))
HALT = w(0x91D0_2000)

prog = (
    SETHI(1, 0)          # %g1 = 0
    + SETHI(2, 0)        # %g2 = 0
    + ADD_I(3, 0, 10)    # %g3 = 10  (loop limit)
    + ADDcc_I(1, 1, 1)   # %g1 += 1; set Z,N
    + ADD_R(2, 2, 1)     # %g2 += %g1
    + ADDcc_I(3, 3, -1)  # %g3 -= 1 (via ADD -1); set Z
    + BNE(-3)            # branch back 3 words if %g3 != 0
    + HALT
)

result = SPARCSimulator().execute(prog)
state  = result.final_state
print(state.regs[2])   # 55  (= 1+2+...+10)
```

## Installation

```bash
pip install coding-adventures-sparc-v8-simulator
```

Or from source (requires `coding-adventures-simulator-protocol` on `PYTHONPATH`):

```bash
cd code/packages/python/sparc-v8-simulator
pip install -e .
```

## Testing

```bash
pytest tests/ -v --cov=sparc_v8_simulator
```

## How it fits in the Layer Stack

| Layer | Package | Processor | Year |
|-------|---------|-----------|------|
| 07k | `pdp11-simulator` | PDP-11 | 1970 |
| 07m | `m68k-simulator` | Motorola 68000 | 1979 |
| 07n | `z80-simulator` | Zilog Z-80 | 1976 |
| 07p | `6502-simulator` | MOS 6502 | 1975 |
| 07q | `mips-r2000-simulator` | MIPS R2000 | 1985 |
| **07r** | **`sparc-v8-simulator`** | **SPARC V8** | **1987** |

Each simulator shares the `Simulator[S]` protocol from `simulator-protocol`.

## Design Notes & Simplifications

* **No delay slots** — SPARC V8 normally has a branch delay slot; this
  simulator omits it for clarity.  Branches take effect immediately.
* **3 register windows** — Standard SPARC hardware had 8–32; 3 is the
  architectural minimum and is sufficient to demonstrate window overflow.
* **64 KiB flat memory** — Simplified address space; no MMU, no traps for
  unmapped pages.
* **Alignment enforced** — Word loads/stores (`LD`/`ST`) require 4-byte
  alignment; halfword ops require 2-byte alignment; byte ops are always OK.
* **Ticc HALT** — Only `ta 0` (trap-always, vector 0) is handled; all other
  trap conditions raise `ValueError`.
* **No FPU / CP** — Floating-point and coprocessor instructions are omitted.
* **No interrupts or privileged instructions** — Supervisor mode features
  are not modelled.
