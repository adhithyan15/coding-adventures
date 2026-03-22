# Intel 4004 Simulator

**Layer 4d of the computing stack** — simulates the Intel 4004 (1971), the world's first commercial microprocessor. 4-bit, accumulator architecture.

## What this package does

Simulates the complete Intel 4004 instruction set — all 46 real instructions plus a simulator-only HLT. The 4004 was designed by Federico Faggin, Ted Hoff, and Stanley Mazor for the Busicom 141-PF calculator, and proved that a general-purpose processor could be built on a single chip.

**Architecture:**
- 4-bit data bus, 8-bit instructions (some 2-byte)
- 16 × 4-bit registers (R0–R15), organized as 8 pairs
- 4-bit accumulator + carry flag
- 12-bit program counter → 4096 bytes of ROM
- 3-level hardware call stack (no software stack)
- 4 RAM banks × 4 registers × (16 main + 4 status) nibbles
- ROM I/O port for peripheral communication

**Execution engine:** Uses `GenericVM` from the `virtual-machine` package. Each 4004 opcode is registered as a handler. GenericVM provides the fetch-decode-execute loop, PC management, step/run, and tracing.

## Where it fits

```
Logic Gates → Arithmetic → CPU → [Intel 4004 Simulator] → Assembler → Compiler → VM
```

## Installation

```bash
uv add coding-adventures-intel4004-simulator
```

## Usage

```python
from intel4004_simulator import Intel4004Simulator

sim = Intel4004Simulator()

# x = 1 + 2 program:
#   LDM 1    → A = 1
#   XCH R0   → R0 = 1, A = 0
#   LDM 2    → A = 2
#   ADD R0   → A = 2 + 1 = 3
#   XCH R1   → R1 = 3
#   HLT      → stop
traces = sim.run(bytes([0xD1, 0xB0, 0xD2, 0x80, 0xB1, 0x01]))

for t in traces:
    print(f"{t.address:03X}: {t.mnemonic:<10} A={t.accumulator_after}")

assert sim.registers[1] == 3  # R1 = 1 + 2 = 3
```

### Step-by-step execution

```python
sim = Intel4004Simulator()
sim.load_program(bytes([0xD5, 0xF2, 0x01]))  # LDM 5, IAC, HLT
sim._prepare_execution()

trace1 = sim.step()  # LDM 5 → A=5
trace2 = sim.step()  # IAC   → A=6
trace3 = sim.step()  # HLT   → halted
```

### Multiply 3 × 4 via repeated addition

```python
sim = Intel4004Simulator()
sim.run(bytes([
    0xD3, 0xB0,   # LDM 3, XCH R0     R0=3 (multiplicand)
    0xDC, 0xB1,   # LDM 12, XCH R1    R1=12 (=-4 in 4-bit)
    0xD0,         # LDM 0              A=0 (running sum)
    0x80,         # ADD R0             A += R0
    0x71, 0x05,   # ISZ R1, 0x05       R1++; loop if R1≠0
    0xB2,         # XCH R2             store result
    0x01,         # HLT
]))
assert sim.registers[2] == 12  # 3 × 4 = 12
```

## Supported instructions

| Range | Instruction | Description |
|-------|------------|-------------|
| 0x00 | NOP | No operation |
| 0x01 | HLT | Halt (simulator-only) |
| 0x1_ | JCN c,a | Conditional jump |
| 0x2_ even | FIM Pp,d | Fetch immediate to pair |
| 0x2_ odd | SRC Pp | Send register control |
| 0x3_ even | FIN Pp | Fetch indirect from ROM |
| 0x3_ odd | JIN Pp | Jump indirect |
| 0x4_ | JUN a | Unconditional jump |
| 0x5_ | JMS a | Jump to subroutine |
| 0x6_ | INC Rn | Increment register |
| 0x7_ | ISZ Rn,a | Increment and skip if zero |
| 0x8_ | ADD Rn | Add register to accumulator |
| 0x9_ | SUB Rn | Subtract register from accumulator |
| 0xA_ | LD Rn | Load register to accumulator |
| 0xB_ | XCH Rn | Exchange accumulator and register |
| 0xC_ | BBL n | Branch back and load |
| 0xD_ | LDM n | Load immediate |
| 0xE0–EF | I/O | RAM/ROM read/write operations |
| 0xF0–FD | Accum | Accumulator manipulation (CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL) |

## Spec

See [07d-intel4004-simulator.md](../../../specs/07d-intel4004-simulator.md) for the full specification.
