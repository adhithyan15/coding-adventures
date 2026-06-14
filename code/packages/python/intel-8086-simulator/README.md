# intel-8086-simulator

A pure-Python behavioral simulator for the **Intel 8086** (1978) — Layer 07m
in the coding-adventures simulator stack.

The Intel 8086 is the direct ancestor of every x86 CPU made today. Introduced
in 1978, it powered the IBM PC (via its 8088 variant) and defined the segment-
based memory model, variable-length CISC instructions, and register conventions
(AX/BX/CX/DX) that every modern x86 programmer still uses.

## Position in the stack

```
Layer 07m — Intel 8086 (1978)         ← this package
Layer 07l — Manchester Baby (1948)
Layer 07k — Zilog Z80 (1976)
Layer 07j — MOS 6502 (1975)
...
```

## Architecture highlights

| Feature | Value |
|---------|-------|
| Data width | 16-bit |
| Address space | 1 MB (20-bit physical via CS×16+IP) |
| General registers | AX, BX, CX, DX (each split into high/low byte) |
| Pointer/index | SP, BP, SI, DI |
| Segment registers | CS, DS, SS, ES |
| Flags | CF, PF, AF, ZF, SF, TF, IF, DF, OF |
| Instruction length | 1–6 bytes (variable CISC) |
| Instruction encoding | Opcode + optional ModRM + disp + imm |
| Memory model | Segmented: physical = seg×16 + offset |

## Installation

```bash
pip install coding-adventures-intel-8086-simulator
```

Or from source (requires `coding-adventures-simulator-protocol`):

```bash
pip install -e ../simulator-protocol -e .
```

## Quick start

```python
from intel_8086_simulator import X86Simulator

sim = X86Simulator()

# MOV AX, 42 (B8 2A 00) + HLT (F4)
prog = bytes([0xB8, 42, 0x00,   # MOV AX, 42
              0xF4])             # HLT
result = sim.execute(prog)
assert result.ok
assert result.final_state.ax == 42
```

## Adding two numbers

```python
from intel_8086_simulator import X86Simulator

sim = X86Simulator()
prog = bytes([
    0xB8, 10, 0x00,   # MOV AX, 10
    0xBB, 20, 0x00,   # MOV BX, 20
    0x01, 0xD8,       # ADD AX, BX  (opcode=01, ModRM=D8: mod=11 reg=3(BX) r/m=0(AX))
    0xF4,             # HLT
])
result = sim.execute(prog)
assert result.final_state.ax == 30
```

## Step-by-step debugging

```python
from intel_8086_simulator import X86Simulator

sim = X86Simulator()
sim.reset()
sim.load(prog)
while not sim._halted:
    trace = sim.step()
    print(f"IP={trace.pc_before:04X} {trace.mnemonic}")
```

## Protocol

`X86Simulator` implements the `Simulator[X86State]` protocol from
`coding-adventures-simulator-protocol` (SIM00):

| Method | Description |
|--------|-------------|
| `reset()` | Clear all registers, memory, flags to 0 |
| `load(bytes, origin=0)` | Write bytes to physical memory at `origin` |
| `step()` → `StepTrace` | Execute one instruction |
| `execute(bytes, max_steps=10_000)` → `ExecutionResult` | Run a program |
| `get_state()` → `X86State` | Frozen snapshot of current machine state |

## Instruction set

Implements the full Intel 8086 real-mode ISA:
- **Data transfer**: MOV, XCHG, PUSH/POP, PUSHF/POPF, LEA, LDS/LES,
  LAHF/SAHF, CBW, CWD, XLAT
- **Arithmetic**: ADD, ADC, SUB, SBB, INC, DEC, NEG, CMP, MUL, IMUL,
  DIV, IDIV, DAA, DAS, AAA, AAS, AAM, AAD
- **Logical**: AND, OR, XOR, NOT, TEST
- **Shifts/rotates**: SHL, SHR, SAR, ROL, ROR, RCL, RCR
- **Control**: JMP (short/near/far), CALL/RET, all 16 Jcc, LOOP, JCXZ, INT
- **String**: MOVS, CMPS, SCAS, LODS, STOS + REP/REPE/REPNE
- **Misc**: NOP, HLT, CLC/STC/CMC, CLD/STD, CLI/STI, IN/OUT

## Testing

```bash
pip install -e .[dev]
pytest tests/ -v
```

114 tests, 100% line coverage.
