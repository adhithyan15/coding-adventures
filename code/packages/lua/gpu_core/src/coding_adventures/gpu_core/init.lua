-- gpu_core — Generic Accelerator Processing Element
-- ===================================================
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- It sits in layer 9 of the accelerator stack:
--
--   Layer 11: Logic Gates (AND, OR, XOR, NAND)
--       │
--   Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
--       │
--   Layer 9:  Accelerator Core  ← YOU ARE HERE
--       │
--       ├──→ GPU: Warp/SIMT Engine (32 cores in lockstep)
--       ├──→ TPU: Systolic Array (NxN grid of PEs)
--       └──→ NPU: MAC Array (parallel MACs)
--
-- # Why a Generic Core?
-- ======================
--
-- Every accelerator — NVIDIA GPU, AMD GPU, Intel Arc, ARM Mali, Google TPU,
-- NPU — has a tiny "processing element" (PE) at its heart.  Despite wildly
-- different marketing names (CUDA Core, Stream Processor, Vector Engine, MAC
-- Unit), they all share the same pattern:
--
--   1. They hold LOCAL STATE — a floating-point register file.
--   2. They COMPUTE — FP add, multiply, fused-multiply-add.
--   3. They execute INSTRUCTIONS from a program counter.
--   4. They access LOCAL MEMORY — a small scratchpad.
--
-- By modelling this common pattern, we can simulate one CUDA core today and
-- swap in a PTX instruction set tomorrow without touching the core
-- infrastructure.
--
-- # Execution Model
-- =================
--
-- The GPUCore runs a simple fetch-execute loop:
--
--   ┌──────────────────────────────────────────────────────────┐
--   │  while not halted:                                        │
--   │    instruction = program[pc]                              │
--   │    result = isa.execute(instruction, registers, memory)   │
--   │    pc += 1   (or branch target)                           │
--   │    emit trace record                                      │
--   └──────────────────────────────────────────────────────────┘
--
-- There is no branch predictor, no out-of-order execution, no pipeline.
-- GPUs achieve throughput through MASSIVE PARALLELISM (thousands of cores
-- running simultaneously), not per-core sophistication.
--
-- # The Generic ISA
-- =================
--
-- The GenericISA is a teaching instruction set — not tied to any vendor:
--
--   FADD  Rd, Rs1, Rs2         Rd = Rs1 + Rs2
--   FSUB  Rd, Rs1, Rs2         Rd = Rs1 - Rs2
--   FMUL  Rd, Rs1, Rs2         Rd = Rs1 * Rs2
--   FFMA  Rd, Rs1, Rs2, Rs3    Rd = Rs1 * Rs2 + Rs3  (fused multiply-add)
--   FNEG  Rd, Rs1              Rd = -Rs1
--   FABS  Rd, Rs1              Rd = |Rs1|
--   LOAD  Rd, Rs1, imm         Rd = Mem[Rs1 + imm]
--   STORE Rs1, Rs2, imm        Mem[Rs1 + imm] = Rs2
--   MOV   Rd, Rs1              Rd = Rs1
--   LIMM  Rd, imm              Rd = immediate float
--   BEQ   Rs1, Rs2, offset     if Rs1 == Rs2: PC += offset
--   BLT   Rs1, Rs2, offset     if Rs1 < Rs2:  PC += offset
--   BNE   Rs1, Rs2, offset     if Rs1 != Rs2: PC += offset
--   JMP   target               PC = target
--   NOP                        no-op
--   HALT                       stop execution
--
-- Branch offsets are in instruction units, relative to the NEXT instruction.
-- BEQ R0, R1, +2 means "skip 2 instructions forward if R0 == R1."
--
-- # Register File
-- ================
--
-- Each GPUCore owns an FPRegisterFile — a flat array of floating-point values.
-- Default: 32 registers.  All start at 0.0.
--
--   ┌───────────────────────────────────────┐
--   │         FP Register File (32 regs)    │
--   ├───────────────────────────────────────┤
--   │  R0:  0.0                             │
--   │  R1:  0.0                             │
--   │  ...                                  │
--   │  R31: 0.0                             │
--   └───────────────────────────────────────┘
--
-- # Local Memory
-- ==============
--
-- A small scratchpad — 4096 floats addressable by integer index.
-- Represents per-thread local memory in GPU terminology.
--
--   ┌──────────────────────────────────┐
--   │  Local Memory (4096 slots)       │
--   ├──────────────────────────────────┤
--   │  [0]: 0.0                        │
--   │  [1]: 0.0                        │
--   │  ...                             │
--   └──────────────────────────────────┘
--
-- # Execution Traces
-- ===================
--
-- Every instruction produces a GPUCoreTrace record — a snapshot of what
-- happened on that clock cycle.  This is invaluable for debugging and
-- visualizing program execution:
--
--   {
--     cycle       = 3,
--     pc          = 2,
--     instruction = {opcode="FFMA", rd=3, rs1=0, rs2=1, rs3=2},
--     description = "R3 = R0 * R1 + R2 = 2.0 * 3.0 + 1.0 = 7.0",
--     registers_changed = { R3 = 7.0 },
--     memory_changed    = {},
--     next_pc    = 3,
--     halted     = false,
--   }
--
-- This design mirrors professional GPU debugger output (NVIDIA Nsight, AMD
-- Radeon GPU Profiler) where you can step through shader execution cycle by
-- cycle.

local M = {}

-- ============================================================================
-- Instruction constructors
-- ============================================================================
--
-- These helper functions make programs readable.  Instead of writing:
--
--   {opcode="FFMA", rd=3, rs1=0, rs2=1, rs3=2, immediate=0}
--
-- you write:
--
--   M.ffma(3, 0, 1, 2)
--
-- which is far closer to actual GPU assembly syntax.

--- Create an FADD instruction: Rd = Rs1 + Rs2
function M.fadd(rd, rs1, rs2)
  return {opcode="FADD", rd=rd, rs1=rs1, rs2=rs2, rs3=0, immediate=0}
end

--- Create an FSUB instruction: Rd = Rs1 - Rs2
function M.fsub(rd, rs1, rs2)
  return {opcode="FSUB", rd=rd, rs1=rs1, rs2=rs2, rs3=0, immediate=0}
end

--- Create an FMUL instruction: Rd = Rs1 * Rs2
function M.fmul(rd, rs1, rs2)
  return {opcode="FMUL", rd=rd, rs1=rs1, rs2=rs2, rs3=0, immediate=0}
end

--- Create an FFMA instruction: Rd = Rs1 * Rs2 + Rs3  (fused multiply-add)
-- The FMA is the workhorse of GPU compute — it performs multiply and add
-- in a single instruction with only one rounding step, which is faster and
-- more numerically accurate than separate FMUL + FADD.
function M.ffma(rd, rs1, rs2, rs3)
  return {opcode="FFMA", rd=rd, rs1=rs1, rs2=rs2, rs3=rs3, immediate=0}
end

--- Create an FNEG instruction: Rd = -Rs1
function M.fneg(rd, rs1)
  return {opcode="FNEG", rd=rd, rs1=rs1, rs2=0, rs3=0, immediate=0}
end

--- Create an FABS instruction: Rd = |Rs1|
function M.fabs(rd, rs1)
  return {opcode="FABS", rd=rd, rs1=rs1, rs2=0, rs3=0, immediate=0}
end

--- Create a LOAD instruction: Rd = Mem[Rs1 + imm]
-- Loads a float from local memory at address Rs1 + imm.
function M.load(rd, rs1, imm)
  return {opcode="LOAD", rd=rd, rs1=rs1, rs2=0, rs3=0, immediate=imm or 0}
end

--- Create a STORE instruction: Mem[Rs1 + imm] = Rs2
-- Stores a float to local memory at address Rs1 + imm.
function M.store(rs1, rs2, imm)
  return {opcode="STORE", rd=0, rs1=rs1, rs2=rs2, rs3=0, immediate=imm or 0}
end

--- Create a MOV instruction: Rd = Rs1
function M.mov(rd, rs1)
  return {opcode="MOV", rd=rd, rs1=rs1, rs2=0, rs3=0, immediate=0}
end

--- Create a LIMM instruction: Rd = immediate float value
-- "Load Immediate" — used to load constants into registers.
function M.limm(rd, value)
  return {opcode="LIMM", rd=rd, rs1=0, rs2=0, rs3=0, immediate=value}
end

--- Create a BEQ instruction: if Rs1 == Rs2 then PC += offset
-- Branch offset is in instruction units, relative to the NEXT instruction.
function M.beq(rs1, rs2, offset)
  return {opcode="BEQ", rd=0, rs1=rs1, rs2=rs2, rs3=0, immediate=offset}
end

--- Create a BLT instruction: if Rs1 < Rs2 then PC += offset
function M.blt(rs1, rs2, offset)
  return {opcode="BLT", rd=0, rs1=rs1, rs2=rs2, rs3=0, immediate=offset}
end

--- Create a BNE instruction: if Rs1 != Rs2 then PC += offset
function M.bne(rs1, rs2, offset)
  return {opcode="BNE", rd=0, rs1=rs1, rs2=rs2, rs3=0, immediate=offset}
end

--- Create a JMP instruction: PC = target (absolute)
function M.jmp(target)
  return {opcode="JMP", rd=0, rs1=0, rs2=0, rs3=0, immediate=target}
end

--- Create a NOP instruction: do nothing, advance PC by 1
function M.nop()
  return {opcode="NOP", rd=0, rs1=0, rs2=0, rs3=0, immediate=0}
end

--- Create a HALT instruction: stop execution
function M.halt()
  return {opcode="HALT", rd=0, rs1=0, rs2=0, rs3=0, immediate=0}
end

-- ============================================================================
-- FPRegisterFile
-- ============================================================================
--
-- A flat array of 32 (or more) floating-point registers, all initialized to
-- 0.0.  Register access is bounds-checked to catch programming errors early.

local FPRegisterFile = {}
FPRegisterFile.__index = FPRegisterFile

--- Create a new FPRegisterFile with `num_registers` slots (default 32).
function FPRegisterFile.new(num_registers)
  local self = setmetatable({}, FPRegisterFile)
  self._num = num_registers or 32
  self._regs = {}
  for i = 0, self._num - 1 do
    self._regs[i] = 0.0
  end
  return self
end

--- Read the value of register `index`.  Raises on out-of-bounds access.
function FPRegisterFile:read(index)
  assert(index >= 0 and index < self._num,
    string.format("register index %d out of range [0,%d)", index, self._num))
  return self._regs[index]
end

--- Write `value` to register `index`.  Raises on out-of-bounds access.
function FPRegisterFile:write(index, value)
  assert(index >= 0 and index < self._num,
    string.format("register index %d out of range [0,%d)", index, self._num))
  self._regs[index] = value
end

--- Reset all registers to 0.0.
function FPRegisterFile:reset()
  for i = 0, self._num - 1 do
    self._regs[i] = 0.0
  end
end

--- Return the number of registers.
function FPRegisterFile:size()
  return self._num
end

M.FPRegisterFile = FPRegisterFile

-- ============================================================================
-- LocalMemory
-- ============================================================================
--
-- A simple scratchpad modelled as a Lua table indexed by integer address.
-- Represents per-thread shared memory in GPU terminology.
--
-- In real hardware this would be backed by SRAM — fast but small.
-- Typical sizes: 32 KB to 96 KB per SM (Streaming Multiprocessor).

local LocalMemory = {}
LocalMemory.__index = LocalMemory

--- Create a new LocalMemory with `size` addressable slots (default 4096).
function LocalMemory.new(size)
  local self = setmetatable({}, LocalMemory)
  self._size = size or 4096
  self._mem = {}
  return self
end

--- Load a float from address `addr`.  Returns 0.0 if never written.
function LocalMemory:load(addr)
  assert(addr >= 0 and addr < self._size,
    string.format("memory address %d out of range [0,%d)", addr, self._size))
  return self._mem[addr] or 0.0
end

--- Store a float value at address `addr`.
function LocalMemory:store(addr, value)
  assert(addr >= 0 and addr < self._size,
    string.format("memory address %d out of range [0,%d)", addr, self._size))
  self._mem[addr] = value
end

--- Reset all memory to 0.0.
function LocalMemory:reset()
  self._mem = {}
end

M.LocalMemory = LocalMemory

-- ============================================================================
-- GenericISA
-- ============================================================================
--
-- The GenericISA implements the InstructionSet protocol for the teaching
-- instruction set described in the module header.  It is intentionally simple
-- — real vendor ISAs (PTX, GCN) have hundreds of opcodes, but the pattern
-- here is identical.
--
-- To add a new ISA, create a table with a single method:
--   execute(instruction, registers, memory) → {registers_changed, memory_changed,
--                                               description, next_pc_offset, halted}

local GenericISA = {}
GenericISA.__index = GenericISA

function GenericISA.new()
  return setmetatable({name = "Generic"}, GenericISA)
end

--- Execute one instruction.  Returns a result table:
--
--   {
--     registers_changed = { [reg_index] = new_value, ... },
--     memory_changed    = { [addr] = new_value, ... },
--     description       = "human readable string",
--     next_pc_offset    = integer,   -- added to current pc+1 to get next pc
--     halted            = boolean,
--   }
function GenericISA:execute(instr, registers, memory)
  local op = instr.opcode
  local result = {
    registers_changed = {},
    memory_changed    = {},
    description       = "",
    next_pc_offset    = 0,   -- default: sequential execution
    halted            = false,
  }

  if op == "FADD" then
    -- Rd = Rs1 + Rs2
    local a = registers:read(instr.rs1)
    local b = registers:read(instr.rs2)
    local v = a + b
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = R%d + R%d = %g + %g = %g",
      instr.rd, instr.rs1, instr.rs2, a, b, v)

  elseif op == "FSUB" then
    -- Rd = Rs1 - Rs2
    local a = registers:read(instr.rs1)
    local b = registers:read(instr.rs2)
    local v = a - b
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = R%d - R%d = %g - %g = %g",
      instr.rd, instr.rs1, instr.rs2, a, b, v)

  elseif op == "FMUL" then
    -- Rd = Rs1 * Rs2
    local a = registers:read(instr.rs1)
    local b = registers:read(instr.rs2)
    local v = a * b
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = R%d * R%d = %g * %g = %g",
      instr.rd, instr.rs1, instr.rs2, a, b, v)

  elseif op == "FFMA" then
    -- Rd = Rs1 * Rs2 + Rs3  (fused multiply-add)
    -- In hardware this is a SINGLE operation — no intermediate rounding.
    local a = registers:read(instr.rs1)
    local b = registers:read(instr.rs2)
    local c = registers:read(instr.rs3)
    local v = a * b + c
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = R%d * R%d + R%d = %g * %g + %g = %g",
      instr.rd, instr.rs1, instr.rs2, instr.rs3, a, b, c, v)

  elseif op == "FNEG" then
    -- Rd = -Rs1
    local a = registers:read(instr.rs1)
    local v = -a
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = -R%d = -%g = %g",
      instr.rd, instr.rs1, a, v)

  elseif op == "FABS" then
    -- Rd = |Rs1|
    local a = registers:read(instr.rs1)
    local v = math.abs(a)
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = |R%d| = |%g| = %g",
      instr.rd, instr.rs1, a, v)

  elseif op == "LOAD" then
    -- Rd = Mem[Rs1 + imm]
    local base = registers:read(instr.rs1)
    local addr = math.floor(base + instr.immediate)
    local v = memory:load(addr)
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = Mem[R%d + %g] = Mem[%d] = %g",
      instr.rd, instr.rs1, instr.immediate, addr, v)

  elseif op == "STORE" then
    -- Mem[Rs1 + imm] = Rs2
    local base = registers:read(instr.rs1)
    local addr = math.floor(base + instr.immediate)
    local v = registers:read(instr.rs2)
    memory:store(addr, v)
    result.memory_changed[addr] = v
    result.description = string.format("Mem[R%d + %g] = R%d → Mem[%d] = %g",
      instr.rs1, instr.immediate, instr.rs2, addr, v)

  elseif op == "MOV" then
    -- Rd = Rs1
    local v = registers:read(instr.rs1)
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = R%d = %g", instr.rd, instr.rs1, v)

  elseif op == "LIMM" then
    -- Rd = immediate float
    local v = instr.immediate
    registers:write(instr.rd, v)
    result.registers_changed[instr.rd] = v
    result.description = string.format("R%d = %g (immediate)", instr.rd, v)

  elseif op == "BEQ" then
    -- if Rs1 == Rs2 then PC = current_pc + offset
    -- Core computes: next_pc = current_pc + 1 + next_pc_offset
    -- So next_pc_offset = immediate - 1 to get target = current_pc + immediate
    local a = registers:read(instr.rs1)
    local b = registers:read(instr.rs2)
    if a == b then
      result.next_pc_offset = math.floor(instr.immediate) - 1
      result.description = string.format("BEQ R%d, R%d: %g == %g → taken (offset %d)",
        instr.rs1, instr.rs2, a, b, result.next_pc_offset)
    else
      result.description = string.format("BEQ R%d, R%d: %g != %g → not taken",
        instr.rs1, instr.rs2, a, b)
    end

  elseif op == "BLT" then
    -- if Rs1 < Rs2 then PC = current_pc + offset
    local a = registers:read(instr.rs1)
    local b = registers:read(instr.rs2)
    if a < b then
      result.next_pc_offset = math.floor(instr.immediate) - 1
      result.description = string.format("BLT R%d, R%d: %g < %g → taken (offset %d)",
        instr.rs1, instr.rs2, a, b, result.next_pc_offset)
    else
      result.description = string.format("BLT R%d, R%d: %g >= %g → not taken",
        instr.rs1, instr.rs2, a, b)
    end

  elseif op == "BNE" then
    -- if Rs1 != Rs2 then PC = current_pc + offset
    local a = registers:read(instr.rs1)
    local b = registers:read(instr.rs2)
    if a ~= b then
      result.next_pc_offset = math.floor(instr.immediate) - 1
      result.description = string.format("BNE R%d, R%d: %g != %g → taken (offset %d)",
        instr.rs1, instr.rs2, a, b, result.next_pc_offset)
    else
      result.description = string.format("BNE R%d, R%d: %g == %g → not taken",
        instr.rs1, instr.rs2, a, b)
    end

  elseif op == "JMP" then
    -- PC = target (absolute jump — set next_pc_offset so core computes correctly)
    -- The core does: next_pc = current_pc + 1 + next_pc_offset
    -- We want: next_pc = target
    -- So: next_pc_offset = target - current_pc - 1
    -- But we don't have current_pc here, so we encode target in a special way.
    -- Instead, we return a special flag "jmp_target" for the core to handle.
    result.jmp_target = math.floor(instr.immediate)
    result.description = string.format("JMP %d", result.jmp_target)

  elseif op == "NOP" then
    result.description = "NOP"

  elseif op == "HALT" then
    result.halted = true
    result.description = "HALT"

  else
    error(string.format("Unknown opcode: %s", tostring(op)))
  end

  return result
end

M.GenericISA = GenericISA

-- ============================================================================
-- GPUCore
-- ============================================================================
--
-- The main simulation object.  One GPUCore = one CUDA core / stream processor
-- / vector engine — whichever vendor ISA you plug in.
--
-- Usage:
--
--   local isa  = gpu_core.GenericISA.new()
--   local core = gpu_core.GPUCore.new(isa)
--   core:load_program({
--     gpu_core.limm(0, 2.0),    -- R0 = 2.0
--     gpu_core.limm(1, 3.0),    -- R1 = 3.0
--     gpu_core.fmul(2, 0, 1),   -- R2 = R0 * R1 = 6.0
--     gpu_core.halt(),
--   })
--   local traces = core:run()
--   print(core.registers:read(2))   -- 6.0

local GPUCore = {}
GPUCore.__index = GPUCore

--- Create a new GPUCore.
-- @param isa            InstructionSet object with :execute() method.
-- @param num_registers  Number of FP registers (default 32).
-- @param memory_size    Local memory size in slots (default 4096).
function GPUCore.new(isa, num_registers, memory_size)
  local self = setmetatable({}, GPUCore)
  self.isa       = isa or GenericISA.new()
  self.registers = FPRegisterFile.new(num_registers or 32)
  self.memory    = LocalMemory.new(memory_size or 4096)
  self.program   = {}
  self.pc        = 0
  self.cycle     = 0
  self.halted    = false
  return self
end

--- Load a program (table of instruction tables) into the core.
-- Resets pc, cycle, and halted state.
function GPUCore:load_program(program)
  self.program = program
  self.pc      = 0
  self.cycle   = 0
  self.halted  = false
end

--- Execute one clock cycle.
-- Returns a GPUCoreTrace table describing what happened.
-- If already halted, returns a trace with halted=true and no changes.
function GPUCore:step()
  -- If we are already halted, return a sentinel trace.
  if self.halted then
    return {
      cycle              = self.cycle,
      pc                 = self.pc,
      instruction        = nil,
      description        = "already halted",
      registers_changed  = {},
      memory_changed     = {},
      next_pc            = self.pc,
      halted             = true,
    }
  end

  -- Fetch: bounds-check the program counter.
  if self.pc < 0 or self.pc >= #self.program then
    self.halted = true
    return {
      cycle              = self.cycle,
      pc                 = self.pc,
      instruction        = nil,
      description        = string.format("PC %d out of range — implicit HALT", self.pc),
      registers_changed  = {},
      memory_changed     = {},
      next_pc            = self.pc,
      halted             = true,
    }
  end

  local instr = self.program[self.pc + 1]  -- Lua arrays are 1-indexed
  local current_pc = self.pc

  -- Execute: delegate to the ISA.
  local result = self.isa:execute(instr, self.registers, self.memory)

  -- Compute next PC.
  local next_pc
  if result.jmp_target ~= nil then
    -- Absolute jump
    next_pc = result.jmp_target
  else
    -- Sequential + optional branch offset
    next_pc = current_pc + 1 + result.next_pc_offset
  end

  self.pc = next_pc
  self.cycle = self.cycle + 1

  if result.halted then
    self.halted = true
  end

  return {
    cycle             = self.cycle - 1,
    pc                = current_pc,
    instruction       = instr,
    description       = result.description,
    registers_changed = result.registers_changed,
    memory_changed    = result.memory_changed,
    next_pc           = next_pc,
    halted            = result.halted or false,
  }
end

--- Run the program until HALT or max_steps is reached.
-- Returns a list of GPUCoreTrace records — one per cycle executed.
-- @param max_steps  Safety limit (default 10000) to prevent infinite loops.
function GPUCore:run(max_steps)
  max_steps = max_steps or 10000
  local traces = {}
  for _ = 1, max_steps do
    local trace = self:step()
    traces[#traces + 1] = trace
    if trace.halted then
      break
    end
  end
  return traces
end

--- Reset the core to initial state.
-- Keeps the loaded program; resets registers, memory, PC, and cycle count.
function GPUCore:reset()
  self.registers:reset()
  self.memory:reset()
  self.pc     = 0
  self.cycle  = 0
  self.halted = false
end

M.GPUCore = GPUCore

-- ============================================================================
-- Example programs (exported for documentation and testing)
-- ============================================================================
--
-- These programs illustrate the kinds of kernels that run on real GPUs.

--- SAXPY kernel: y = a * x + y
-- The canonical GPU "hello world" — a single FMA computes one output element.
-- In a real GPU, thousands of threads run this simultaneously on different
-- elements of the vectors x[] and y[].
--
--   R0 = a (scalar multiplier)
--   R1 = x (one element)
--   R2 = y (one element, also accumulator)
--   R3 = result = a*x + y
M.SAXPY_PROGRAM = function(a, x, y)
  return {
    M.limm(0, a),          -- R0 = a
    M.limm(1, x),          -- R1 = x
    M.limm(2, y),          -- R2 = y
    M.ffma(3, 0, 1, 2),    -- R3 = a * x + y
    M.halt(),
  }
end

--- Dot product: sum = sum_i(A[i] * B[i]) for 3 elements
-- Each element pair is multiplied and accumulated.
-- R6 holds the running sum.
M.DOT_PRODUCT_PROGRAM = {
  M.limm(0, 1.0),        -- R0 = A[0]
  M.limm(1, 2.0),        -- R1 = A[1]
  M.limm(2, 3.0),        -- R2 = A[2]
  M.limm(3, 4.0),        -- R3 = B[0]
  M.limm(4, 5.0),        -- R4 = B[1]
  M.limm(5, 6.0),        -- R5 = B[2]
  M.limm(6, 0.0),        -- R6 = accumulator = 0.0
  M.ffma(6, 0, 3, 6),    -- R6 = 1.0 * 4.0 + 0.0 = 4.0
  M.ffma(6, 1, 4, 6),    -- R6 = 2.0 * 5.0 + 4.0 = 14.0
  M.ffma(6, 2, 5, 6),    -- R6 = 3.0 * 6.0 + 14.0 = 32.0
  M.halt(),
}

return M
