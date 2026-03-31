-- Tests for gpu_core
-- ===================
--
-- Comprehensive busted test suite for the GPU core package.
--
-- Test coverage:
--   - Module loads and exposes the public API
--   - FPRegisterFile: read, write, reset, bounds checking
--   - LocalMemory: load, store, reset, bounds checking
--   - GenericISA: all opcodes (FADD/FSUB/FMUL/FFMA/FNEG/FABS/LOAD/STORE/MOV/LIMM)
--   - GenericISA: control flow (BEQ/BLT/BNE/JMP/NOP/HALT)
--   - GPUCore: load_program, step, run, reset
--   - Complete programs: SAXPY, dot product, loop

package.path = (
    "../src/?.lua;"  ..
    "../src/?/init.lua;"  ..
    package.path
)

local gpu_core = require("coding_adventures.gpu_core")

-- ============================================================================
-- Module API
-- ============================================================================

describe("gpu_core module", function()
  it("loads successfully", function()
    assert.is_not_nil(gpu_core)
  end)

  it("exposes GPUCore", function()
    assert.is_not_nil(gpu_core.GPUCore)
  end)

  it("exposes GenericISA", function()
    assert.is_not_nil(gpu_core.GenericISA)
  end)

  it("exposes FPRegisterFile", function()
    assert.is_not_nil(gpu_core.FPRegisterFile)
  end)

  it("exposes LocalMemory", function()
    assert.is_not_nil(gpu_core.LocalMemory)
  end)

  it("exposes instruction constructors", function()
    assert.is_function(gpu_core.fadd)
    assert.is_function(gpu_core.fsub)
    assert.is_function(gpu_core.fmul)
    assert.is_function(gpu_core.ffma)
    assert.is_function(gpu_core.fneg)
    assert.is_function(gpu_core.fabs)
    assert.is_function(gpu_core.load)
    assert.is_function(gpu_core.store)
    assert.is_function(gpu_core.mov)
    assert.is_function(gpu_core.limm)
    assert.is_function(gpu_core.beq)
    assert.is_function(gpu_core.blt)
    assert.is_function(gpu_core.bne)
    assert.is_function(gpu_core.jmp)
    assert.is_function(gpu_core.nop)
    assert.is_function(gpu_core.halt)
  end)
end)

-- ============================================================================
-- Instruction constructors
-- ============================================================================

describe("instruction constructors", function()
  it("fadd sets opcode and registers", function()
    local i = gpu_core.fadd(1, 2, 3)
    assert.equal("FADD", i.opcode)
    assert.equal(1, i.rd)
    assert.equal(2, i.rs1)
    assert.equal(3, i.rs2)
  end)

  it("ffma sets rs3", function()
    local i = gpu_core.ffma(0, 1, 2, 3)
    assert.equal("FFMA", i.opcode)
    assert.equal(0, i.rd)
    assert.equal(1, i.rs1)
    assert.equal(2, i.rs2)
    assert.equal(3, i.rs3)
  end)

  it("limm stores immediate value", function()
    local i = gpu_core.limm(5, 3.14)
    assert.equal("LIMM", i.opcode)
    assert.equal(5, i.rd)
    assert.near(3.14, i.immediate, 1e-6)
  end)

  it("halt has correct opcode", function()
    local i = gpu_core.halt()
    assert.equal("HALT", i.opcode)
  end)

  it("beq stores offset in immediate", function()
    local i = gpu_core.beq(0, 1, -3)
    assert.equal("BEQ", i.opcode)
    assert.equal(-3, i.immediate)
  end)

  it("jmp stores target in immediate", function()
    local i = gpu_core.jmp(7)
    assert.equal("JMP", i.opcode)
    assert.equal(7, i.immediate)
  end)
end)

-- ============================================================================
-- FPRegisterFile
-- ============================================================================

describe("FPRegisterFile", function()
  it("initializes with zeros", function()
    local rf = gpu_core.FPRegisterFile.new(8)
    for i = 0, 7 do
      assert.equal(0.0, rf:read(i))
    end
  end)

  it("reads and writes correctly", function()
    local rf = gpu_core.FPRegisterFile.new(4)
    rf:write(2, 3.14)
    assert.near(3.14, rf:read(2), 1e-6)
  end)

  it("write to one register does not affect others", function()
    local rf = gpu_core.FPRegisterFile.new(4)
    rf:write(1, 99.0)
    assert.equal(0.0, rf:read(0))
    assert.equal(0.0, rf:read(2))
    assert.equal(0.0, rf:read(3))
  end)

  it("reset clears all registers", function()
    local rf = gpu_core.FPRegisterFile.new(4)
    rf:write(0, 1.0)
    rf:write(3, -5.0)
    rf:reset()
    for i = 0, 3 do
      assert.equal(0.0, rf:read(i))
    end
  end)

  it("reports correct size", function()
    local rf = gpu_core.FPRegisterFile.new(16)
    assert.equal(16, rf:size())
  end)

  it("raises on out-of-bounds read", function()
    local rf = gpu_core.FPRegisterFile.new(4)
    assert.has_error(function() rf:read(4) end)
    assert.has_error(function() rf:read(-1) end)
  end)

  it("raises on out-of-bounds write", function()
    local rf = gpu_core.FPRegisterFile.new(4)
    assert.has_error(function() rf:write(4, 1.0) end)
  end)

  it("defaults to 32 registers", function()
    local rf = gpu_core.FPRegisterFile.new()
    assert.equal(32, rf:size())
  end)
end)

-- ============================================================================
-- LocalMemory
-- ============================================================================

describe("LocalMemory", function()
  it("returns 0.0 for unwritten addresses", function()
    local mem = gpu_core.LocalMemory.new(16)
    assert.equal(0.0, mem:load(0))
    assert.equal(0.0, mem:load(15))
  end)

  it("stores and loads correctly", function()
    local mem = gpu_core.LocalMemory.new(16)
    mem:store(5, 2.718)
    assert.near(2.718, mem:load(5), 1e-6)
  end)

  it("reset clears stored values", function()
    local mem = gpu_core.LocalMemory.new(8)
    mem:store(3, 42.0)
    mem:reset()
    assert.equal(0.0, mem:load(3))
  end)

  it("raises on out-of-bounds load", function()
    local mem = gpu_core.LocalMemory.new(8)
    assert.has_error(function() mem:load(8) end)
    assert.has_error(function() mem:load(-1) end)
  end)

  it("raises on out-of-bounds store", function()
    local mem = gpu_core.LocalMemory.new(8)
    assert.has_error(function() mem:store(8, 1.0) end)
  end)
end)

-- ============================================================================
-- GenericISA — arithmetic
-- ============================================================================

describe("GenericISA arithmetic", function()
  local isa, rf, mem

  before_each(function()
    isa = gpu_core.GenericISA.new()
    rf  = gpu_core.FPRegisterFile.new(8)
    mem = gpu_core.LocalMemory.new(64)
  end)

  it("FADD computes sum", function()
    rf:write(1, 3.0)
    rf:write(2, 4.0)
    isa:execute(gpu_core.fadd(0, 1, 2), rf, mem)
    assert.near(7.0, rf:read(0), 1e-6)
  end)

  it("FSUB computes difference", function()
    rf:write(1, 10.0)
    rf:write(2, 3.0)
    isa:execute(gpu_core.fsub(0, 1, 2), rf, mem)
    assert.near(7.0, rf:read(0), 1e-6)
  end)

  it("FMUL computes product", function()
    rf:write(1, 3.0)
    rf:write(2, 5.0)
    isa:execute(gpu_core.fmul(0, 1, 2), rf, mem)
    assert.near(15.0, rf:read(0), 1e-6)
  end)

  it("FFMA computes fused multiply-add", function()
    -- R3 = R0 * R1 + R2 = 2 * 3 + 1 = 7
    rf:write(0, 2.0)
    rf:write(1, 3.0)
    rf:write(2, 1.0)
    isa:execute(gpu_core.ffma(3, 0, 1, 2), rf, mem)
    assert.near(7.0, rf:read(3), 1e-6)
  end)

  it("FNEG negates value", function()
    rf:write(1, 5.0)
    isa:execute(gpu_core.fneg(0, 1), rf, mem)
    assert.near(-5.0, rf:read(0), 1e-6)
  end)

  it("FNEG of negative gives positive", function()
    rf:write(1, -3.0)
    isa:execute(gpu_core.fneg(0, 1), rf, mem)
    assert.near(3.0, rf:read(0), 1e-6)
  end)

  it("FABS returns absolute value", function()
    rf:write(1, -7.5)
    isa:execute(gpu_core.fabs(0, 1), rf, mem)
    assert.near(7.5, rf:read(0), 1e-6)
  end)

  it("FABS of positive is unchanged", function()
    rf:write(1, 4.2)
    isa:execute(gpu_core.fabs(0, 1), rf, mem)
    assert.near(4.2, rf:read(0), 1e-6)
  end)

  it("MOV copies register value", function()
    rf:write(2, 9.9)
    isa:execute(gpu_core.mov(0, 2), rf, mem)
    assert.near(9.9, rf:read(0), 1e-6)
  end)

  it("LIMM loads immediate constant", function()
    isa:execute(gpu_core.limm(3, 3.14159), rf, mem)
    assert.near(3.14159, rf:read(3), 1e-5)
  end)
end)

-- ============================================================================
-- GenericISA — memory
-- ============================================================================

describe("GenericISA memory", function()
  local isa, rf, mem

  before_each(function()
    isa = gpu_core.GenericISA.new()
    rf  = gpu_core.FPRegisterFile.new(8)
    mem = gpu_core.LocalMemory.new(64)
  end)

  it("STORE writes to memory", function()
    rf:write(0, 0.0)  -- base address = 0
    rf:write(1, 42.0)
    isa:execute(gpu_core.store(0, 1, 5), rf, mem)
    assert.near(42.0, mem:load(5), 1e-6)
  end)

  it("LOAD reads from memory", function()
    mem:store(10, 3.14)
    rf:write(1, 5.0)   -- base = 5
    isa:execute(gpu_core.load(0, 1, 5), rf, mem)
    assert.near(3.14, rf:read(0), 1e-5)
  end)

  it("LOAD from unwritten address returns 0.0", function()
    rf:write(1, 0.0)
    isa:execute(gpu_core.load(0, 1, 20), rf, mem)
    assert.equal(0.0, rf:read(0))
  end)

  it("STORE then LOAD roundtrip", function()
    rf:write(0, 0.0)
    rf:write(1, 7.77)
    isa:execute(gpu_core.store(0, 1, 3), rf, mem)
    rf:write(2, 0.0)
    isa:execute(gpu_core.load(2, 0, 3), rf, mem)
    assert.near(7.77, rf:read(2), 1e-5)
  end)
end)

-- ============================================================================
-- GenericISA — control flow
-- ============================================================================

describe("GenericISA control flow", function()
  local isa, rf, mem

  before_each(function()
    isa = gpu_core.GenericISA.new()
    rf  = gpu_core.FPRegisterFile.new(8)
    mem = gpu_core.LocalMemory.new(64)
  end)

  it("BEQ taken when Rs1 == Rs2", function()
    rf:write(0, 5.0)
    rf:write(1, 5.0)
    local r = isa:execute(gpu_core.beq(0, 1, 3), rf, mem)
    assert.equal(3, r.next_pc_offset)
  end)

  it("BEQ not taken when Rs1 != Rs2", function()
    rf:write(0, 5.0)
    rf:write(1, 6.0)
    local r = isa:execute(gpu_core.beq(0, 1, 3), rf, mem)
    assert.equal(0, r.next_pc_offset)
  end)

  it("BLT taken when Rs1 < Rs2", function()
    rf:write(0, 2.0)
    rf:write(1, 5.0)
    local r = isa:execute(gpu_core.blt(0, 1, -2), rf, mem)
    assert.equal(-2, r.next_pc_offset)
  end)

  it("BLT not taken when Rs1 >= Rs2", function()
    rf:write(0, 5.0)
    rf:write(1, 3.0)
    local r = isa:execute(gpu_core.blt(0, 1, -2), rf, mem)
    assert.equal(0, r.next_pc_offset)
  end)

  it("BNE taken when Rs1 != Rs2", function()
    rf:write(0, 1.0)
    rf:write(1, 2.0)
    local r = isa:execute(gpu_core.bne(0, 1, 4), rf, mem)
    assert.equal(4, r.next_pc_offset)
  end)

  it("BNE not taken when Rs1 == Rs2", function()
    rf:write(0, 3.0)
    rf:write(1, 3.0)
    local r = isa:execute(gpu_core.bne(0, 1, 4), rf, mem)
    assert.equal(0, r.next_pc_offset)
  end)

  it("JMP stores target", function()
    local r = isa:execute(gpu_core.jmp(10), rf, mem)
    assert.equal(10, r.jmp_target)
  end)

  it("NOP is not halted", function()
    local r = isa:execute(gpu_core.nop(), rf, mem)
    assert.is_false(r.halted)
    assert.equal(0, r.next_pc_offset)
  end)

  it("HALT sets halted flag", function()
    local r = isa:execute(gpu_core.halt(), rf, mem)
    assert.is_true(r.halted)
  end)

  it("unknown opcode raises error", function()
    assert.has_error(function()
      isa:execute({opcode="BOGUS", rd=0, rs1=0, rs2=0, rs3=0, immediate=0}, rf, mem)
    end)
  end)
end)

-- ============================================================================
-- GPUCore — basic operation
-- ============================================================================

describe("GPUCore basic operation", function()
  it("creates with defaults", function()
    local core = gpu_core.GPUCore.new()
    assert.is_false(core.halted)
    assert.equal(0, core.pc)
    assert.equal(0, core.cycle)
  end)

  it("load_program resets state", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.halt() })
    assert.equal(0, core.pc)
    assert.is_false(core.halted)
  end)

  it("step returns a trace table", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.limm(0, 1.0), gpu_core.halt() })
    local t = core:step()
    assert.is_table(t)
    assert.equal(0, t.cycle)
    assert.equal(0, t.pc)
    assert.is_table(t.registers_changed)
    assert.is_table(t.memory_changed)
    assert.is_boolean(t.halted)
  end)

  it("LIMM followed by HALT stores value and halts", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.limm(0, 42.0), gpu_core.halt() })
    core:step()  -- LIMM
    core:step()  -- HALT
    assert.near(42.0, core.registers:read(0), 1e-6)
    assert.is_true(core.halted)
  end)

  it("run returns traces", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.nop(), gpu_core.halt() })
    local traces = core:run()
    assert.equal(2, #traces)
    assert.is_true(traces[2].halted)
  end)

  it("run stops at HALT", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.halt(), gpu_core.nop() })
    local traces = core:run()
    assert.equal(1, #traces)
  end)

  it("step on empty program halts immediately", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({})
    local t = core:step()
    assert.is_true(t.halted)
  end)

  it("reset clears registers and PC", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.limm(0, 5.0), gpu_core.halt() })
    core:run()
    core:reset()
    assert.equal(0.0, core.registers:read(0))
    assert.equal(0, core.pc)
    assert.is_false(core.halted)
    assert.equal(0, core.cycle)
  end)

  it("step after halted returns halted trace", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.halt() })
    core:step()  -- HALT
    local t = core:step()  -- after halt
    assert.is_true(t.halted)
  end)
end)

-- ============================================================================
-- GPUCore — complete programs
-- ============================================================================

describe("GPUCore complete programs", function()
  it("SAXPY: R3 = 2.0 * 3.0 + 1.0 = 7.0", function()
    local core = gpu_core.GPUCore.new()
    core:load_program(gpu_core.SAXPY_PROGRAM(2.0, 3.0, 1.0))
    core:run()
    assert.near(7.0, core.registers:read(3), 1e-6)
  end)

  it("SAXPY: R3 = 1.5 * 4.0 + 0.5 = 6.5", function()
    local core = gpu_core.GPUCore.new()
    core:load_program(gpu_core.SAXPY_PROGRAM(1.5, 4.0, 0.5))
    core:run()
    assert.near(6.5, core.registers:read(3), 1e-6)
  end)

  it("dot product: [1,2,3]·[4,5,6] = 32", function()
    local core = gpu_core.GPUCore.new()
    core:load_program(gpu_core.DOT_PRODUCT_PROGRAM)
    core:run()
    assert.near(32.0, core.registers:read(6), 1e-6)
  end)

  it("loop: sum 1+2+3+4 = 10", function()
    -- Sum first N numbers using BLT loop
    local core = gpu_core.GPUCore.new()
    core:load_program({
      gpu_core.limm(0, 0.0),   -- R0 = sum = 0
      gpu_core.limm(1, 1.0),   -- R1 = i = 1
      gpu_core.limm(2, 1.0),   -- R2 = increment = 1
      gpu_core.limm(3, 5.0),   -- R3 = limit = 5
      -- Loop body at PC=4:
      gpu_core.fadd(0, 0, 1),  -- sum += i
      gpu_core.fadd(1, 1, 2),  -- i += 1
      gpu_core.blt(1, 3, -2),  -- if i < 5: back 2
      gpu_core.halt(),
    })
    core:run()
    assert.near(10.0, core.registers:read(0), 1e-6)
  end)

  it("absolute value program: R1 = |R0| where R0 = -3.14", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({
      gpu_core.limm(0, -3.14),
      gpu_core.fabs(1, 0),
      gpu_core.halt(),
    })
    core:run()
    assert.near(3.14, core.registers:read(1), 1e-5)
  end)

  it("JMP jumps over instructions", function()
    local core = gpu_core.GPUCore.new()
    -- PC 0: limm R0=99 (should be skipped by JMP at PC 2)
    -- PC 1: JMP 3  (jump to PC 3)
    -- PC 2: limm R0=99 (should be skipped)
    -- PC 3: limm R1=7
    -- PC 4: halt
    core:load_program({
      gpu_core.jmp(2),          -- jump to index 2 (0-based)
      gpu_core.limm(0, 99.0),   -- should be skipped
      gpu_core.limm(1, 7.0),
      gpu_core.halt(),
    })
    core:run()
    assert.near(0.0, core.registers:read(0), 1e-6)  -- was never written
    assert.near(7.0, core.registers:read(1), 1e-6)
  end)

  it("memory store-load roundtrip in program", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({
      gpu_core.limm(0, 0.0),    -- R0 = base address 0
      gpu_core.limm(1, 3.14),   -- R1 = value to store
      gpu_core.store(0, 1, 10), -- Mem[10] = R1 = 3.14
      gpu_core.load(2, 0, 10),  -- R2 = Mem[10]
      gpu_core.halt(),
    })
    core:run()
    assert.near(3.14, core.registers:read(2), 1e-5)
  end)

  it("BEQ branches correctly on equality", function()
    local core = gpu_core.GPUCore.new()
    -- R0 and R1 are both 5.0
    -- BEQ R0, R1, +1 should skip the LIMM
    -- PC 0: limm R0=5
    -- PC 1: limm R1=5
    -- PC 2: BEQ 0,1 +1  → skip PC 3
    -- PC 3: limm R2=99  (should be skipped)
    -- PC 4: halt
    core:load_program({
      gpu_core.limm(0, 5.0),
      gpu_core.limm(1, 5.0),
      gpu_core.beq(0, 1, 1),
      gpu_core.limm(2, 99.0),
      gpu_core.halt(),
    })
    core:run()
    assert.near(0.0, core.registers:read(2), 1e-6)  -- skipped
  end)

  it("max_steps limits execution", function()
    -- Infinite loop: jmp back to self
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.jmp(0) })
    local traces = core:run(5)
    assert.equal(5, #traces)
    assert.is_false(traces[5].halted)
  end)
end)

-- ============================================================================
-- Trace content
-- ============================================================================

describe("execution traces", function()
  it("trace includes cycle counter", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.limm(0, 1.0), gpu_core.halt() })
    local t1 = core:step()
    local t2 = core:step()
    assert.equal(0, t1.cycle)
    assert.equal(1, t2.cycle)
  end)

  it("LIMM trace records register change", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.limm(2, 5.0), gpu_core.halt() })
    local t = core:step()
    assert.near(5.0, t.registers_changed[2], 1e-6)
  end)

  it("STORE trace records memory change", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({
      gpu_core.limm(0, 0.0),
      gpu_core.limm(1, 7.0),
      gpu_core.store(0, 1, 4),
      gpu_core.halt(),
    })
    core:step()  -- limm R0
    core:step()  -- limm R1
    local t = core:step()  -- store
    assert.near(7.0, t.memory_changed[4], 1e-6)
  end)

  it("trace description is a string", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.nop(), gpu_core.halt() })
    local t = core:step()
    assert.is_string(t.description)
  end)

  it("next_pc advances after sequential instructions", function()
    local core = gpu_core.GPUCore.new()
    core:load_program({ gpu_core.nop(), gpu_core.nop(), gpu_core.halt() })
    local t1 = core:step()
    assert.equal(1, t1.next_pc)
    local t2 = core:step()
    assert.equal(2, t2.next_pc)
  end)
end)
