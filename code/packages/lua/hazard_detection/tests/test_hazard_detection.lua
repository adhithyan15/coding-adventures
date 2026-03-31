-- test_hazard_detection.lua — Tests for pipeline hazard detection
--
-- These tests verify that the hazard detectors correctly identify and
-- classify hazards in a pipelined CPU:
--
--   1. DataHazardDetector  — RAW hazards, forwarding, load-use stalls
--   2. ControlHazardDetector — branch mispredictions, flushes
--   3. StructuralHazardDetector — resource conflicts

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local hazard_mod             = require("coding_adventures.hazard_detection")
local PipelineSlot           = hazard_mod.PipelineSlot
local HazardResult           = hazard_mod.HazardResult
local DataHazardDetector     = hazard_mod.DataHazardDetector
local ControlHazardDetector  = hazard_mod.ControlHazardDetector
local StructuralHazardDetector = hazard_mod.StructuralHazardDetector

-- ========================================================================
-- PipelineSlot tests
-- ========================================================================

describe("PipelineSlot", function()

    it("new() creates a valid slot with defaults", function()
        local s = PipelineSlot.new()
        assert.is_true(s.valid)
        assert.are.equal(0,  s.pc)
        assert.are.equal(-1, s.dest_reg)
        assert.is_false(s.mem_read)
        assert.is_false(s.is_branch)
    end)

    it("empty() creates an invalid slot", function()
        local s = PipelineSlot.empty()
        assert.is_false(s.valid)
    end)

    it("can be constructed with custom fields", function()
        local s = PipelineSlot.new({
            valid       = true,
            pc          = 0x100,
            dest_reg    = 3,
            dest_value  = 42,
            source_regs = { 1, 2 },
            mem_read    = true,
        })
        assert.are.equal(0x100, s.pc)
        assert.are.equal(3,     s.dest_reg)
        assert.are.equal(42,    s.dest_value)
        assert.are.same({ 1, 2 }, s.source_regs)
        assert.is_true(s.mem_read)
    end)

end)

-- ========================================================================
-- HazardResult tests
-- ========================================================================

describe("HazardResult", function()

    it("default action is 'none'", function()
        local r = HazardResult.new()
        assert.are.equal("none", r.action)
        assert.are.equal(0,      r.stall_cycles)
        assert.are.equal(0,      r.forwarded_value)
        assert.are.equal("",     r.forwarded_from)
    end)

    it("can be constructed with all fields", function()
        local r = HazardResult.new({
            action          = "forward_ex",
            forwarded_value = 99,
            forwarded_from  = "EX",
            reason          = "test",
        })
        assert.are.equal("forward_ex", r.action)
        assert.are.equal(99,           r.forwarded_value)
        assert.are.equal("EX",         r.forwarded_from)
    end)

end)

-- ========================================================================
-- DataHazardDetector tests
-- ========================================================================

describe("DataHazardDetector", function()

    local det

    before_each(function()
        det = DataHazardDetector.new()
    end)

    -- Helper to make a slot with a destination register
    local function make_producer(dest_reg, dest_value, mem_read)
        return PipelineSlot.new({
            valid      = true,
            pc         = 0x10,
            dest_reg   = dest_reg,
            dest_value = dest_value,
            mem_read   = mem_read or false,
        })
    end

    -- Helper to make a slot with source registers
    local function make_consumer(src_regs)
        return PipelineSlot.new({
            valid       = true,
            pc          = 0x14,
            source_regs = src_regs,
        })
    end

    it("returns 'none' when ID stage is a bubble", function()
        local id  = PipelineSlot.empty()
        local ex  = PipelineSlot.empty()
        local mem = PipelineSlot.empty()
        local r   = det:detect(id, ex, mem)
        assert.are.equal("none", r.action)
    end)

    it("returns 'none' when instruction has no source registers", function()
        local id  = PipelineSlot.new({ valid = true, source_regs = {} })
        local ex  = make_producer(1, 100, false)
        local mem = PipelineSlot.empty()
        local r   = det:detect(id, ex, mem)
        assert.are.equal("none", r.action)
    end)

    it("returns 'none' when there are no pending writes", function()
        local id  = make_consumer({ 1, 2 })
        local ex  = make_producer(5, 0)   -- writes R5, not R1 or R2
        local mem = make_producer(6, 0)   -- writes R6
        local r   = det:detect(id, ex, mem)
        assert.are.equal("none", r.action)
    end)

    it("returns 'forward_ex' for EX-to-EX RAW hazard", function()
        -- ADD R1, R2, R3 is in EX (writing R1)
        -- SUB R4, R1, R5 is in ID (reading R1) → should forward from EX
        local id  = make_consumer({ 1, 5 })
        local ex  = make_producer(1, 42, false)  -- writes R1, value=42
        local mem = PipelineSlot.empty()
        local r   = det:detect(id, ex, mem)
        assert.are.equal("forward_ex", r.action)
        assert.are.equal(42,           r.forwarded_value)
        assert.are.equal("EX",         r.forwarded_from)
    end)

    it("returns 'forward_mem' for MEM-to-EX RAW hazard", function()
        -- R3 is being written in MEM (2 cycles before ID)
        local id  = make_consumer({ 3 })
        local ex  = make_producer(7, 0)   -- writes R7, not R3
        local mem = make_producer(3, 77)  -- writes R3, value=77
        local r   = det:detect(id, ex, mem)
        assert.are.equal("forward_mem", r.action)
        assert.are.equal(77,            r.forwarded_value)
        assert.are.equal("MEM",         r.forwarded_from)
    end)

    it("returns 'stall' for load-use hazard (EX is a LOAD)", function()
        -- LDR R1, [R2] is in EX (mem_read=true, writing R1)
        -- ADD R3, R1, R4 is in ID (reading R1)
        -- R1 won't be available until after MEM — can't forward, must stall
        local id  = make_consumer({ 1, 4 })
        local ex  = make_producer(1, 0, true)  -- LDR: mem_read=true
        local mem = PipelineSlot.empty()
        local r   = det:detect(id, ex, mem)
        assert.are.equal("stall", r.action)
        assert.are.equal(1,       r.stall_cycles)
    end)

    it("stall takes priority over forward_ex (both sources hazardous)", function()
        -- R1 in EX (load-use) and R2 in MEM (normal forward)
        -- load-use stall has higher priority
        local id  = make_consumer({ 1, 2 })
        local ex  = make_producer(1, 0, true)   -- load-use on R1
        local mem = make_producer(2, 55)         -- forward on R2
        local r   = det:detect(id, ex, mem)
        assert.are.equal("stall", r.action)
    end)

    it("forward_ex takes priority over forward_mem", function()
        -- R3 is in both EX (non-load) and MEM
        -- EX wins because it's closer (more recent)
        local id  = make_consumer({ 3 })
        local ex  = make_producer(3, 10, false)
        local mem = make_producer(3, 20)
        local r   = det:detect(id, ex, mem)
        assert.are.equal("forward_ex", r.action)
        assert.are.equal(10, r.forwarded_value)
    end)

    it("reason string is non-empty for all actions", function()
        local id  = make_consumer({ 1 })
        local ex  = make_producer(1, 5, false)
        local r   = det:detect(id, ex, PipelineSlot.empty())
        assert.truthy(#r.reason > 0)
    end)

end)

-- ========================================================================
-- ControlHazardDetector tests
-- ========================================================================

describe("ControlHazardDetector", function()

    local det

    before_each(function()
        det = ControlHazardDetector.new()
    end)

    it("returns 'none' when EX stage is empty", function()
        local r = det:detect(PipelineSlot.empty())
        assert.are.equal("none", r.action)
    end)

    it("returns 'none' when EX stage is not a branch", function()
        local ex = PipelineSlot.new({
            valid     = true,
            pc        = 0x20,
            is_branch = false,
        })
        local r = det:detect(ex)
        assert.are.equal("none", r.action)
    end)

    it("returns 'none' when branch is correctly predicted as not-taken", function()
        local ex = PipelineSlot.new({
            valid                  = true,
            pc                     = 0x20,
            is_branch              = true,
            branch_taken           = false,
            branch_predicted_taken = false,
            branch_target          = 0x80,
        })
        local r = det:detect(ex)
        assert.are.equal("none", r.action)
    end)

    it("returns 'none' when branch is correctly predicted as taken", function()
        local ex = PipelineSlot.new({
            valid                  = true,
            pc                     = 0x20,
            is_branch              = true,
            branch_taken           = true,
            branch_predicted_taken = true,
            branch_target          = 0x80,
        })
        local r = det:detect(ex)
        assert.are.equal("none", r.action)
    end)

    it("returns 'flush' when predicted not-taken but actually taken", function()
        local ex = PipelineSlot.new({
            valid                  = true,
            pc                     = 0x20,
            is_branch              = true,
            branch_taken           = true,
            branch_predicted_taken = false,  -- predicted not-taken
            branch_target          = 0x80,   -- actual target
        })
        local r = det:detect(ex)
        assert.are.equal("flush",  r.action)
        assert.are.equal(0x80,     r.flush_target)  -- redirect to branch target
    end)

    it("returns 'flush' when predicted taken but actually not taken", function()
        local ex = PipelineSlot.new({
            valid                  = true,
            pc                     = 0x20,
            is_branch              = true,
            branch_taken           = false,
            branch_predicted_taken = true,   -- predicted taken
            branch_target          = 0x80,
        })
        local r = det:detect(ex)
        assert.are.equal("flush", r.action)
        -- Redirect to PC + 4 (sequential next)
        assert.are.equal(0x24, r.flush_target)
    end)

    it("reason string mentions misprediction details", function()
        local ex = PipelineSlot.new({
            valid                  = true,
            pc                     = 0x20,
            is_branch              = true,
            branch_taken           = true,
            branch_predicted_taken = false,
            branch_target          = 0x80,
        })
        local r = det:detect(ex)
        assert.truthy(r.reason:find("mispredicted"))
    end)

end)

-- ========================================================================
-- StructuralHazardDetector tests
-- ========================================================================

describe("StructuralHazardDetector", function()

    local det

    before_each(function()
        det = StructuralHazardDetector.new()
    end)

    it("returns 'none' with split cache (default)", function()
        local mem = PipelineSlot.new({
            valid    = true,
            pc       = 0x10,
            dest_reg = 1,
            mem_read = true,
        })
        local wb = PipelineSlot.empty()
        -- With split cache (default), no structural hazard
        local r = det:detect(mem, wb, true)
        assert.are.equal("none", r.action)
    end)

    it("returns 'stall' for unified cache conflict", function()
        local mem = PipelineSlot.new({
            valid    = true,
            pc       = 0x10,
            dest_reg = 1,
            mem_read = true,
        })
        local wb = PipelineSlot.empty()
        -- Unified cache: IF and MEM both need memory
        local r = det:detect(mem, wb, false)  -- false = unified cache
        assert.are.equal("stall", r.action)
    end)

    it("returns 'none' when neither MEM nor WB write to the same register", function()
        local mem = PipelineSlot.new({ valid = true, dest_reg = 1 })
        local wb  = PipelineSlot.new({ valid = true, dest_reg = 2 })
        local r   = det:detect(mem, wb, true)
        assert.are.equal("none", r.action)
    end)

    it("returns 'stall' when MEM and WB both write to the same register", function()
        local mem = PipelineSlot.new({ valid = true, dest_reg = 3 })
        local wb  = PipelineSlot.new({ valid = true, dest_reg = 3 })
        local r   = det:detect(mem, wb, true)
        assert.are.equal("stall", r.action)
    end)

    it("returns 'none' when stages are empty", function()
        local r = det:detect(PipelineSlot.empty(), PipelineSlot.empty(), true)
        assert.are.equal("none", r.action)
    end)

end)
