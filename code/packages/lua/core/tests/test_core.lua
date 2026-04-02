-- Tests for coding_adventures.core

local core_mod = require("coding_adventures.core")
local Core       = core_mod.Core
local CoreConfig = core_mod.CoreConfig
local CoreStats  = core_mod.CoreStats

-- ---------------------------------------------------------------------------
-- Minimal ISA decoder for tests
-- ---------------------------------------------------------------------------
-- Instruction encoding (1 byte at address 0, rest ignored):
--   0x00 = NOP    — no operation
--   0xFF = HALT   — stop execution
--   0x01 = LOAD_IMM rd=2, imm=77   — R2 ← 77
--   0x02 = ADD_IMM  rd=3, imm=10   — R3 ← R3 + 10

local NopDecoder = {}
NopDecoder.__index = NopDecoder

function NopDecoder.new()
    return setmetatable({}, NopDecoder)
end

function NopDecoder:decode(raw, token)
    local opcode_byte = raw & 0xFF
    if opcode_byte == 0xFF then
        token.opcode    = "HALT"
        token.is_halt   = true
    elseif opcode_byte == 0x01 then
        token.opcode    = "LOAD_IMM"
        token.rd        = 2
        token.immediate = 77
        token.reg_write = true
    elseif opcode_byte == 0x02 then
        token.opcode    = "ADD_IMM"
        token.rd        = 3
        token.rs1       = 3
        token.immediate = 10
        token.reg_write = true
    else
        token.opcode = "NOP"
    end
    return token
end

function NopDecoder:execute(token, reg_file)
    if token.opcode == "LOAD_IMM" then
        token.alu_result = token.immediate
        token.write_data = token.immediate
    elseif token.opcode == "ADD_IMM" then
        local src = token.rs1 >= 0 and reg_file:read(token.rs1) or 0
        token.alu_result = src + token.immediate
        token.write_data = token.alu_result
    end
    return token
end

function NopDecoder:instruction_size()
    return 4
end

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

local function make_core(opts)
    opts = opts or {}
    local config  = opts.config or CoreConfig.simple()
    local decoder = opts.decoder or NopDecoder.new()
    local result = Core.new(config, decoder)
    assert(result.ok, "Core.new failed: " .. tostring(result.err))
    return result.core
end

-- Build a memory byte list with a specific opcode at address 0 followed by HALT
-- and NOPs filling the rest.
local function program_with(first_opcode, halt_offset)
    halt_offset = halt_offset or 4
    local mem = {}
    for i = 1, 256 do mem[i] = 0x00 end  -- NOP everywhere
    mem[1] = first_opcode & 0xFF           -- instruction at address 0
    mem[halt_offset + 1] = 0xFF            -- HALT
    return mem
end

-- ---------------------------------------------------------------------------
-- CoreConfig presets
-- ---------------------------------------------------------------------------

describe("CoreConfig presets", function()
    it("simple() returns valid config", function()
        local cfg = CoreConfig.simple()
        assert.equals("Simple", cfg.name)
        assert.equals(16, cfg.num_registers)
        assert.equals(32, cfg.register_width)
        assert.equals(65536, cfg.memory_size)
    end)

    it("performance() returns 13-stage, 31-register config", function()
        local cfg = CoreConfig.performance()
        assert.equals("Performance", cfg.name)
        assert.equals(31, cfg.num_registers)
    end)
end)

-- ---------------------------------------------------------------------------
-- Core construction
-- ---------------------------------------------------------------------------

describe("Core.new", function()
    it("succeeds with valid config and decoder", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder.new())
        assert.is_true(result.ok)
        assert.not_nil(result.core)
    end)

    it("returns error for nil config", function()
        local result = Core.new(nil, NopDecoder.new())
        assert.is_false(result.ok)
        assert.truthy(result.err)
    end)

    it("returns error for nil decoder", function()
        local result = Core.new(CoreConfig.simple(), nil)
        assert.is_false(result.ok)
        assert.truthy(result.err)
    end)

    it("starts not halted", function()
        local c = make_core()
        assert.is_false(c:is_halted())
    end)

    it("starts at cycle 0", function()
        local c = make_core()
        assert.equals(0, c:get_cycle())
    end)
end)

-- ---------------------------------------------------------------------------
-- load_program / step / run
-- ---------------------------------------------------------------------------

describe("Core.load_program", function()
    it("loads bytes into memory correctly", function()
        local c = make_core()
        c:load_program({0xFF, 0, 0, 0}, 0)
        -- The HALT byte should be at address 0
        local word = c:read_memory_word(0)
        assert.equals(0xFF, word & 0xFF)
    end)
end)

describe("Core.step", function()
    it("advances cycle counter", function()
        local c = make_core()
        c:load_program({0x00, 0, 0, 0, 0xFF, 0, 0, 0}, 0)
        c:step()
        assert.equals(1, c:get_cycle())
    end)

    it("returns a Snapshot with cycle number", function()
        local c = make_core()
        c:load_program({0x00, 0, 0, 0}, 0)
        local snap = c:step()
        assert.equals(1, snap.cycle)
    end)

    it("does not advance cycle when already halted", function()
        local c = make_core()
        c:load_program({0xFF, 0, 0, 0}, 0)
        for _ = 1, 10 do c:step() end
        local after_halt = c:get_cycle()
        c:step()
        assert.equals(after_halt, c:get_cycle())
    end)
end)

describe("Core.run — NOP then HALT", function()
    it("halts after HALT instruction completes pipeline", function()
        local c = make_core()
        -- HALT at address 0: takes 5 cycles (5-stage pipeline) + a few for drain
        c:load_program({0xFF, 0, 0, 0}, 0)
        c:run(20)
        assert.is_true(c:is_halted())
    end)

    it("multiple NOP then HALT completes", function()
        local c = make_core()
        -- NOP at 0, NOP at 4, HALT at 8
        local prog = {0x00,0,0,0, 0x00,0,0,0, 0xFF,0,0,0}
        c:load_program(prog, 0)
        c:run(30)
        assert.is_true(c:is_halted())
    end)
end)

-- ---------------------------------------------------------------------------
-- Register access
-- ---------------------------------------------------------------------------

describe("Core register access", function()
    it("read_register returns 0 initially", function()
        local c = make_core()
        assert.equals(0, c:read_register(0))
        assert.equals(0, c:read_register(7))
    end)

    it("write_register / read_register roundtrip", function()
        local c = make_core()
        c:write_register(5, 0xBEEF)
        assert.equals(0xBEEF, c:read_register(5))
    end)
end)

-- ---------------------------------------------------------------------------
-- LOAD_IMM execution test
-- ---------------------------------------------------------------------------
-- This end-to-end test verifies that a LOAD_IMM instruction (opcode 0x01)
-- actually writes the immediate value (77) to R2.
--
-- Program:
--   0x00:  LOAD_IMM → R2 = 77
--   0x04:  HALT
--
-- After the pipeline drains, R2 should contain 77.

describe("Core — LOAD_IMM execution", function()
    it("LOAD_IMM writes immediate value to destination register", function()
        local c = make_core()
        -- 0x01 = LOAD_IMM (R2 ← 77), 0xFF = HALT
        local prog = {0x01, 0, 0, 0,   0xFF, 0, 0, 0}
        c:load_program(prog, 0)
        c:run(20)
        assert.is_true(c:is_halted())
        assert.equals(77, c:read_register(2))
    end)
end)

-- ---------------------------------------------------------------------------
-- Stats
-- ---------------------------------------------------------------------------

describe("CoreStats", function()
    it("ipc is 0 when no cycles", function()
        local s = CoreStats.new(0, 0, nil)
        assert.equals(0.0, s:ipc())
    end)

    it("cpi is 0 when no instructions", function()
        local s = CoreStats.new(0, 5, nil)
        assert.equals(0.0, s:cpi())
    end)

    it("ipc = instructions / cycles", function()
        local s = CoreStats.new(10, 20, nil)
        assert.equals(0.5, s:ipc())
    end)

    it("get_stats() after run returns meaningful data", function()
        local c = make_core()
        c:load_program({0xFF, 0, 0, 0}, 0)
        c:run(20)
        local stats = c:get_stats()
        assert.truthy(stats.total_cycles > 0)
        assert.truthy(stats.instructions_completed > 0)
    end)
end)

-- ---------------------------------------------------------------------------
-- Trace
-- ---------------------------------------------------------------------------

describe("Core.get_trace", function()
    it("returns snapshots equal to step count", function()
        local c = make_core()
        c:load_program({0x00,0,0,0, 0xFF,0,0,0}, 0)
        c:step(); c:step(); c:step()
        local trace = c:get_trace()
        assert.equals(3, #trace)
    end)

    it("snapshots are in chronological order", function()
        local c = make_core()
        c:load_program({0x00,0,0,0}, 0)
        c:step(); c:step()
        local trace = c:get_trace()
        assert.equals(1, trace[1].cycle)
        assert.equals(2, trace[2].cycle)
    end)
end)

-- ---------------------------------------------------------------------------
-- Performance config
-- ---------------------------------------------------------------------------

describe("Core with performance config", function()
    it("runs 13-stage pipeline without error", function()
        local c = make_core({config = CoreConfig.performance()})
        c:load_program({0xFF, 0, 0, 0}, 0)
        c:run(40)
        assert.is_true(c:is_halted())
    end)
end)
