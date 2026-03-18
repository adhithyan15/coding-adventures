# 04d — Intel 4004 Simulator

## Overview

The Intel 4004 simulator implements the instruction set of the world's first commercial microprocessor, released by Intel in November 1971. It was a 4-bit CPU designed for the Busicom 141-PF calculator.

Simulating the 4004 is historically fascinating and educationally valuable — it shows how computing started with extremely limited resources (4-bit data bus, 2,300 transistors, 740 kHz clock).

This is an alternative Layer 4 alongside RISC-V, ARM, and WASM.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

## Why the Intel 4004?

- **Historical** — the chip that started the microprocessor revolution
- **Tiny** — 4-bit data, only 46 instructions, 16 registers (4-bit each)
- **Real hardware constraints** — forces you to think about how `1 + 2` works when you only have 4 bits
- **Contrast** — shows how far we've come from 1971 (4004) to 2017 (WASM)

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 4 bits (values 0-15) |
| Registers | 16 × 4-bit (R0-R15, organized as 8 pairs) |
| Accumulator | 4-bit (A) — most arithmetic goes through here |
| RAM | 4096 × 4-bit |
| ROM | 4096 × 8-bit |
| Stack | 3-level hardware stack (for subroutine calls) |
| Clock | 740 kHz (original) |

## MVP Instruction Set (for `1 + 2`)

| Mnemonic | Opcode | Description |
|----------|--------|-------------|
| `LDM n` | 0xDn | Load immediate 4-bit value into accumulator (A = n) |
| `XCH Rn` | 0xBn | Exchange accumulator with register Rn (swap A and Rn) |
| `ADD Rn` | 0x8n | Add register Rn to accumulator (A = A + Rn) |
| `HLT` | 0x01 | Halt (not original 4004 — added for our simulator) |

The program `x = 1 + 2` on a 4004:
```asm
LDM 1       ;; A = 1 (load immediate)
XCH R0      ;; R0 = 1, A = 0 (swap to store in register)
LDM 2       ;; A = 2
ADD R0      ;; A = A + R0 = 2 + 1 = 3
XCH R1      ;; R1 = 3 (result stored in R1)
HLT         ;; stop
```

Note: the 4004 uses an accumulator architecture — one operand is always the accumulator. This is different from RISC-V (register-register) and WASM (stack-based). Three architectural styles for the same computation.

## Public API

```python
class Intel4004Simulator:
    def __init__(self) -> None: ...
        # 16 4-bit registers, 4-bit accumulator, carry flag

    @property
    def accumulator(self) -> int: ...     # 0-15

    @property
    def registers(self) -> list[int]: ... # 16 values, each 0-15

    @property
    def carry(self) -> bool: ...

    def load_program(self, rom: bytes, start_address: int = 0) -> None: ...
    def step(self) -> Intel4004Instruction: ...
    def run(self, max_steps: int = 10000) -> list[Intel4004Instruction]: ...

@dataclass
class Intel4004Instruction:
    address: int
    raw: int                # 8-bit opcode
    mnemonic: str           # "LDM", "ADD", "XCH"
    arg: int | None         # Register number or immediate
    accumulator_before: int
    accumulator_after: int
    carry_before: bool
    carry_after: bool
```

## Test Strategy

- Execute `LDM 1`: verify accumulator = 1
- Execute `XCH R0`: verify R0 = old accumulator, accumulator = old R0
- Execute `ADD R0` with A=2, R0=1: verify A=3, carry=false
- Execute `ADD R0` with A=15, R0=1: verify A=0, carry=true (4-bit overflow!)
- End-to-end: run `1 + 2` program, verify R1 = 3
- Verify 4-bit constraints: all values clamp to 0-15

## Future Extensions

- `SUB Rn` — subtract with borrow
- `JUN addr` — unconditional jump
- `JCN cond, addr` — conditional jump
- `FIM Rp, data` — fetch immediate to register pair
- `SRC Rp` — send register pair to ROM/RAM address
- `WRM` / `RDM` — write/read main memory
- Full 46-instruction set
- BCD (Binary Coded Decimal) arithmetic — what the 4004 was actually designed for
