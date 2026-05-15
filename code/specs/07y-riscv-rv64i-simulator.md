# 07y — RISC-V RV64I Behavioral Simulator

## Overview

Layer 07y implements a behavioral simulator for the **RISC-V RV64I** base
integer instruction set, plus the standard **M extension** (multiply/divide).
RISC-V is an open, royalty-free ISA designed at UC Berkeley for simplicity and
regularity.  RV64I is the 64-bit base that underpins all modern RISC-V
deployments.

This is Layer 07y in the coding-adventures simulator series.  The earlier
`riscv-simulator` (Layer 07a) covered a minimal RV32I subset; this package
covers the full 64-bit ISA with clean SIM00 protocol integration.

---

## Architecture Summary

| Property | Value |
|----------|-------|
| Word width | 64-bit |
| Registers | 32 × 64-bit integer (x0–x31); x0 hardwired to 0 |
| PC | 64-bit, always 4-byte aligned |
| Instruction width | Fixed 32 bits |
| Byte order | Little-endian |
| Extensions covered | RV64I + M (multiply/divide) |

---

## Register File

| Reg | ABI name | Role |
|-----|----------|------|
| x0  | zero     | Hardwired 0 — reads always return 0; writes ignored |
| x1  | ra       | Return address |
| x2  | sp       | Stack pointer |
| x3  | gp       | Global pointer |
| x4  | tp       | Thread pointer |
| x5–x7  | t0–t2 | Temporaries |
| x8  | s0/fp    | Saved / frame pointer |
| x9  | s1       | Saved |
| x10–x11 | a0–a1 | Function args / return values |
| x12–x17 | a2–a7 | Function args |
| x18–x27 | s2–s11 | Saved |
| x28–x31 | t3–t6 | Temporaries |

---

## Instruction Encoding Formats

RISC-V uses six instruction formats.  The opcode always sits in bits [6:0]
(with the two LSBs always `11` for 32-bit instructions).

```
R-type  [31:25 funct7][24:20 rs2][19:15 rs1][14:12 funct3][11:7 rd][6:0 opcode]
I-type  [31:20 imm[11:0]][19:15 rs1][14:12 funct3][11:7 rd][6:0 opcode]
S-type  [31:25 imm[11:5]][24:20 rs2][19:15 rs1][14:12 funct3][11:7 imm[4:0]][6:0 opcode]
B-type  [31 imm[12]][30:25 imm[10:5]][24:20 rs2][19:15 rs1][14:12 funct3][11:8 imm[4:1]][7 imm[11]][6:0 opcode]
U-type  [31:12 imm[31:12]][11:7 rd][6:0 opcode]
J-type  [31 imm[20]][30:21 imm[10:1]][20 imm[11]][19:12 imm[19:12]][11:7 rd][6:0 opcode]
```

Immediate reconstruction:
- **I-type**: `imm = sign_extend(bits[31:20], 12)`
- **S-type**: `imm = sign_extend(bits[31:25]:bits[11:7], 12)`
- **B-type**: `imm = sign_extend(bits[31]:bits[7]:bits[30:25]:bits[11:8]:0, 13)`
- **U-type**: `imm = sign_extend(bits[31:12] << 12, 32)`
- **J-type**: `imm = sign_extend(bits[31]:bits[19:12]:bits[20]:bits[30:21]:0, 21)`

---

## Base Integer ISA (RV64I)

### Opcode table

| Opcode (bits[6:0]) | Mnemonic group   | Format |
|--------------------|-----------------|--------|
| 0110111 (0x37)     | LUI             | U      |
| 0010111 (0x17)     | AUIPC           | U      |
| 1101111 (0x6F)     | JAL             | J      |
| 1100111 (0x67)     | JALR            | I      |
| 1100011 (0x63)     | Branches        | B      |
| 0000011 (0x03)     | Loads           | I      |
| 0100011 (0x23)     | Stores          | S      |
| 0010011 (0x13)     | ALU immediate   | I      |
| 0110011 (0x33)     | ALU register    | R      |
| 0011011 (0x1B)     | ALU imm word    | I (RV64) |
| 0111011 (0x3B)     | ALU reg word    | R (RV64) |
| 0001111 (0x0F)     | FENCE           | I      |
| 1110011 (0x73)     | SYSTEM          | I      |

### LUI / AUIPC

```
LUI   rd, imm20    rd = sign_extend(imm20 << 12, 64)
AUIPC rd, imm20    rd = PC + sign_extend(imm20 << 12, 64)
```

### JAL / JALR

```
JAL   rd, offset   rd = PC+4;  PC = PC + sign_extend(offset, 21)
JALR  rd, rs1, imm rd = PC+4;  PC = (rs1 + sign_extend(imm,12)) & ~1
```

### Branches (B-type, funct3)

| funct3 | Mnemonic | Condition |
|--------|----------|-----------|
| 000    | BEQ      | rs1 == rs2 |
| 001    | BNE      | rs1 != rs2 |
| 100    | BLT      | rs1 < rs2  (signed) |
| 101    | BGE      | rs1 >= rs2 (signed) |
| 110    | BLTU     | rs1 < rs2  (unsigned) |
| 111    | BGEU     | rs1 >= rs2 (unsigned) |

Branch target = PC + sign_extend(imm, 13).

### Loads (I-type, funct3)

| funct3 | Mnemonic | Width    | Sign |
|--------|----------|----------|------|
| 000    | LB       | 8-bit    | signed  |
| 001    | LH       | 16-bit   | signed  |
| 010    | LW       | 32-bit   | signed  |
| 011    | LD       | 64-bit   | unsigned|
| 100    | LBU      | 8-bit    | unsigned|
| 101    | LHU      | 16-bit   | unsigned|
| 110    | LWU      | 32-bit   | unsigned|

Address = rs1 + sign_extend(imm, 12).

### Stores (S-type, funct3)

| funct3 | Mnemonic | Width  |
|--------|----------|--------|
| 000    | SB       | 8-bit  |
| 001    | SH       | 16-bit |
| 010    | SW       | 32-bit |
| 011    | SD       | 64-bit |

Address = rs1 + sign_extend(imm, 12).

### ALU Immediate (I-type, opcode 0x13)

| funct3 | Mnemonic | Operation |
|--------|----------|-----------|
| 000    | ADDI     | rd = rs1 + sext(imm,12) |
| 010    | SLTI     | rd = (rs1 < sext(imm,12)) ? 1 : 0  (signed) |
| 011    | SLTIU    | rd = ((u64)rs1 < (u64)sext(imm,12)) ? 1 : 0 |
| 100    | XORI     | rd = rs1 ^ sext(imm,12) |
| 110    | ORI      | rd = rs1 | sext(imm,12) |
| 111    | ANDI     | rd = rs1 & sext(imm,12) |
| 001    | SLLI     | rd = rs1 << shamt6 |
| 101    | SRLI/SRAI| funct7[5]=0: SRLI logical; =1: SRAI arithmetic |

For 64-bit shifts, `shamt` is bits [25:20] (6-bit shift amount).

### ALU Register (R-type, opcode 0x33)

| funct7  | funct3 | Mnemonic | Operation |
|---------|--------|----------|-----------|
| 0000000 | 000    | ADD      | rd = rs1 + rs2 |
| 0100000 | 000    | SUB      | rd = rs1 - rs2 |
| 0000000 | 001    | SLL      | rd = rs1 << (rs2 & 63) |
| 0000000 | 010    | SLT      | rd = (rs1 < rs2) signed |
| 0000000 | 011    | SLTU     | rd = (rs1 < rs2) unsigned |
| 0000000 | 100    | XOR      | rd = rs1 ^ rs2 |
| 0000000 | 101    | SRL      | rd = rs1 >> (rs2 & 63) logical |
| 0100000 | 101    | SRA      | rd = rs1 >> (rs2 & 63) arithmetic |
| 0000000 | 110    | OR       | rd = rs1 \| rs2 |
| 0000000 | 111    | AND      | rd = rs1 & rs2 |

### RV64I word ops — ALU Immediate Word (opcode 0x1B)

Operate on the lower 32 bits of rs1; result sign-extended to 64 bits.

| funct3 | Mnemonic | Operation |
|--------|----------|-----------|
| 000    | ADDIW    | rd = sext32(rs1[31:0] + sext(imm,12)) |
| 001    | SLLIW    | rd = sext32(rs1[31:0] << shamt5) |
| 101    | SRLIW/SRAIW | logical or arithmetic 32-bit shift, sext to 64 |

### RV64I word ops — ALU Register Word (opcode 0x3B)

| funct7  | funct3 | Mnemonic | Operation |
|---------|--------|----------|-----------|
| 0000000 | 000    | ADDW     | rd = sext32(rs1[31:0] + rs2[31:0]) |
| 0100000 | 000    | SUBW     | rd = sext32(rs1[31:0] - rs2[31:0]) |
| 0000000 | 001    | SLLW     | rd = sext32(rs1[31:0] << (rs2 & 31)) |
| 0000000 | 101    | SRLW     | rd = sext32(rs1[31:0] >> (rs2 & 31)) logical |
| 0100000 | 101    | SRAW     | rd = sext32(rs1[31:0] >> (rs2 & 31)) arithmetic |

---

## M Extension (Multiply/Divide)

### 64-bit multiply/divide (opcode 0x33, funct7=0000001)

| funct3 | Mnemonic | Operation |
|--------|----------|-----------|
| 000    | MUL      | rd = (rs1 * rs2)[63:0] (lower 64 bits) |
| 001    | MULH     | rd = (signed(rs1) * signed(rs2))[127:64] |
| 010    | MULHSU   | rd = (signed(rs1) * unsigned(rs2))[127:64] |
| 011    | MULHU    | rd = (unsigned(rs1) * unsigned(rs2))[127:64] |
| 100    | DIV      | rd = signed(rs1) / signed(rs2); truncated toward zero |
| 101    | DIVU     | rd = unsigned(rs1) / unsigned(rs2) |
| 110    | REM      | rd = signed(rs1) % signed(rs2); sign matches dividend |
| 111    | REMU     | rd = unsigned(rs1) % unsigned(rs2) |

Division by zero: DIV→-1, DIVU→MAXUINT, REM/REMU→dividend.
Overflow (INT64_MIN / -1): DIV→INT64_MIN, REM→0.

### 32-bit multiply/divide word (opcode 0x3B, funct7=0000001)

| funct3 | Mnemonic | Operation |
|--------|----------|-----------|
| 000    | MULW     | rd = sext32(rs1[31:0] * rs2[31:0]) |
| 100    | DIVW     | rd = sext32(signed32(rs1) / signed32(rs2)) |
| 101    | DIVUW    | rd = sext32(unsigned32(rs1) / unsigned32(rs2)) |
| 110    | REMW     | rd = sext32(signed32(rs1) % signed32(rs2)) |
| 111    | REMUW    | rd = sext32(unsigned32(rs1) % unsigned32(rs2)) |

---

## SYSTEM Instructions (opcode 0x73)

| imm[11:0] | Mnemonic | Effect |
|-----------|----------|--------|
| 000000000000 | ECALL  | Halt (treated as halt sentinel) |
| 000000000001 | EBREAK | Halt |

All other CSR instructions are NOP in this simulator.

---

## Halt Sentinel

A 32-bit zero word (`0x00000000`) fetched at PC causes an immediate halt.
This is the `ECALL` encoding with all fields zeroed, which is architecturally
invalid and unambiguously identifies the halt condition.

---

## Memory Model

| Property | Value |
|----------|-------|
| Size | 65 536 bytes (64 KiB) |
| Layout | Little-endian |
| Wrapping | All addresses masked with `0xFFFF` |

---

## Reset State

| Resource | Reset value |
|----------|-------------|
| x0 | 0 (always) |
| x2 (sp) | 0xFFF8 |
| x1, x3–x31 | 0 |
| PC | 0 |
| Memory | All 0 |
| Halted | False |

---

## SIM00 Protocol

The simulator implements the standard SIM00 `Simulator[State]` protocol:

| Method | Behaviour |
|--------|-----------|
| `reset()` | Zero all registers and memory; set SP=0xFFF8; PC=0 |
| `load(program: bytes)` | Reset then copy program bytes to memory[0..] |
| `step()` | Execute one instruction; return `StepTrace` |
| `execute(program, max_steps=100_000)` | Load and run until halt or limit |
| `get_state()` | Return frozen `RV64IState` snapshot |
| `set_input_port(port, value)` | Stub (no I/O model) |
| `get_output_port(port)` | Stub → returns 0 |
| `interrupt(vector)` | Stub |
| `nmi()` | Stub |

---

## StepTrace

`StepTrace` is a locally defined frozen dataclass (not from `simulator_protocol`):

```python
@dataclass(frozen=True)
class StepTrace:
    pc_before: int
    pc_after:  int
    halted:    bool
```

---

## Implementation Scope

This simulator targets **behavioral correctness** for compiler testing:

- Full RV64I base integer instruction set
- M extension (integer multiply/divide, 64-bit and word variants)
- No floating-point (F/D), no atomics (A), no compressed (C)
- No privilege modes, CSRs, or MMU
- FENCE is a NOP
- ECALL/EBREAK halt the simulator

---

## Package Layout

```
code/packages/python/riscv-rv64i-simulator/
├── BUILD
├── CHANGELOG.md
├── README.md
├── pyproject.toml
└── src/
    └── riscv_rv64i_simulator/
        ├── __init__.py
        ├── py.typed
        ├── state.py       ← constants, sext helpers, RV64IState dataclass
        └── simulator.py   ← _CPU, instruction decode/execute, RV64ISimulator
tests/
    ├── conftest.py
    ├── test_protocol.py
    └── test_instructions.py
```
