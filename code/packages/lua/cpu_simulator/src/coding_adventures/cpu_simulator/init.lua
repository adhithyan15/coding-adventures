-- init.lua — CodingAdventures CPU Simulator
--
-- The CPU simulator models the core components of a processor:
--
--   Registers     — fast, small storage inside the CPU
--   Memory        — large, slower storage (RAM)
--   Program Counter (PC) — address of the next instruction
--   Flags         — condition codes (zero, carry, negative, overflow)
--
-- The simulated CPU runs the classic fetch-decode-execute cycle:
--
--   ┌──────────────────────────────────────────────────────────────┐
--   │  1. FETCH   — Read instruction at PC from memory             │
--   │  2. DECODE  — Parse opcode, registers, immediate values      │
--   │  3. EXECUTE — Run the ALU operation                          │
--   │  4. STORE   — Write result to register or memory             │
--   │  5. ADVANCE — Move PC to the next instruction                │
--   │  6. REPEAT  — Unless halted                                  │
--   └──────────────────────────────────────────────────────────────┘
--
-- This CPU is generic — it is not tied to any specific ISA. The ARM
-- simulator (Layer 4) builds on this by providing a concrete instruction
-- set. Our CPU just provides the infrastructure.
--
-- LAYER POSITION:
--
--   Logic Gates → Arithmetic → CPU Simulator (here) → ARM → Assembler → ...
--
-- Packages exported:
--   Memory        — byte-addressable RAM (dense)
--   SparseMemory  — sparse address space (for large but mostly-empty spaces)
--   RegisterFile  — fast CPU register storage

local mem_mod = require("coding_adventures.cpu_simulator.memory")
local RegisterFile = require("coding_adventures.cpu_simulator.register_file")

return {
    Memory        = mem_mod.Memory,
    SparseMemory  = mem_mod.SparseMemory,
    RegisterFile  = RegisterFile,
}
