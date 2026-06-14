# CDC 6600 Simulator — Layer 07t

Behavioral simulator for the **CDC 6600 (1964)**, the world's first supercomputer,
designed by Seymour Cray at Control Data Corporation.

## Architecture

| Feature | Value |
|---------|-------|
| Word width | 60 bits |
| Registers | X0–X7 (60-bit), A0–A7 (18-bit), B0–B7 (18-bit) |
| B0 | Hardwired to 0 |
| Memory | 4096 × 60-bit words |
| Instruction sizes | 15-bit (short) or 30-bit (long) |
| Packing | 4 parcels per 60-bit word |
| HALT | All-zeros 15-bit parcel |

## Usage

```python
from cdc6600_simulator import CDC6600Simulator, HALT, long_instr, short_instr
from cdc6600_simulator.simulator import F_LDXI, F_IXXP, F_JNE, F_LDBI, F_TXB, F_IBBM

sim = CDC6600Simulator()

# Sum 1+2+...+10 = 55
prog = (
    long_instr(F_LDXI, 1, 0, 0)   +   # X1 = 0
    long_instr(F_LDBI, 1, 0, 10)  +   # B1 = 10
    long_instr(F_LDBI, 2, 0, 1)   +   # B2 = 1
    short_instr(F_TXB, 3, 1, 0)   +   # X3 = B1
    short_instr(F_IXXP, 1, 1, 3)  +   # X1 += X3
    short_instr(F_IBBM, 1, 1, 2)  +   # B1 -= 1
    long_instr(F_JNE, 0, 1, 6)    +   # if B1!=0: goto P=6 (loop top)
    HALT
)

result = sim.execute(prog)
print(result.final_state.x1)   # 55
```

## Instruction Encoding

### Short (15-bit) — `short_instr(f, i, j, k)`
```
[14:9] f  opcode (6 bits)
[ 8:6] i  destination register
[ 5:3] j  left source
[ 2:0] k  right source
```

### Long (30-bit) — `long_instr(f, i, j, K)`
```
First parcel  [14:9] f, [8:6] i, [5:3] j, [2:0] K[17:15]
Second parcel [14:0] K[14:0]
```

## SIM00 Protocol

Implements `Simulator[CDC6600State]`:

| Method | Description |
|--------|-------------|
| `reset()` | Zero all state |
| `load(program)` | Reset + pack bytes into 60-bit words |
| `step()` → `StepTrace` | Execute one instruction |
| `execute(program, max_steps)` → `ExecutionResult` | Run to HALT |
| `get_state()` → `CDC6600State` | Frozen snapshot |

## Series Position

This is Layer 07t in the CPU simulator series:

```
07a RISC-V → 07b ARM → 07c Wasm → 07d Intel 4004 → 07e ARM1 → 07f Intel 8008
→ 07g GE-225 → 07h IBM 704 → 07i Intel 8080 → 07j MOS 6502 → 07k Z80
→ 07l Manchester Baby → 07m Intel 8086 → 07n Motorola 68000 → 07o PDP-11
→ 07p Intel 8051 → 07q MIPS R2000 → 07r SPARC V8 → 07s DEC Alpha AXP
→ [07t CDC 6600]  ← YOU ARE HERE
```
