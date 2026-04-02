# arm1-simulator (Lua)

Behavioral simulator of the ARM1 processor — the first commercial ARM chip, designed by Sophie Wilson and Steve Furber at Acorn Computers and first powered on April 26, 1985.

## What is the ARM1?

The ARM1 had just 25,000 transistors yet implemented a complete 32-bit RISC instruction set. Its accidental low power consumption (~0.1W) made ARM architecture dominant in mobile. Today, ARM-based chips are in over 250 billion devices — every smartphone, tablet, and IoT device.

The ARM1 famously worked correctly on its very first power-on. Sophie Wilson tested it by typing `PRINT PI` at a BBC Micro and got the right answer.

## Key Features

- 32-bit RISC with fixed 32-bit instructions
- 16 visible registers (R0-R15), 25 physical (banked modes)
- **R15 = PC + flags + mode** (unique to ARMv1 — later ARM split these)
- **Conditional execution on every instruction** — not just branches
- **Barrel shifter** — free shift/rotate on every data operation
- 26-bit address space (64 MiB)
- 4 processor modes: USR, FIQ, IRQ, SVC (with banked registers)
- NO multiply instruction (added in ARM2)

## Installation

```bash
luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec
```

## Usage

```lua
local ARM1 = require("coding_adventures.arm1_simulator")

-- Create a simulator with 64 KiB memory
local cpu = ARM1.new(64 * 1024)

-- Load a simple program: R2 = R0 + R1
cpu:load_instructions({
  ARM1.encode_mov_imm(ARM1.COND_AL, 0, 1),    -- MOV R0, #1
  ARM1.encode_mov_imm(ARM1.COND_AL, 1, 2),    -- MOV R1, #2
  ARM1.encode_alu_reg(ARM1.COND_AL, ARM1.OP_ADD, false, 2, 0, 1), -- ADD R2, R0, R1
  ARM1.encode_halt()
})

-- Run until halted
local traces = cpu:run(100)
print("R2 =", cpu:read_register(2))  -- 3

-- Conditional execution example: abs(R0)
-- CMP R0, #0 / RSBLT R0, R0, #0  (two instructions, no branch!)
```

## Instruction Set

### Data Processing (16 operations)
AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC, TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN

All support:
- Immediate operand with 8-bit value × 16 rotation positions
- Register operand with optional shift (LSL, LSR, ASR, ROR, RRX)
- S-bit to update condition flags

### Load/Store
LDR, STR, LDRB, STRB with:
- Pre/post-indexed addressing
- Add/subtract offset
- Write-back

### Block Transfer
LDM, STM in all four modes: IA, IB, DA, DB

### Branch
B, BL — with all 16 condition codes

### SWI
Software interrupt for OS calls; `SWI 0x123456` acts as a halt instruction

## Stack Usage

```
code/specs/07e-arm1-simulator.md  ← specification
code/packages/lua/arm1_simulator/ ← this package
```

## CHANGELOG

See [CHANGELOG.md](CHANGELOG.md).
