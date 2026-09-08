# 04b — RISC-V Simulator

## Overview

The RISC-V simulator implements a minimal subset of the RISC-V RV32I base integer instruction set. RISC-V is an open, clean ISA designed by academics (Patterson & Hennessy at UC Berkeley) — it avoids the historical cruft of ARM and x86, making it ideal for learning.

This is an alternative Layer 4 alongside the ARM simulator. Both depend on the CPU simulator (Layer 3).

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

**Input from:** CPU simulator (registers, memory, ALU, fetch-decode-execute cycle).
**Output to:** Assembler (defines the binary encoding the assembler produces).

## Why RISC-V over ARM?

| | RISC-V RV32I | ARMv7 |
|---|---|---|
| Instructions (base) | 47 | 150+ |
| Condition codes | None | On every instruction |
| Encoding regularity | Very regular | Complex |
| Register count | 32 (x0 hardwired to 0) | 16 |
| Open standard | Yes (free) | No (licensed) |
| University adoption | MIT, Berkeley, Stanford | Legacy |

## Concepts

### Registers

RISC-V has 32 registers, each 32 bits wide:

```
x0  = 0 (hardwired, always zero — reads as 0, writes are ignored)
x1  = ra (return address)
x2  = sp (stack pointer)
x3-x31 = general purpose
```

The hardwired zero register (x0) is key — it simplifies many operations:
- Load immediate: `addi x1, x0, 42` (x1 = 0 + 42 = 42)
- Move: `addi x2, x1, 0` (x2 = x1 + 0 = x1)
- No-op: `addi x0, x0, 0`

### Instruction Encoding

All instructions are 32 bits. The opcode is always in bits [6:0]. Register fields are always in the same position:

```
R-type: [funct7 | rs2 | rs1 | funct3 | rd | opcode]  — register-register ops
I-type: [    imm[11:0] | rs1 | funct3 | rd | opcode]  — immediate ops
```

### MVP Instruction Set (3 instructions for `x = 1 + 2`)

| Instruction | Type | Encoding | Description |
|------------|------|----------|-------------|
| `addi rd, rs1, imm` | I-type | opcode=0010011, funct3=000 | rd = rs1 + sign_extend(imm) |
| `add rd, rs1, rs2` | R-type | opcode=0110011, funct3=000, funct7=0000000 | rd = rs1 + rs2 |
| `ecall` | I-type | opcode=1110011, imm=0 | System call (used as halt) |

The program `x = 1 + 2`:
```asm
addi x1, x0, 1    # x1 = 0 + 1 = 1
addi x2, x0, 2    # x2 = 0 + 2 = 2
add  x3, x1, x2   # x3 = 1 + 2 = 3
ecall              # halt
```

### Binary Encoding Example

`addi x1, x0, 1`:
```
imm[11:0]    rs1   funct3  rd    opcode
000000000001 00000 000     00001 0010011
```

= 0x00100093

## Public API

```python
class RiscVSimulator:
    def __init__(self, memory_size: int = 65536) -> None: ...
        # Creates a CPU with 32 registers (x0-x31), x0 hardwired to 0

    @property
    def registers(self) -> list[int]: ...

    @property
    def memory(self) -> bytearray: ...

    @property
    def pc(self) -> int: ...

    def load_program(self, machine_code: bytes, start_address: int = 0) -> None: ...
    def step(self) -> Instruction: ...
    def run(self, max_steps: int = 10000) -> list[Instruction]: ...

@dataclass
class Instruction:
    address: int
    raw: int               # 32-bit binary instruction
    mnemonic: str          # "add", "addi", "ecall"
    rd: int | None         # Destination register
    rs1: int | None        # Source register 1
    rs2: int | None        # Source register 2
    imm: int | None        # Immediate value
    format: str            # "R", "I", "S", "B", "U", "J"

def decode(instruction: int) -> Instruction: ...
```

## Data Flow

```
Input:  Raw bytes (machine code) loaded into memory
Output: Instruction trace + final CPU state (registers, memory)
```

## Test Strategy

- Decode known RV32I instruction encodings and verify fields
- Execute `addi x1, x0, 1`: verify x1 = 1
- Execute `add x3, x1, x2`: verify x3 = x1 + x2
- Verify x0 is always 0 (write to x0, read back, must be 0)
- Execute `ecall`: verify execution halts
- End-to-end: run `addi x1,x0,1; addi x2,x0,2; add x3,x1,x2; ecall` → verify x3 = 3

## Historical Future Extensions

The original three-instruction teaching slice listed the following work as
future extensions. The normative Rust completion described below now includes
the entire list; it is retained here as the implementation history.

- `sub`, `and`, `or`, `xor` — more arithmetic/logic
- `lw`, `sw` — memory load/store
- `beq`, `bne`, `blt`, `bge` — conditional branches
- `jal`, `jalr` — jump and link (function calls)
- `lui`, `auipc` — upper immediate (for large constants)
- `sll`, `srl`, `sra` — shifts
- Full RV32I (47 instructions)
- RV32M extension (multiply/divide)

## Normative Rust Completion (RCPU-043)

The Spec 07a Rust functional cell is `riscv-simulator::Rv32ISimulator`. Its
architectural boundary is the RV32I base integer surface already implemented by
the Python oracle, plus the repository's documented M-mode CSR, ECALL trap, and
MRET teaching extension. Selected RV32M operations and compiler host ECALL
services remain available only through the older `RiscVSimulator` compatibility
harness and are not part of this RV32I completion boundary.

The completed machine owns exactly 65,536 bytes of little-endian memory, 32
32-bit GPRs with x0 hardwired to zero, a 32-bit PC, `mstatus`, `mtvec`,
`mscratch`, `mepc`, `mcause`, and halt state. Program origin and length are
validated lifecycle metadata. Program loads are word-aligned and strictly
bounded. Instruction fetches must remain within the installed range; halfword
and word data accesses are naturally aligned and every data access is bounded
by the 64 KiB machine.

`restore`, origin-aware load, direct register/memory access, `step_checked`, and
`run_checked` are typed and fail closed. A failed step preserves the complete
pre-step state. A failed bounded run, including budget exhaustion, preserves the
complete pre-run state. Successful traces include raw instruction, mnemonic,
PC before/after, and complete state before/after.

Completion evidence is 98 preserved unit/consumer tests, seven exact lifecycle
and fault suites, and a reproducible 256-vector Python full-state differential
covering every RV32I/CSR/MRET decode family across four deterministic seeds.
Strict Rustfmt, Clippy (`-D warnings`), and rustdoc (`-D warnings`) pass. Package
line coverage is 91.46% (1,125/1,230).
