# coding-adventures-arm1-simulator (Lua)

ARM1 (ARMv1) behavioral instruction set simulator — the complete ARMv1 instruction set in pure Lua.

## What Is the ARM1?

The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers in Cambridge, UK. First silicon powered on April 26, 1985 — and worked correctly on the very first attempt. With just 25,000 transistors and a 26-bit address space (64 MiB), its accidentally low power consumption (~0.1W) later made the ARM architecture dominant in mobile computing, with over 250 billion chips shipped.

## Architecture

```
Every ARM instruction:
  Bits 31:28 — Condition code (16 conditions, every instr is conditional)
  Bits 27:26 — Instruction type
  Bits 25:0  — Operands, registers, immediate values

R15 = PC (bits 25:2) + N/Z/C/V/I/F flags (bits 31:26) + mode (bits 1:0)

Physical register file (27 registers):
  R0-R15       — base set (always visible)
  R8-R14 (FIQ) — banked for fast interrupt
  R13-R14 (IRQ, SVC) — banked for interrupt/supervisor modes
```

## Installation

```bash
luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec
```

## Usage

```lua
local ARM1 = require("coding_adventures.arm1_simulator")

local cpu = ARM1.new(4096)
ARM1.load_instructions(cpu, {
    ARM1.encode_mov_imm(ARM1.COND_AL, 0, 42),  -- MOV R0, #42
    ARM1.encode_halt(),
})
local traces = ARM1.run(cpu, 100)
print(ARM1.read_register(cpu, 0))  -- 42
```

## API

```lua
-- Construction
local cpu = ARM1.new(memory_size)   -- default 1MB
ARM1.reset(cpu)

-- Registers
ARM1.read_register(cpu, n)          -- n = 0..15
ARM1.write_register(cpu, n, value)
ARM1.get_pc(cpu)                    -- 26-bit byte address
ARM1.set_pc(cpu, addr)
ARM1.get_flags(cpu)                 -- {n, z, c, v} booleans
ARM1.get_mode(cpu)                  -- MODE_USR/FIQ/IRQ/SVC

-- Memory
ARM1.read_word(cpu, addr)
ARM1.write_word(cpu, addr, value)
ARM1.read_byte(cpu, addr)
ARM1.write_byte(cpu, addr, value)
ARM1.load_instructions(cpu, {word, ...})

-- Execution
local trace = ARM1.step(cpu)
local traces = ARM1.run(cpu, max_steps)

-- Core functions (also available directly)
ARM1.barrel_shift(value, shift_type, amount, carry_in, by_register)
ARM1.decode_immediate(imm8, rotate)
ARM1.alu_execute(opcode, a, b, carry_in, shifter_carry, old_v)
ARM1.evaluate_condition(cond, flags)
ARM1.decode(instruction)
ARM1.disassemble(decoded)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
