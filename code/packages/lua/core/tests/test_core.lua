-- test_core.lua — Tests for the CPU Core integration package
--
-- The Core integrates pipeline, register file, memory, and an ISA decoder.
-- These tests verify:
--
--   1. CoreConfig presets (simple, performance)
--   2. Core construction with a minimal decoder
--   3. load_program and memory access
--   4. step() and run()
--   5. Register read/write through the core
--   6. Halt detection via the decoder
--   7. Statistics propagation from pipeline to core

package.path = "../src/?.lua;" .. "../src/?/init.lua;" ..
               "../../cpu_pipeline/src/?.lua;" .. "../../cpu_pipeline/src/?/init.lua;" ..
               "../../cpu_simulator/src/?.lua;" .. "../../cpu_simulator/src/?/init.lua;" ..
               package.path

local core_mod   = require("coding_adventures.core")
local Core       = core_mod.Core
local CoreConfig = core_mod.CoreConfig

-- ========================================================================
-- Minimal ISA decoder for testing
-- ========================================================================
--
-- This decoder treats every instruction as a NOP. The halt opcode is 0xFF.
-- All other raw instructions are no-ops.
--
-- Instruction encoding (simplified):
--   Opcode = raw_instruction & 0xFF
--   0x00 = NOP
--   0xFF = HALT
--   0x01 = LOAD_IMM: rd = (raw >> 8) & 0xF, immediate = (raw >> 16) & 0xFF
--   0x02 = ADD:      rd = (raw >> 8) & 0xF, rs1 = (raw >> 12) & 0xF
--   0x03 = STORE:    rs1 = (raw >> 8) & 0xF, addr = (raw >> 16) & 0xFFFF

local NopDecoder = {}

function NopDecoder.decode(raw, token)
    local opcode = raw & 0xFF

    if opcode == 0xFF then
        token.opcode   = "HALT"
        token.is_halt  = true
    elseif opcode == 0x01 then
        token.opcode    = "LOAD_IMM"
        token.rd        = (raw >> 8) & 0xF
        token.immediate = (raw >> 16) & 0xFF
        token.reg_write = true
    elseif opcode == 0x02 then
        token.opcode    = "ADD_IMM"
        token.rd        = (raw >> 8)  & 0xF
        token.rs1       = (raw >> 12) & 0xF
        token.immediate = (raw >> 16) & 0xFF
        token.reg_write = true
        token.source_regs = { token.rs1 }
    else
        token.opcode = "NOP"
    end
    return token
end

function NopDecoder.execute(token, rf)
    if token.opcode == "LOAD_IMM" then
        token.alu_result = token.immediate
        token.write_data = token.immediate
    elseif token.opcode == "ADD_IMM" then
        local v1 = token.rs1 >= 0 and rf:read(token.rs1) or 0
        token.alu_result = (v1 + token.immediate) & 0xFFFFFFFF
        token.write_data = token.alu_result
    end
    return token
end

function NopDecoder.instruction_size()
    return 4
end

-- ========================================================================
-- CoreConfig tests
-- ========================================================================

describe("CoreConfig", function()

    it("simple() creates a 5-stage config", function()
        local cfg = CoreConfig.simple()
        assert.are.equal("Simple", cfg.name)
        assert.are.equal(5,  cfg.pipeline_config:num_stages())
        assert.are.equal(16, cfg.num_registers)
        assert.are.equal(32, cfg.register_width)
    end)

    it("performance() creates a 13-stage config", function()
        local cfg = CoreConfig.performance()
        assert.are.equal("Performance", cfg.name)
        assert.are.equal(13, cfg.pipeline_config:num_stages())
        assert.are.equal(31, cfg.num_registers)
        assert.are.equal(64, cfg.register_width)
    end)

    it("new() with defaults creates a 5-stage config", function()
        local cfg = CoreConfig.new()
        assert.are.equal("Core", cfg.name)
        assert.are.equal(5, cfg.pipeline_config:num_stages())
    end)

end)

-- ========================================================================
-- Core construction tests
-- ========================================================================

describe("Core construction", function()

    it("returns ok=true with a valid config and decoder", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        assert.is_true(result.ok, result.err)
        assert.is_not_nil(result.core)
    end)

    it("starts with cycle=0 and not halted", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        assert.are.equal(0,     core:get_cycle())
        assert.is_false(core:is_halted())
    end)

    it("starts with all registers at 0", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        for i = 0, 15 do
            assert.are.equal(0, core:read_register(i))
        end
    end)

    it("starts with PC=0", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        assert.are.equal(0, result.core:get_pc())
    end)

end)

-- ========================================================================
-- Memory and program loading tests
-- ========================================================================

describe("Core load_program and memory", function()

    it("load_program stores bytes in memory", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        -- Write 0xDEADBEEF in little-endian
        core:load_program({ 0xEF, 0xBE, 0xAD, 0xDE }, 0)
        local word = core:read_memory_word(0)
        assert.are.equal(0xDEADBEEF, word)
    end)

    it("load_program sets the PC to start_address", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0x00, 0x00, 0x00, 0x00 }, 0x100)
        assert.are.equal(0x100, core:get_pc())
    end)

    it("write_memory_word / read_memory_word round-trip", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:write_memory_word(0x20, 0xABCDEF01)
        assert.are.equal(0xABCDEF01, core:read_memory_word(0x20))
    end)

end)

-- ========================================================================
-- step() and run() tests
-- ========================================================================

describe("Core step and run", function()

    it("step() increments the cycle counter", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)
        core:step()
        assert.are.equal(1, core:get_cycle())
        core:step()
        assert.are.equal(2, core:get_cycle())
    end)

    it("step() returns a snapshot", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)
        local snap = core:step()
        assert.are.equal(1, snap.cycle)
    end)

    it("run() executes multiple cycles", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)  -- NOP program
        core:run(10)
        assert.are.equal(10, core:get_cycle())
    end)

    it("run() returns CoreStats", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)
        local stats = core:run(10)
        assert.are.equal(10, stats.total_cycles)
        assert.is_true(stats:ipc() > 0)
    end)

end)

-- ========================================================================
-- Register access tests
-- ========================================================================

describe("Core register access", function()

    it("write_register / read_register round-trip", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:write_register(3, 999)
        assert.are.equal(999, core:read_register(3))
    end)

    it("LOAD_IMM instruction writes a register after reaching WB", function()
        -- Encode: opcode=0x01, rd=2, immediate=77
        -- raw = (77 << 16) | (2 << 8) | 0x01 = 0x004D0201
        local raw = (77 << 16) | (2 << 8) | 0x01
        -- Little-endian bytes of raw
        local b0 = raw & 0xFF
        local b1 = (raw >> 8) & 0xFF
        local b2 = (raw >> 16) & 0xFF
        local b3 = (raw >> 24) & 0xFF

        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ b0, b1, b2, b3, 0, 0, 0, 0 }, 0)

        -- Need 5 cycles to propagate through 5-stage pipeline
        core:run(6)
        -- R2 should now be 77
        assert.are.equal(77, core:read_register(2))
    end)

end)

-- ========================================================================
-- Halt detection tests
-- ========================================================================

describe("Core halt detection", function()

    it("halts when HALT instruction reaches WB", function()
        -- 0xFF = HALT opcode
        local halt_bytes = { 0xFF, 0x00, 0x00, 0x00 }
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program(halt_bytes, 0)

        -- Run enough cycles for HALT to propagate through 5 stages
        for i = 1, 10 do
            core:step()
            if core:is_halted() then break end
        end

        assert.is_true(core:is_halted())
    end)

    it("run() stops at max_cycles even without halt", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)  -- infinite NOPs
        core:run(5)
        assert.are.equal(5, core:get_cycle())
        assert.is_false(core:is_halted())
    end)

end)

-- ========================================================================
-- Statistics tests
-- ========================================================================

describe("Core statistics", function()

    it("get_stats() returns CoreStats with correct total_cycles", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)
        core:run(20)
        local stats = core:get_stats()
        assert.are.equal(20, stats.total_cycles)
    end)

    it("IPC approaches 1.0 for independent instructions", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)
        core:run(50)
        local stats = core:get_stats()
        assert.is_true(stats:ipc() > 0.8,
            "IPC should be > 0.8 for NOPs, got " .. stats:ipc())
    end)

    it("to_string() includes useful info", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:run(5)
        local s = core:get_stats():to_string()
        assert.truthy(s:find("IPC"))
        assert.truthy(s:find("cycles"))
    end)

end)

-- ========================================================================
-- Trace tests
-- ========================================================================

describe("Core trace", function()

    it("get_trace() returns snapshots for each cycle", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)
        for i = 1, 3 do core:step() end
        local trace = core:get_trace()
        assert.are.equal(3, #trace)
    end)

    it("trace snapshots have sequential cycle numbers", function()
        local result = Core.new(CoreConfig.simple(), NopDecoder)
        local core   = result.core
        core:load_program({ 0, 0, 0, 0 }, 0)
        for i = 1, 4 do core:step() end
        local trace = core:get_trace()
        for i, snap in ipairs(trace) do
            assert.are.equal(i, snap.cycle)
        end
    end)

end)
